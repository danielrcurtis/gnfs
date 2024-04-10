// src/factor/factor_pair.rs

use std::cmp::Eq;
use std::hash::{Hash, Hasher};
use serde::{Serialize, Deserialize};
use num::BigInt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FactorPair {
    pub p: i32,
    pub r: i32,
}

impl FactorPair {
    pub fn new_from_bigint(p: &BigInt, r: &BigInt) -> Self {
        FactorPair {
            p: p.to_i32().unwrap(),
            r: r.to_i32().unwrap(),
        }
    }

    pub fn new(p: i32, r: i32) -> Self {
        FactorPair { p, r }
    }
}

impl Hash for FactorPair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.p.hash(state);
        self.r.hash(state);
    }
}

impl Eq for FactorPair {}

impl PartialEq for FactorPair {
    fn eq(&self, other: &FactorPair) -> bool {
        self.p == other.p && self.r == other.r
    }
}

impl std::fmt::Display for FactorPair {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({},{})", self.p, self.r)
    }
}