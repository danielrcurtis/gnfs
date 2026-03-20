// src/distributed/messages.rs
//
// Serializable message types for Redis communication.

use serde::{Serialize, Deserialize};

/// A work chunk representing a partition of the (A, B) search space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique identifier for this chunk
    pub id: String,
    /// Starting B value (inclusive)
    pub b_start: i64,
    /// Ending B value (exclusive)
    pub b_end: i64,
    /// Starting A value
    pub a_start: i64,
    /// A range (search from a_start to a_start + a_range)
    pub a_range: i64,
}

impl Chunk {
    pub fn new(b_start: i64, b_end: i64, a_start: i64, a_range: i64) -> Self {
        let id = format!("b{}_{}_a{}_{}", b_start, b_end, a_start, a_range);
        Chunk { id, b_start, b_end, a_start, a_range }
    }

    /// Serialize to a string for Redis sorted set member
    pub fn to_redis_member(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize chunk")
    }

    /// Deserialize from a Redis sorted set member
    pub fn from_redis_member(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("Failed to deserialize chunk: {}", e))
    }
}

/// Status update from a worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStatus {
    pub worker_id: String,
    pub current_chunk: Option<String>,
    pub relations_found: usize,
    pub timestamp: i64,
}

/// Global factorization status stored in Redis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FactorizationStatus {
    /// Coordinator has published params, sieving in progress
    Sieving,
    /// Enough relations found, matrix solve phase
    Matrix,
    /// Square root extraction phase
    SquareRoot,
    /// Factorization complete
    Done,
    /// Factorization failed
    Failed,
}

impl FactorizationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FactorizationStatus::Sieving => "sieving",
            FactorizationStatus::Matrix => "matrix",
            FactorizationStatus::SquareRoot => "sqrt",
            FactorizationStatus::Done => "done",
            FactorizationStatus::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sieving" => Some(FactorizationStatus::Sieving),
            "matrix" => Some(FactorizationStatus::Matrix),
            "sqrt" => Some(FactorizationStatus::SquareRoot),
            "done" => Some(FactorizationStatus::Done),
            "failed" => Some(FactorizationStatus::Failed),
            _ => None,
        }
    }
}

/// Redis key namespace for a specific factorization job
pub struct RedisKeys {
    prefix: String,
}

impl RedisKeys {
    pub fn new(n: &str) -> Self {
        RedisKeys {
            prefix: format!("gnfs:{}", n),
        }
    }

    /// SerializableGNFS parameters (JSON)
    pub fn params(&self) -> String { format!("{}:params", self.prefix) }

    /// Factorization status string
    pub fn status(&self) -> String { format!("{}:status", self.prefix) }

    /// Smooth relations target count
    pub fn target(&self) -> String { format!("{}:target", self.prefix) }

    /// Atomic smooth relation counter
    pub fn smooth_count(&self) -> String { format!("{}:smooth_count", self.prefix) }

    /// Sorted set of unclaimed work chunks (score = priority)
    pub fn chunks(&self) -> String { format!("{}:chunks", self.prefix) }

    /// Hash of claimed chunks: chunk_id → worker_id:timestamp
    pub fn claimed(&self) -> String { format!("{}:claimed", self.prefix) }

    /// Set of completed chunk IDs
    pub fn completed(&self) -> String { format!("{}:completed", self.prefix) }

    /// Redis Stream of smooth relations (JSONL entries)
    pub fn relations(&self) -> String { format!("{}:relations", self.prefix) }

    /// Hash of worker heartbeats: worker_id → timestamp
    pub fn heartbeat(&self) -> String { format!("{}:heartbeat", self.prefix) }

    /// Per-worker progress hash
    pub fn worker_progress(&self, worker_id: &str) -> String {
        format!("{}:worker:{}:progress", self.prefix, worker_id)
    }
}
