// src/factor/factor_pair_collection.rs

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use num::BigInt;
use serde::{Deserialize, Serialize};
use crate::core::gnfs::GNFS;
use crate::polynomial::polynomial::Polynomial;
use crate::factor::factor_pair::FactorPair;
use num::ToPrimitive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorPairCollection(pub Vec<FactorPair>);

impl FactorPairCollection {
    pub fn new() -> Self {
        FactorPairCollection(Vec::new())
    }

    pub fn from_collection(collection: &[FactorPair]) -> Self {
        FactorPairCollection(collection.to_vec())
    }

    pub fn to_string(&self) -> String {
        self.0.iter().map(|factor| factor.to_string()).collect::<Vec<String>>().join("\t")
    }

    pub fn to_string_take(&self, take: usize) -> String {
        self.0.iter().take(take).map(|factor| factor.to_string()).collect::<Vec<String>>().join("\t")
    }
}

impl Iterator for FactorPairCollection {
    type Item = FactorPair;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }

}

impl Default for FactorPairCollection {
    fn default() -> Self {
        FactorPairCollection::new()
    }
}


impl ExactSizeIterator for FactorPairCollection {
    fn len(&self) -> usize {
        self.0.len()
    }
}


pub struct Factory;

impl Factory {
    // array of (p, m % p) up to bound
    // quantity = phi(bound)
    pub fn build_rational_factor_pair_collection(gnfs: &GNFS) -> FactorPairCollection {
        let result: Vec<FactorPair> = gnfs.prime_factor_base.rational_factor_base.iter()
            .map(|&p| FactorPair::new(p.to_i32().unwrap(), (&gnfs.polynomial_base % p).to_i32().unwrap())) // Use to_i32() instead of as
            .collect();
        FactorPairCollection::from_collection(&result)
    }

    // array of (p, r) where ƒ(r) % p == 0
    // quantity = 2-3 times RFB.quantity
    pub fn build_algebraic_factor_pair_collection(cancel_token: &Arc<AtomicBool>, gnfs: &GNFS) -> FactorPairCollection {
        let roots = Self::find_polynomial_roots_in_range(
            cancel_token,
            &gnfs.current_polynomial,
            &gnfs.prime_factor_base.algebraic_factor_base,
            &BigInt::from(0),
            &gnfs.prime_factor_base.algebraic_factor_base_max,
            2000,
        );
        FactorPairCollection::from_collection(&roots)
    }

    // array of (p, r) where ƒ(r) % p == 0
    // quantity =< 100
    // magnitude p > AFB.Last().p
    pub fn build_quadratic_factor_pair_collection(cancel_token: &Arc<AtomicBool>, gnfs: &GNFS) -> FactorPairCollection {
        let roots = Self::find_polynomial_roots_in_range(
            cancel_token,
            &gnfs.current_polynomial,
            &gnfs.prime_factor_base.quadratic_factor_base,
            &BigInt::from(2),
            &gnfs.prime_factor_base.quadratic_factor_base_max,
            gnfs.prime_factor_base.quadratic_base_count as usize, // Convert i32 to usize
        );
        FactorPairCollection::from_collection(&roots)
    }

    pub fn find_polynomial_roots_in_range(
        cancel_token: &Arc<AtomicBool>,
        polynomial: &Polynomial,
        primes: &[BigInt],
        range_from: &BigInt,
        range_to: &BigInt,
        total_factor_pairs: usize,
    ) -> Vec<FactorPair> {
        let mut result = Vec::new();
        let mut r = range_from.clone();
        let mod_list: Vec<BigInt> = primes.to_vec();

        while !cancel_token.load(Ordering::SeqCst) && &r < range_to && result.len() < total_factor_pairs {
            // Finds p such that ƒ(r) ≡ 0 (mod p)
            let roots = Self::get_roots_mod(polynomial, &r, &mod_list);
            if !roots.is_empty() {
                result.extend(roots.iter().map(|&p| FactorPair::new_from_bigint(&p, &r)));
            }
            r += 1;
        }

        result.sort_by_key(|factor_pair| factor_pair.p);
        result
    }

    /// Given a list of primes, returns primes p such that ƒ(r) ≡ 0 (mod p)
    pub fn get_roots_mod(polynomial: &Polynomial, base_m: &BigInt, mod_list: &[BigInt]) -> Vec<BigInt> {
        let poly_result = polynomial.evaluate(base_m);
        let result: Vec<BigInt> = mod_list.iter()
            .filter(|&mod_val| &poly_result % mod_val == BigInt::from(0))
            .cloned()
            .collect();
        result
    }
}