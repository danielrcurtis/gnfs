// src/benchmark/mod.rs

pub mod system_info;
pub mod results;
pub mod runner;

pub use system_info::SystemInfo;
pub use results::{BenchmarkSuite, BenchmarkResult, FactorizationBenchmark};
pub use runner::BenchmarkRunner;
