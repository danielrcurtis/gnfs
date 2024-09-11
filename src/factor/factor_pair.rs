// src/factor/factor_pair.rs

use std::cmp::Eq;
use std::hash::{Hash, Hasher};
use serde::{Serialize, Deserialize};
use num::BigInt;
use num::ToPrimitive;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FactorPair {
    pub p: i128,
    pub r: i128,
}

impl FactorPair {
    pub fn new_from_bigint(p: &BigInt, r: &BigInt) -> Result<Self, String> {
        let p = p.to_i128().ok_or("BigInt value for p is out of range for i128")?;
        let r = r.to_i128().ok_or("BigInt value for r is out of range for i128")?;
        Ok(FactorPair { p, r })
        }
    

    pub fn new(p: i128, r: i128) -> Self {
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