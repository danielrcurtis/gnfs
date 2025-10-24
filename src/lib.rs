// src/lib.rs

#![recursion_limit = "512"]

pub mod core;
pub mod backends;
pub mod config;
pub mod factor;
pub mod polynomial;
pub mod integer_math;
pub mod matrix;
pub mod relation_sieve;
pub mod square_root;
pub mod benchmark;
pub mod benchmark_cli;