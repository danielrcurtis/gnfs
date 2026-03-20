// src/core/solution.rs

use num::BigInt;
use std::fmt::Display;

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

impl Display for Solution {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "p: {}, q: {}", self.p, self.q)
    }
}