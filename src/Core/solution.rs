// src/core/solution.rs

use num::BigInt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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