// src/config/mod.rs

pub mod gnfs_config;

// Re-export main types for convenience
pub use gnfs_config::{GnfsConfig, BufferConfig, PerformanceConfig};
