// src/distributed/redis_client.rs
//
// Redis connection management with retry logic.

use redis::{Client, Connection, RedisError};
use log::{info, warn};
use std::thread;
use std::time::Duration;

/// Create a Redis client with the given URL
pub fn create_client(redis_url: &str) -> Result<Client, RedisError> {
    Client::open(redis_url)
}

/// Get a connection with exponential backoff retry (up to 4 attempts)
pub fn get_connection_with_retry(client: &Client) -> Result<Connection, RedisError> {
    let delays = [2, 4, 8, 16];
    let mut last_err = None;

    for (attempt, delay) in std::iter::once(&0).chain(delays.iter()).enumerate() {
        if *delay > 0 {
            warn!("Redis connection attempt {} failed, retrying in {}s...", attempt, delay);
            thread::sleep(Duration::from_secs(*delay as u64));
        }

        match client.get_connection() {
            Ok(conn) => {
                if attempt > 0 {
                    info!("Redis connection established on attempt {}", attempt + 1);
                }
                return Ok(conn);
            }
            Err(e) => {
                last_err = Some(e);
            }
        }
    }

    Err(last_err.unwrap())
}

/// Lua script for atomic chunk claiming
/// Returns the chunk JSON string if successful, nil if no chunks available
pub const CLAIM_CHUNK_SCRIPT: &str = r#"
local chunk = redis.call('ZPOPMIN', KEYS[1])
if #chunk == 0 then return nil end
local chunk_id = chunk[1]
redis.call('HSET', KEYS[2], chunk_id, ARGV[1] .. ':' .. ARGV[2])
return chunk_id
"#;

/// Lua script for reclaiming stale chunks
/// Scans claimed hash, moves stale entries back to chunks sorted set
pub const RECLAIM_STALE_SCRIPT: &str = r#"
local claimed = redis.call('HGETALL', KEYS[1])
local reclaimed = 0
local now = tonumber(ARGV[1])
local timeout = tonumber(ARGV[2])

for i = 1, #claimed, 2 do
    local chunk_json = claimed[i]
    local claim_info = claimed[i+1]
    local colon_pos = string.find(claim_info, ':', -20)
    if colon_pos then
        local ts = tonumber(string.sub(claim_info, colon_pos + 1))
        if ts and (now - ts) > timeout then
            redis.call('HDEL', KEYS[1], chunk_json)
            redis.call('ZADD', KEYS[2], 0, chunk_json)
            reclaimed = reclaimed + 1
        end
    end
end
return reclaimed
"#;
