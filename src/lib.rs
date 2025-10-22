// src/lib.rs

#![recursion_limit = "512"]

pub mod core;
pub mod factor;
pub mod polynomial;
pub mod integer_math;
pub mod matrix;
pub mod relation_sieve;
pub mod square_root;
// Temporarily disabled due to compilation errors unrelated to polynomial optimization
// pub mod benchmark;