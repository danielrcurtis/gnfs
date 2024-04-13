// src/core/solution.rs

use num::BigInt;

#[derive(Debug, Clone)]
pub struct Solution {
    pub p: BigInt,
    pub q: BigInt,
}

impl Solution {
    pub fn new(p: &BigInt, q: &BigInt) -> Self {
        Solution {
            p: p.clone(),
            q: q.clone(),
        }
    }
}