// src/distributed/coordinator.rs
//
// Coordinator node: computes GNFS params, partitions search space,
// monitors worker progress, collects relations for matrix solve.

use redis::{Commands, FromRedisValue, Value};
use log::{info, warn};
use num::BigInt;
use std::thread;
use std::time::Duration;

use crate::config::{GnfsConfig, DistributedConfig};
use crate::core::cancellation_token::CancellationToken;
use crate::core::gnfs_wrapper::GNFSWrapper;
use crate::core::serialization::types::{SerializableGNFS, SerializableRelation};
use crate::distributed::messages::{Chunk, FactorizationStatus, RedisKeys};
use crate::distributed::redis_client;

/// Run the coordinator: initialize params, create chunks, monitor until target reached
pub fn run(
    n: &BigInt,
    config: &GnfsConfig,
    cancel_token: &CancellationToken,
) -> Result<(), String> {
    let dist_config = &config.distributed;
    let keys = RedisKeys::new(&n.to_string());

    // Connect to Redis
    let client = redis_client::create_client(&dist_config.redis_url)
        .map_err(|e| format!("Failed to create Redis client: {}", e))?;
    let mut conn = redis_client::get_connection_with_retry(&client)
        .map_err(|e| format!("Failed to connect to Redis: {}", e))?;

    info!("Coordinator connected to Redis at {}", dist_config.redis_url);

    // Check if there's already a job running for this N
    let existing_status: Option<String> = conn.get(keys.status())
        .map_err(|e| format!("Redis GET error: {}", e))?;

    if let Some(status_str) = &existing_status {
        if let Some(status) = FactorizationStatus::parse(status_str) {
            if status == FactorizationStatus::Sieving {
                info!("Existing sieving job found for N={}. Resuming coordination...", n);
                return monitor_loop(&mut conn, &keys, dist_config, cancel_token);
            } else if status == FactorizationStatus::Done {
                info!("Factorization already complete for N={}", n);
                return Ok(());
            }
        }
    }

    // Create fresh GNFS instance to compute polynomial + factor bases
    info!("Computing polynomial and factor bases...");
    let gnfs = create_gnfs_for_distributed(n, config);

    // Serialize and publish params
    let params_json = serialize_params(&gnfs);
    let _: () = conn.set(keys.params(), &params_json)
        .map_err(|e| format!("Redis SET params error: {}", e))?;
    info!("Published GNFS parameters to Redis");

    // Get relation target
    let (_, target) = gnfs.get_relations_info();
    let _: () = conn.set(keys.target(), target)
        .map_err(|e| format!("Redis SET target error: {}", e))?;

    // Initialize smooth count
    let _: () = conn.set(keys.smooth_count(), 0i64)
        .map_err(|e| format!("Redis SET smooth_count error: {}", e))?;

    // Create initial work chunks
    let initial_max_b = 500i64; // Start with B range [3, 500]
    let a_range = 150i64; // Match the MAX_VALUE_RANGE from poly_relations_sieve_progress
    let chunks = create_chunks(3, initial_max_b, 0, a_range, dist_config.chunk_size);
    info!("Created {} work chunks (B range: 3..{}, A range: 0..{})", chunks.len(), initial_max_b, a_range);

    // Push chunks to Redis sorted set
    for (priority, chunk) in chunks.iter().enumerate() {
        let _: () = conn.zadd(keys.chunks(), chunk.to_redis_member(), priority as f64)
            .map_err(|e| format!("Redis ZADD error: {}", e))?;
    }

    // Set status to sieving
    let _: () = conn.set(keys.status(), FactorizationStatus::Sieving.as_str())
        .map_err(|e| format!("Redis SET status error: {}", e))?;

    info!("Coordinator initialized. Waiting for workers to connect...");
    info!("Start workers with: gnfs --distributed worker {} {}", dist_config.redis_url, n);

    // Enter monitor loop
    monitor_loop(&mut conn, &keys, dist_config, cancel_token)
}

/// Monitor loop: check progress, reclaim stale chunks, expand if needed
fn monitor_loop(
    conn: &mut redis::Connection,
    keys: &RedisKeys,
    dist_config: &DistributedConfig,
    cancel_token: &CancellationToken,
) -> Result<(), String> {
    let reclaim_script = redis::Script::new(redis_client::RECLAIM_STALE_SCRIPT);

    let mut last_expand_b = 500i64;
    let mut monitor_interval = 0u64;

    loop {
        if cancel_token.is_cancellation_requested() {
            info!("Coordinator shutting down...");
            return Ok(());
        }

        thread::sleep(Duration::from_secs(5));
        monitor_interval += 5;

        // Check smooth relation count vs target
        let smooth_count: i64 = conn.get(keys.smooth_count())
            .map_err(|e| format!("Redis GET smooth_count error: {}", e))?;
        let target: i64 = conn.get(keys.target())
            .map_err(|e| format!("Redis GET target error: {}", e))?;

        // Check how many chunks are pending and claimed
        let pending: i64 = conn.zcard(keys.chunks())
            .map_err(|e| format!("Redis ZCARD error: {}", e))?;
        let claimed: i64 = conn.hlen(keys.claimed())
            .map_err(|e| format!("Redis HLEN error: {}", e))?;
        let completed: i64 = conn.scard(keys.completed())
            .map_err(|e| format!("Redis SCARD error: {}", e))?;

        // Count active workers
        let worker_count: i64 = conn.hlen(keys.heartbeat())
            .map_err(|e| format!("Redis HLEN heartbeat error: {}", e))?;

        if monitor_interval.is_multiple_of(15) {
            info!("Progress: {}/{} smooth relations ({:.1}%)",
                  smooth_count, target,
                  100.0 * smooth_count as f64 / target.max(1) as f64);
            info!("  Chunks: {} pending, {} claimed, {} completed",
                  pending, claimed, completed);
            info!("  Workers: {}", worker_count);
        }

        // Check if target reached
        if smooth_count >= target {
            info!("========================================");
            info!("TARGET REACHED: {} smooth relations found!", smooth_count);
            info!("========================================");

            let _: () = conn.set(keys.status(), FactorizationStatus::Matrix.as_str())
                .map_err(|e| format!("Redis SET status error: {}", e))?;

            info!("Set status to 'matrix'. Workers will stop claiming new chunks.");
            info!("Run 'gnfs --distributed collect <N>' to collect relations and solve.");
            return Ok(());
        }

        // Reclaim stale chunks every 30 seconds
        if monitor_interval.is_multiple_of(30) {
            let now = chrono::Utc::now().timestamp();
            let reclaimed: i64 = reclaim_script
                .key(keys.claimed())
                .key(keys.chunks())
                .arg(now)
                .arg(dist_config.claim_timeout_secs as i64)
                .invoke(conn)
                .unwrap_or(0);

            if reclaimed > 0 {
                warn!("Reclaimed {} stale chunks from dead workers", reclaimed);
            }
        }

        // Expand search space if all chunks consumed but target not met
        if pending == 0 && claimed == 0 && smooth_count < target {
            let new_b_start = last_expand_b + 1;
            let new_b_end = last_expand_b + 500;
            let a_range = 150i64;

            let new_chunks = create_chunks(
                new_b_start, new_b_end, 0, a_range, dist_config.chunk_size,
            );

            info!("Expanding search space: B range {}..{} ({} new chunks)",
                  new_b_start, new_b_end, new_chunks.len());

            let base_priority = completed as usize + new_chunks.len();
            for (i, chunk) in new_chunks.iter().enumerate() {
                let _: () = conn.zadd(
                    keys.chunks(),
                    chunk.to_redis_member(),
                    (base_priority + i) as f64,
                ).map_err(|e| format!("Redis ZADD error: {}", e))?;
            }

            last_expand_b = new_b_end;
        }
    }
}

/// Create work chunks from a B range
fn create_chunks(
    b_start: i64,
    b_end: i64,
    a_start: i64,
    a_range: i64,
    chunk_size: usize,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut b = if b_start % 2 == 0 { b_start + 1 } else { b_start }; // Ensure odd start

    while b < b_end {
        let chunk_b_end = (b + (chunk_size as i64 * 2)).min(b_end); // Step by 2 for odd B values
        chunks.push(Chunk::new(b, chunk_b_end, a_start, a_range));
        b = chunk_b_end;
    }

    chunks
}

/// Create a GNFS instance for computing params (polynomial + factor bases)
fn create_gnfs_for_distributed(n: &BigInt, config: &GnfsConfig) -> GNFSWrapper {
    // Reuse the same parameter selection logic from main.rs
    let cancel_token = CancellationToken::new();
    let polynomial_base = BigInt::from(31);
    let poly_degree = 3;

    let digits = n.to_string().len();
    let prime_bound = if digits <= 10 {
        let bound = 50 * (digits as i64 - 5).max(1);
        BigInt::from(bound.max(50))
    } else if digits <= 15 {
        let exponent = (digits as i32) - 10;
        let bound = (100.0 * 1.6_f64.powi(exponent)) as i64;
        BigInt::from(bound)
    } else if digits <= 30 {
        use num::ToPrimitive;
        let n_f64 = n.to_f64().unwrap_or(10_f64.powi(digits as i32));
        let ln_n = n_f64.ln();
        let ln_ln_n = ln_n.ln();
        let bound = (0.3 * (ln_n * ln_ln_n).sqrt().exp()) as i64;
        BigInt::from(bound)
    } else {
        use num::ToPrimitive;
        let n_f64 = n.to_f64().unwrap_or(10_f64.powi(digits as i32));
        let ln_n = n_f64.ln();
        let ln_ln_n = ln_n.ln();
        let bound = (0.5 * ln_n.powf(1.0 / 3.0) * ln_ln_n.powf(2.0 / 3.0)).exp() as i64;
        BigInt::from(bound)
    };

    let relation_quantity = 5;
    let relation_value_range = 50;

    GNFSWrapper::with_config(
        &cancel_token,
        n,
        &polynomial_base,
        poly_degree,
        &prime_bound,
        relation_quantity,
        relation_value_range,
        true,
        config.buffer.clone(),
    )
}

/// Serialize GNFS params to JSON string
fn serialize_params(gnfs: &GNFSWrapper) -> String {
    // Dispatch through the wrapper to get SerializableGNFS
    match gnfs {
        GNFSWrapper::Native64Signed(g) => {
            let s = SerializableGNFS::from(g);
            serde_json::to_string(&s).expect("Failed to serialize params")
        }
        GNFSWrapper::Native128Signed(g) => {
            let s = SerializableGNFS::from(g);
            serde_json::to_string(&s).expect("Failed to serialize params")
        }
        GNFSWrapper::Fixed256(g) => {
            let s = SerializableGNFS::from(g);
            serde_json::to_string(&s).expect("Failed to serialize params")
        }
        GNFSWrapper::Fixed512(g) => {
            let s = SerializableGNFS::from(g);
            serde_json::to_string(&s).expect("Failed to serialize params")
        }
        GNFSWrapper::Arbitrary(g) => {
            let s = SerializableGNFS::from(g);
            serde_json::to_string(&s).expect("Failed to serialize params")
        }
    }
}

/// Collect all smooth relations from the Redis stream
pub fn collect_relations(
    n: &BigInt,
    dist_config: &DistributedConfig,
) -> Result<Vec<SerializableRelation>, String> {
    let keys = RedisKeys::new(&n.to_string());

    let client = redis_client::create_client(&dist_config.redis_url)
        .map_err(|e| format!("Failed to create Redis client: {}", e))?;
    let mut conn = redis_client::get_connection_with_retry(&client)
        .map_err(|e| format!("Failed to connect to Redis: {}", e))?;

    info!("Collecting relations from Redis stream...");

    // Use XRANGE to read all entries from the stream as raw Values
    let raw: Value = redis::cmd("XRANGE")
        .arg(keys.relations())
        .arg("-")
        .arg("+")
        .query(&mut conn)
        .map_err(|e| format!("Redis XRANGE error: {}", e))?;

    let mut relations = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut total_entries = 0usize;

    // Parse the raw XRANGE response: array of [stream_id, [field, value, field, value, ...]]
    if let Value::Array(entries) = raw {
        total_entries = entries.len();
        for entry in entries {
            if let Value::Array(pair) = entry {
                // pair[0] = stream ID, pair[1] = field-value array
                if pair.len() >= 2 {
                    if let Value::Array(fields) = &pair[1] {
                        // fields is [field_name, field_value, field_name, field_value, ...]
                        let mut i = 0;
                        while i + 1 < fields.len() {
                            let field_name: String = FromRedisValue::from_redis_value(&fields[i])
                                .unwrap_or_default();
                            if field_name == "data" {
                                let data: String = FromRedisValue::from_redis_value(&fields[i + 1])
                                    .unwrap_or_default();
                                match serde_json::from_str::<SerializableRelation>(&data) {
                                    Ok(rel) => {
                                        let key = format!("{}:{}", rel.a, rel.b);
                                        if seen.insert(key) {
                                            relations.push(rel);
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to deserialize relation: {}", e);
                                    }
                                }
                            }
                            i += 2;
                        }
                    }
                }
            }
        }
    }

    info!("Collected {} unique relations ({} total stream entries)",
          relations.len(), total_entries);

    Ok(relations)
}

/// Print current status of a distributed job
pub fn print_status(
    n: &BigInt,
    dist_config: &DistributedConfig,
) -> Result<(), String> {
    let keys = RedisKeys::new(&n.to_string());

    let client = redis_client::create_client(&dist_config.redis_url)
        .map_err(|e| format!("Failed to create Redis client: {}", e))?;
    let mut conn = redis_client::get_connection_with_retry(&client)
        .map_err(|e| format!("Failed to connect to Redis: {}", e))?;

    let status: Option<String> = conn.get(keys.status())
        .map_err(|e| format!("Redis error: {}", e))?;
    let smooth_count: Option<i64> = conn.get(keys.smooth_count())
        .map_err(|e| format!("Redis error: {}", e))?;
    let target: Option<i64> = conn.get(keys.target())
        .map_err(|e| format!("Redis error: {}", e))?;
    let pending: i64 = conn.zcard(keys.chunks())
        .map_err(|e| format!("Redis error: {}", e))?;
    let claimed: i64 = conn.hlen(keys.claimed())
        .map_err(|e| format!("Redis error: {}", e))?;
    let completed: i64 = conn.scard(keys.completed())
        .map_err(|e| format!("Redis error: {}", e))?;
    let workers: i64 = conn.hlen(keys.heartbeat())
        .map_err(|e| format!("Redis error: {}", e))?;

    // Get stream length for relations count
    let stream_len: i64 = redis::cmd("XLEN")
        .arg(keys.relations())
        .query(&mut conn)
        .unwrap_or(0);

    println!("========================================");
    println!("DISTRIBUTED GNFS STATUS");
    println!("========================================");
    println!("Number:           {}", n);
    println!("Status:           {}", status.as_deref().unwrap_or("not started"));
    println!("Smooth relations: {} / {}",
             smooth_count.unwrap_or(0),
             target.unwrap_or(0));
    if let (Some(sc), Some(t)) = (smooth_count, target) {
        if t > 0 {
            println!("Progress:         {:.1}%", 100.0 * sc as f64 / t as f64);
        }
    }
    println!("Stream entries:   {}", stream_len);
    println!("Chunks pending:   {}", pending);
    println!("Chunks claimed:   {}", claimed);
    println!("Chunks completed: {}", completed);
    println!("Active workers:   {}", workers);
    println!("========================================");

    Ok(())
}
