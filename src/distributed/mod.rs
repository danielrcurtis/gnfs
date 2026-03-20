// src/distributed/mod.rs
//
// Distributed GNFS sieving using Redis for coordination.
// Feature-gated behind `--features distributed`.

#[cfg(feature = "distributed")]
pub mod messages;
#[cfg(feature = "distributed")]
pub mod redis_client;
#[cfg(feature = "distributed")]
pub mod coordinator;
#[cfg(feature = "distributed")]
pub mod worker;

/// Distributed mode subcommands
#[derive(Debug, Clone, PartialEq)]
pub enum DistributedMode {
    /// Start as coordinator: compute params, partition search space, monitor progress
    Coordinator,
    /// Start as worker: claim chunks, sieve, push relations
    Worker,
    /// Query current status from Redis
    Status,
    /// Collect relations from Redis and run matrix solve + square root locally
    Collect,
}

impl DistributedMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "coordinator" => Some(DistributedMode::Coordinator),
            "worker" => Some(DistributedMode::Worker),
            "status" => Some(DistributedMode::Status),
            "collect" => Some(DistributedMode::Collect),
            _ => None,
        }
    }
}
