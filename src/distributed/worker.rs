// src/distributed/worker.rs
//
// Worker node: claims chunks from Redis, sieves for smooth relations,
// pushes results back to Redis stream.

use redis::Commands;
use log::{info, warn, debug};
use num::BigInt;
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::{GnfsConfig, DistributedConfig, BufferConfig};
use crate::core::cancellation_token::CancellationToken;
use crate::core::gnfs::GNFS;
use crate::core::gnfs_integer::GnfsInteger;
use crate::core::serialization::types::{SerializableGNFS, SerializableRelation};
use crate::distributed::messages::{Chunk, FactorizationStatus, RedisKeys};
use crate::distributed::redis_client;
use crate::relation_sieve::relation::Relation;
use crate::backends::*;

/// Run a worker: connect to Redis, load params, claim chunks, sieve, push relations
pub fn run(
    n: &BigInt,
    config: &GnfsConfig,
    cancel_token: &CancellationToken,
) -> Result<(), String> {
    let dist_config = &config.distributed;
    let keys = RedisKeys::new(&n.to_string());

    // Generate worker ID
    let worker_id = if dist_config.worker_id == "auto" {
        let hostname = std::env::var("HOSTNAME")
            .unwrap_or_else(|_| "unknown".to_string());
        format!("{}-{}", hostname, std::process::id())
    } else {
        dist_config.worker_id.clone()
    };

    info!("Worker {} starting...", worker_id);

    // Connect to Redis
    let client = redis_client::create_client(&dist_config.redis_url)
        .map_err(|e| format!("Failed to create Redis client: {}", e))?;
    let mut conn = redis_client::get_connection_with_retry(&client)
        .map_err(|e| format!("Failed to connect to Redis: {}", e))?;

    info!("Worker {} connected to Redis at {}", worker_id, dist_config.redis_url);

    // Wait for params to be available
    let params_json: String = loop {
        if cancel_token.is_cancellation_requested() {
            return Ok(());
        }

        let result: Option<String> = conn.get(keys.params())
            .map_err(|e| format!("Redis GET error: {}", e))?;

        match result {
            Some(json) => break json,
            None => {
                info!("Waiting for coordinator to publish parameters...");
                thread::sleep(Duration::from_secs(2));
            }
        }
    };

    // Deserialize params to determine backend
    let serializable: SerializableGNFS = serde_json::from_str(&params_json)
        .map_err(|e| format!("Failed to deserialize params: {}", e))?;
    let backend_name = serializable.backend_name.clone();

    info!("Loaded params: N={}, backend={}", serializable.n, backend_name);

    // Start heartbeat in background
    let heartbeat_running = Arc::new(AtomicBool::new(true));
    let heartbeat_handle = start_heartbeat(
        &dist_config.redis_url,
        &keys,
        &worker_id,
        dist_config.heartbeat_interval_secs,
        heartbeat_running.clone(),
    );

    // Dispatch to the correct backend and run the sieve loop
    let result = match backend_name.as_str() {
        "Native64Signed" => {
            let gnfs = reconstruct_gnfs::<Native64Signed>(serializable, &config.buffer)?;
            sieve_loop::<Native64Signed>(&mut conn, &keys, &worker_id, &gnfs, dist_config, cancel_token)
        }
        "Native128Signed" => {
            let gnfs = reconstruct_gnfs::<Native128Signed>(serializable, &config.buffer)?;
            sieve_loop::<Native128Signed>(&mut conn, &keys, &worker_id, &gnfs, dist_config, cancel_token)
        }
        "Fixed256" => {
            let gnfs = reconstruct_gnfs::<Fixed256>(serializable, &config.buffer)?;
            sieve_loop::<Fixed256>(&mut conn, &keys, &worker_id, &gnfs, dist_config, cancel_token)
        }
        "Fixed512" => {
            let gnfs = reconstruct_gnfs::<Fixed512>(serializable, &config.buffer)?;
            sieve_loop::<Fixed512>(&mut conn, &keys, &worker_id, &gnfs, dist_config, cancel_token)
        }
        "Arbitrary" | "BigInt" => {
            let gnfs = reconstruct_gnfs::<BigIntBackend>(serializable, &config.buffer)?;
            sieve_loop::<BigIntBackend>(&mut conn, &keys, &worker_id, &gnfs, dist_config, cancel_token)
        }
        other => {
            return Err(format!("Unknown backend: {}", other));
        }
    };

    // Stop heartbeat
    heartbeat_running.store(false, Ordering::Relaxed);
    if let Some(handle) = heartbeat_handle {
        let _ = handle.join();
    }

    // Remove heartbeat entry
    let _: Result<(), _> = conn.hdel(keys.heartbeat(), &worker_id);

    result
}

/// Reconstruct a GNFS<T> from serialized params (read-only, for sieving)
fn reconstruct_gnfs<T: GnfsInteger>(
    serializable: SerializableGNFS,
    buffer_config: &BufferConfig,
) -> Result<GNFS<T>, String> {
    // Validate backend
    if serializable.backend_name != T::backend_name() {
        return Err(format!(
            "Backend mismatch: params say {}, but using {}",
            serializable.backend_name,
            T::backend_name()
        ));
    }

    let gnfs = GNFS::<T>::from_checkpoint(serializable, buffer_config.clone());
    info!("Reconstructed GNFS instance with {} backend", T::backend_name());
    info!("  Polynomial degree: {}", gnfs.polynomial_degree);
    info!("  Rational factor pairs: {}", gnfs.rational_factor_pair_collection.len());
    info!("  Algebraic factor pairs: {}", gnfs.algebraic_factor_pair_collection.len());
    info!("  Quadratic factor pairs: {}", gnfs.quadratic_factor_pair_collection.len());

    Ok(gnfs)
}

/// Main sieve loop: claim chunks, sieve, push results
fn sieve_loop<T: GnfsInteger>(
    conn: &mut redis::Connection,
    keys: &RedisKeys,
    worker_id: &str,
    gnfs: &GNFS<T>,
    dist_config: &DistributedConfig,
    cancel_token: &CancellationToken,
) -> Result<(), String> {
    let claim_script = redis::Script::new(redis_client::CLAIM_CHUNK_SCRIPT);
    let mut total_relations = 0usize;
    let mut chunks_completed = 0usize;

    info!("Worker {} entering sieve loop...", worker_id);

    loop {
        if cancel_token.is_cancellation_requested() {
            info!("Worker {} shutting down (cancelled)", worker_id);
            break;
        }

        // Check job status
        let status: Option<String> = conn.get(keys.status())
            .map_err(|e| format!("Redis GET status error: {}", e))?;

        if let Some(s) = &status {
            if let Some(fs) = FactorizationStatus::parse(s) {
                if fs != FactorizationStatus::Sieving {
                    info!("Job status is '{}', stopping sieve loop", s);
                    break;
                }
            }
        }

        // Claim a chunk atomically
        let now = chrono::Utc::now().timestamp();
        let chunk_json: Option<String> = claim_script
            .key(keys.chunks())
            .key(keys.claimed())
            .arg(worker_id)
            .arg(now)
            .invoke(conn)
            .map_err(|e| format!("Claim script error: {}", e))?;

        let chunk = match chunk_json {
            Some(json) => {
                Chunk::from_redis_member(&json)?
            }
            None => {
                // No chunks available, wait and retry
                debug!("No chunks available, waiting...");
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        info!("Worker {} claimed chunk: B=[{},{}), A=[{},{})",
              worker_id, chunk.b_start, chunk.b_end, chunk.a_start, chunk.a_start + chunk.a_range);

        // Sieve the chunk
        let relations = sieve_chunk(gnfs, &chunk);
        let found = relations.len();

        // Push relations to Redis stream in batches
        if !relations.is_empty() {
            push_relations(conn, keys, &relations, dist_config.push_batch_size)?;

            // Increment global smooth count
            let _: () = conn.incr(keys.smooth_count(), found as i64)
                .map_err(|e| format!("Redis INCRBY error: {}", e))?;
        }

        // Mark chunk as completed
        let _: () = conn.hdel(keys.claimed(), chunk.to_redis_member())
            .map_err(|e| format!("Redis HDEL error: {}", e))?;
        let _: () = conn.sadd(keys.completed(), &chunk.id)
            .map_err(|e| format!("Redis SADD error: {}", e))?;

        total_relations += found;
        chunks_completed += 1;

        info!("Worker {} completed chunk {} ({} smooth relations, total: {})",
              worker_id, chunk.id, found, total_relations);
    }

    info!("Worker {} finished: {} chunks, {} total relations",
          worker_id, chunks_completed, total_relations);

    Ok(())
}

/// Sieve a single chunk for smooth relations
fn sieve_chunk<T: GnfsInteger>(gnfs: &GNFS<T>, chunk: &Chunk) -> Vec<SerializableRelation> {
    use crate::integer_math::gcd::GCD;
    use rayon::prelude::*;

    let mut all_smooth = Vec::new();

    // Iterate B values in the chunk (odd only for coprimality)
    let mut b = chunk.b_start;
    if b % 2 == 0 { b += 1; }

    while b < chunk.b_end {
        let current_b = BigInt::from(b);

        // Generate A values and test for smoothness
        let a_values: Vec<BigInt> = (chunk.a_start..chunk.a_start + chunk.a_range)
            .filter(|a| *a != 0) // Skip 0 (gcd(0, b) = b, never coprime)
            .flat_map(|a| vec![BigInt::from(a), BigInt::from(-a)])
            .filter(|a| GCD::are_coprime_pair(a, &current_b))
            .collect();

        let smooth_for_b: Vec<Relation<T>> = a_values
            .par_iter()
            .filter_map(|a| {
                let mut rel = Relation::new(gnfs, a, &current_b);
                rel.sieve(gnfs);
                if rel.is_smooth() {
                    Some(rel)
                } else {
                    None
                }
            })
            .collect();

        // Convert to serializable immediately (we don't need the typed Relation anymore)
        for rel in &smooth_for_b {
            all_smooth.push(SerializableRelation::from(rel));
        }

        b += 2; // Next odd B
    }

    all_smooth
}

/// Push serializable relations to Redis stream
fn push_relations(
    conn: &mut redis::Connection,
    keys: &RedisKeys,
    relations: &[SerializableRelation],
    batch_size: usize,
) -> Result<(), String> {
    for batch in relations.chunks(batch_size) {
        // Use pipeline for efficiency
        let mut pipe = redis::pipe();
        for rel in batch {
            let json = serde_json::to_string(rel)
                .map_err(|e| format!("Failed to serialize relation: {}", e))?;
            pipe.cmd("XADD")
                .arg(keys.relations())
                .arg("*")
                .arg("data")
                .arg(&json);
        }
        pipe.query::<Vec<String>>(conn)
            .map_err(|e| format!("Redis pipeline XADD error: {}", e))?;
    }

    Ok(())
}

/// Start a background heartbeat thread
fn start_heartbeat(
    redis_url: &str,
    keys: &RedisKeys,
    worker_id: &str,
    interval_secs: u64,
    running: Arc<AtomicBool>,
) -> Option<thread::JoinHandle<()>> {
    let redis_url = redis_url.to_string();
    let heartbeat_key = keys.heartbeat();
    let worker_id = worker_id.to_string();

    let handle = thread::spawn(move || {
        let client = match redis_client::create_client(&redis_url) {
            Ok(c) => c,
            Err(e) => {
                warn!("Heartbeat thread failed to create client: {}", e);
                return;
            }
        };
        let mut conn = match client.get_connection() {
            Ok(c) => c,
            Err(e) => {
                warn!("Heartbeat thread failed to connect: {}", e);
                return;
            }
        };

        while running.load(Ordering::Relaxed) {
            let now = chrono::Utc::now().timestamp();
            let _: Result<(), _> = conn.hset(&heartbeat_key, &worker_id, now);
            thread::sleep(Duration::from_secs(interval_secs));
        }
    });

    Some(handle)
}
