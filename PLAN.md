# Distributed GNFS Sieving with Redis

## Motivation

Sieving dominates GNFS runtime (98%+). Each (A, B) pair is independently testable for smoothness — making sieving embarrassingly parallel across machines. Redis provides the coordination layer: work distribution, relation collection, and global progress tracking.

## Architecture Overview

```
                    ┌─────────────────────┐
                    │   Coordinator Node   │
                    │  (single instance)   │
                    ├─────────────────────┤
                    │ - Polynomial select  │
                    │ - Factor base build  │
                    │ - Partition search   │
                    │   space into chunks  │
                    │ - Monitor progress   │
                    │ - Trigger matrix     │
                    │   solve when ready   │
                    └────────┬────────────┘
                             │ Redis
              ┌──────────────┼──────────────┐
              │              │              │
        ┌─────▼─────┐ ┌─────▼─────┐ ┌─────▼─────┐
        │  Worker 1  │ │  Worker 2  │ │  Worker N  │
        │            │ │            │ │            │
        │ Claim chunk│ │ Claim chunk│ │ Claim chunk│
        │ Sieve (A,B)│ │ Sieve (A,B)│ │ Sieve (A,B)│
        │ Push smooth│ │ Push smooth│ │ Push smooth│
        │ relations  │ │ relations  │ │ relations  │
        └────────────┘ └────────────┘ └────────────┘
```

## Redis Data Model

### Keys and Structures

```
gnfs:{n}:params          → JSON   (SerializableGNFS: polynomial, factor bases, bounds)
gnfs:{n}:status          → STRING ("sieving" | "matrix" | "sqrt" | "done" | "failed")
gnfs:{n}:target          → INT    (smooth_relations_target_quantity)
gnfs:{n}:smooth_count    → INT    (atomic counter, incremented by workers)

gnfs:{n}:chunks          → SORTED SET  (score = priority, member = "b_start:b_end:a_start:a_range")
gnfs:{n}:claimed         → HASH   (chunk_id → worker_id:timestamp)
gnfs:{n}:completed       → SET    (chunk_ids that are done)

gnfs:{n}:relations       → STREAM (Redis Stream of smooth relations as JSONL)
gnfs:{n}:heartbeat       → HASH   (worker_id → last_heartbeat_epoch)

gnfs:{n}:worker:{id}:progress → HASH (current_a, current_b, relations_found, chunk_id)
```

### Why Redis Streams (not Pub/Sub)

- **Persistence**: Streams retain messages; pub/sub drops if no listener
- **Consumer groups**: Multiple consumers can process without duplication
- **Backpressure**: Workers write at their own pace
- **Replay**: Can re-read all relations for matrix stage
- **Trimming**: Can trim after matrix solve to reclaim memory

## Search Space Partitioning

### Strategy: B-range Chunks with A-range Subdivision

The search space is 2D: `A ∈ [-value_range, +value_range]` for each `B ∈ [3, max_b]`.

**Partition scheme:**
```
Chunk = { b_start, b_end, a_start, a_range }

Example for max_b=1000, chunk_size=16 (B values per chunk):
  Chunk 0: B=[3,19],     A=[-150, 150]
  Chunk 1: B=[21,37],    A=[-150, 150]
  Chunk 2: B=[39,55],    A=[-150, 150]
  ...
  Chunk 62: B=[987,1003], A=[-150, 150]
```

B values are odd (coprimality optimization), so chunks step by 2.

**Dynamic expansion:** When all chunks complete but target not reached:
- Coordinator doubles `max_b` and creates new chunks
- New chunks added to `gnfs:{n}:chunks` sorted set with lower priority (higher score)

### Chunk Granularity

Each chunk should take ~5-30 seconds of wall time per worker:
- Too small: Redis overhead dominates
- Too large: Poor load balancing, stale claims on failure
- Sweet spot: ~16 B values × 300 A values = ~4800 (A,B) pairs

## Implementation Plan

### Step 1: Redis Client Integration

**New dependency in Cargo.toml:**
```toml
redis = { version = "0.27", features = ["tokio-comp", "streams"] }
```

**New module:** `src/distributed/`
```
src/distributed/
├── mod.rs              # Module exports, feature flag
├── redis_client.rs     # Connection pool, retry logic
├── coordinator.rs      # Search space management, chunk creation
├── worker.rs           # Chunk claiming, sieving, relation pushing
├── messages.rs         # Serializable message types
└── config.rs           # Redis URL, worker ID, timeouts
```

**Config additions to `gnfs.toml`:**
```toml
[distributed]
enabled = false
redis_url = "redis://127.0.0.1:6379"
worker_id = "auto"           # auto-generates from hostname+pid
chunk_size = 16               # B values per chunk
heartbeat_interval_secs = 5
claim_timeout_secs = 120      # reclaim chunk if worker dies
```

### Step 2: Coordinator Node

**File:** `src/distributed/coordinator.rs`

```rust
pub struct Coordinator {
    redis: redis::Client,
    n: BigInt,
    gnfs_params: SerializableGNFS,
}

impl Coordinator {
    /// Initialize: publish params, create initial chunks
    pub async fn initialize(&self, gnfs: &GNFSWrapper) -> Result<()>;

    /// Partition [b_start, b_end] into chunks and push to sorted set
    fn create_chunks(&self, b_start: i64, b_end: i64, a_range: i64) -> Vec<Chunk>;

    /// Monitor loop: check smooth_count vs target, reclaim stale chunks
    pub async fn monitor(&self, cancel_token: &CancellationToken) -> Result<()>;

    /// When target reached: set status="matrix", trigger collection
    pub async fn collect_relations(&self) -> Result<Vec<SerializableRelation>>;

    /// Reclaim chunks from dead workers (heartbeat timeout)
    async fn reclaim_stale_chunks(&self) -> Result<usize>;

    /// Dynamically expand search space if needed
    async fn expand_search_space(&self) -> Result<()>;
}
```

**Coordinator flow:**
1. Run `GNFS::with_config()` locally to compute polynomial + factor bases
2. Serialize `SerializableGNFS` to `gnfs:{n}:params`
3. Create initial chunks from `B=[3, initial_max_b]`
4. Enter monitor loop:
   - Every 5s: check `gnfs:{n}:smooth_count` vs target
   - Every 30s: scan heartbeats, reclaim stale chunks
   - When no unclaimed/claimed chunks remain and target not met: expand
   - When target met: set status="matrix" and collect from stream

### Step 3: Worker Node

**File:** `src/distributed/worker.rs`

```rust
pub struct Worker<T: GnfsInteger> {
    redis: redis::Client,
    worker_id: String,
    gnfs: GNFS<T>,  // Reconstructed from params (read-only during sieving)
}

impl<T: GnfsInteger> Worker<T> {
    /// Connect, load params, reconstruct GNFS<T>
    pub async fn connect(redis_url: &str, n: &BigInt) -> Result<Self>;

    /// Main loop: claim chunk → sieve → push relations → repeat
    pub async fn run(&mut self, cancel_token: &CancellationToken) -> Result<()>;

    /// Atomic chunk claim via Redis WATCH/MULTI or Lua script
    async fn claim_chunk(&self) -> Result<Option<Chunk>>;

    /// Sieve a single chunk, pushing relations to stream
    async fn sieve_chunk(&self, chunk: &Chunk) -> Result<usize>;

    /// Push smooth relations to Redis stream
    async fn push_relations(&self, relations: Vec<SerializableRelation>) -> Result<()>;

    /// Heartbeat sender (background task)
    async fn heartbeat_loop(&self, cancel_token: &CancellationToken);
}
```

**Worker flow:**
1. Connect to Redis, read `gnfs:{n}:params`
2. Reconstruct `GNFS<T>` via `from_checkpoint()` (no sieving state needed — just params)
3. Rebuild factor bases locally (they're derived from bounds, not serialized as full lists)
4. Enter claim loop:
   - `ZPOPMIN gnfs:{n}:chunks` → claim lowest-priority chunk
   - Write claim to `gnfs:{n}:claimed` with timestamp
   - Sieve all (A, B) pairs in chunk using existing rayon parallelism
   - For each smooth relation: `XADD gnfs:{n}:relations * data <json>`
   - `INCRBY gnfs:{n}:smooth_count <found_count>`
   - Move chunk to `gnfs:{n}:completed`
   - Check `gnfs:{n}:status` — if "matrix", stop claiming

### Step 4: Relation Collection via Redis Streams

**Collection for matrix solve:**
```rust
/// Read all relations from the Redis stream
pub async fn collect_all_relations<T: GnfsInteger>(
    redis: &redis::Client,
    n: &BigInt,
) -> Result<Vec<Relation<T>>> {
    // XRANGE gnfs:{n}:relations - +
    // Deserialize each entry back to Relation<T>
    // Deduplicate by (a, b) pair
}
```

**Deduplication:** Workers may produce overlapping relations if chunks are reclaimed and re-processed. Deduplicate by `(a, b)` pair using a HashSet during collection.

### Step 5: Chunk Claiming Protocol (Atomic)

Use a Lua script for atomic claim to prevent race conditions:

```lua
-- KEYS[1] = gnfs:{n}:chunks (sorted set)
-- KEYS[2] = gnfs:{n}:claimed (hash)
-- ARGV[1] = worker_id
-- ARGV[2] = timestamp
local chunk = redis.call('ZPOPMIN', KEYS[1])
if #chunk == 0 then return nil end
local chunk_id = chunk[1]
redis.call('HSET', KEYS[2], chunk_id, ARGV[1] .. ':' .. ARGV[2])
return chunk_id
```

### Step 6: Failure Recovery

**Worker crash:** Coordinator's monitor loop detects missing heartbeat after `claim_timeout_secs`. Reclaims chunk:
```
HDEL gnfs:{n}:claimed <chunk_id>
ZADD gnfs:{n}:chunks <priority> <chunk_id>
```

**Coordinator crash:** Stateless restart — reads all state from Redis. Chunks, claims, and relations are persistent.

**Redis crash:** If using Redis persistence (AOF or RDB), state survives restart. Relations in the stream are durable. Workers reconnect with exponential backoff.

### Step 7: CLI Integration

**New CLI modes:**
```bash
# Start as coordinator (computes params, creates chunks, monitors)
gnfs --distributed coordinator --redis redis://localhost:6379 738883

# Start as worker (connects, claims chunks, sieves)
gnfs --distributed worker --redis redis://localhost:6379 738883

# Check status
gnfs --distributed status --redis redis://localhost:6379 738883

# Collect results and run matrix + sqrt locally
gnfs --distributed collect --redis redis://localhost:6379 738883
```

**Feature flag:** Gate behind `--features distributed` to keep Redis optional:
```toml
[features]
default = []
distributed = ["redis"]
```

## Data Flow Diagram

```
COORDINATOR                    REDIS                         WORKERS
    │                            │                              │
    ├─ Compute polynomial ──────►│ SET gnfs:{n}:params          │
    ├─ Create chunks ───────────►│ ZADD gnfs:{n}:chunks ...     │
    ├─ SET status=sieving ──────►│                              │
    │                            │                              │
    │                            │◄── GET gnfs:{n}:params ──────┤
    │                            │    (reconstruct GNFS<T>)     │
    │                            │                              │
    │                            │◄── ZPOPMIN chunks ───────────┤
    │                            │──► chunk_id ────────────────►│
    │                            │                              ├─ Sieve chunk
    │                            │◄── XADD relations {...} ─────┤  (rayon parallel)
    │                            │◄── INCRBY smooth_count N ────┤
    │                            │◄── HSET heartbeat ───────────┤
    │                            │                              │
    ├─ Monitor smooth_count ────►│                              │
    ├─ Reclaim stale chunks ────►│                              │
    │                            │                              │
    │  (target reached)          │                              │
    ├─ SET status=matrix ───────►│                              │
    ├─ XRANGE relations ────────►│                              │
    │◄─ all relations ───────────┤                              │
    ├─ Matrix solve (local)      │                              │
    ├─ Square root (local)       │                              │
    ├─ SET status=done ─────────►│                              │
    │                            │                              │
```

## Performance Considerations

### Redis Stream Throughput

- Each smooth relation is ~500-2000 bytes JSON
- At 1000 relations/sec across all workers: ~1-2 MB/s write to Redis
- Redis handles 100K+ writes/sec — not a bottleneck
- Stream trimming after matrix collect prevents unbounded growth

### Network Overhead vs Computation

- Sieving a single (A,B) pair: ~10-100μs (trial division of norms)
- Redis XADD round-trip: ~0.1-1ms (LAN)
- **Batch relation pushes**: Buffer 50-100 relations locally, push in one pipeline
- Worker-local rayon parallelism handles the inner loop; Redis handles outer coordination

### Optimal Worker Count

- Each worker uses rayon internally (multi-core)
- Ideal: 1 worker per machine, rayon uses all cores
- For single-machine testing: 1-4 workers (oversubscription works but adds overhead)
- For cluster: 1 worker per node, chunk_size tuned to ~10-30s per chunk

## Migration Path (Incremental)

1. **Phase 1** (this plan): Redis-based coordination for sieving stage only. Matrix solve and square root remain local on coordinator.

2. **Phase 2** (future): Distributed matrix solve. Much harder — Gaussian elimination has data dependencies. Could use Block Wiedemann algorithm for parallelism.

3. **Phase 3** (future): GPU offload for sieving inner loop via OpenCL/CUDA. Workers become GPU dispatchers.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Redis single point of failure | Workers idle | Redis Sentinel/Cluster for HA |
| Duplicate relations from reclaimed chunks | Wasted work, matrix issues | Deduplicate by (a,b) at collection |
| Worker produces invalid relations | Bad matrix input | Verify smoothness on collection |
| Search space exhaustion | Can't reach target | Coordinator detects, logs, adjusts bounds |
| Network partition | Workers can't push | Local buffering + retry with backoff |
| Memory pressure on Redis | OOM | Stream trimming, max stream length |

## Estimated Effort

| Component | Files | Complexity | Estimate |
|-----------|-------|------------|----------|
| Redis client + config | 2 | Low | Small |
| Coordinator | 1 | Medium | Medium |
| Worker | 1 | Medium | Medium |
| Message types | 1 | Low | Small |
| CLI integration | 1 (main.rs) | Low | Small |
| Chunk claiming (Lua) | 1 | Low | Small |
| Relation collection | 1 | Medium | Small |
| Tests | 2-3 | Medium | Medium |
| **Total** | **~10 files** | | |

## Open Questions

1. **Should workers also save local JSONL as backup?** (Defense against Redis data loss)
2. **Should the coordinator be stateless?** (Fully Redis-driven vs coordinator-in-memory)
3. **Redis Cluster vs single instance?** (For very large factorizations with many workers)
4. **Should we support heterogeneous backends?** (e.g., some workers use Fixed256, others BigInt — relations are backend-agnostic after serialization)
