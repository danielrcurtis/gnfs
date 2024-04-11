// src/realation_sieve/relation.rs

use num::BigInt;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use crate::core::count_dictionary::CountDictionary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    #[serde(rename = "A")]
    pub a: BigInt,
    #[serde(rename = "B")]
    pub b: BigInt,
    #[serde(rename = "AlgebraicNorm")]
    pub algebraic_norm: BigInt,
    #[serde(rename = "RationalNorm")]
    pub rational_norm: BigInt,
    #[serde(rename = "AlgebraicQuotient")]
    pub algebraic_quotient: BigInt,
    #[serde(rename = "RationalQuotient")]
    pub rational_quotient: BigInt,
    #[serde(rename = "AlgebraicFactorization")]
    pub algebraic_factorization: CountDictionary,
    #[serde(rename = "RationalFactorization")]
    pub rational_factorization: CountDictionary,
    #[serde(skip)]
    pub is_persisted: bool,
}

impl Relation {
    pub fn new() -> Self {
        Relation {
            a: BigInt::default(),
            b: BigInt::default(),
            algebraic_norm: BigInt::default(),
            rational_norm: BigInt::default(),
            algebraic_quotient: BigInt::default(),
            rational_quotient: BigInt::default(),
            algebraic_factorization: CountDictionary::new(),
            rational_factorization: CountDictionary::new(),
            is_persisted: false,
        }
    }

    pub fn is_smooth(&self) -> bool {
        self.is_rational_quotient_smooth() && self.is_algebraic_quotient_smooth()
    }

    pub fn is_rational_quotient_smooth(&self) -> bool {
        self.rational_quotient == BigInt::from(1) || self.rational_quotient == BigInt::from(0)
    }

    pub fn is_algebraic_quotient_smooth(&self) -> bool {
        self.algebraic_quotient == BigInt::from(1) || self.algebraic_quotient == BigInt::from(0)
    }

    pub fn apply(&self, x: &BigInt) -> BigInt {
        &self.a + &self.b * x
    }
}

impl PartialEq for Relation {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b
    }
}

impl Eq for Relation {}

impl Hash for Relation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.a.hash(state);
        self.b.hash(state);
    }
}

impl PartialOrd for Relation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.a.cmp(&other.a).then(self.b.cmp(&other.b)))
    }
}

impl Ord for Relation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}