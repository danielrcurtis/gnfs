// src/integer_math/legendre.rs

use num::{BigInt, Zero};
use num::Integer;
use num::ToPrimitive;

pub struct Legendre;

impl Legendre {
    /// Legendre Symbol returns 1 for a (nonzero) quadratic residue mod p, -1 for a non-quadratic residue (non-residue), or 0 on zero.
    pub fn symbol(a: &BigInt, p: &BigInt) -> i32 {
        if p < &BigInt::from(2) {
            panic!("Parameter 'p' must not be < 2, but you have supplied: {}", p);
        }
        if a.is_zero() {
            return 0;
        }
        if a == &BigInt::from(1) {
            return 1;
        }

        let result = if a.mod_floor(&BigInt::from(2)) == BigInt::zero() {
            let result = Self::symbol(&(a >> 2), p); // >> right shift == /2
            if (&(p * p - 1) & BigInt::from(8)) != BigInt::zero() {
                // Convert 8 to BigInt
                -result
            } else {
                result
            }
        } else {
            let result = Self::symbol(&p.mod_floor(a), a);
            if (&((a - 1) * (p - 1)) & BigInt::from(4)) != BigInt::zero() {
                // Convert 4 to BigInt
                -result
            } else {
                result
            }
        };

        result
    }

    /// Find r such that (r | m) = goal, where (r | m) is the Legendre symbol, and m = modulus
    pub fn symbol_search(start: &BigInt, modulus: &BigInt, goal: &BigInt) -> BigInt {
        if goal != &BigInt::from(-1) && goal != &BigInt::zero() && goal != &BigInt::from(1) {
            panic!("Parameter 'goal' may only be -1, 0 or 1. It was {}.", goal);
        }

        // OPTIMIZATION: Try small primes first before linear search
        // Most moduli have small primes as quadratic non-residues
        // This avoids O(modulus) linear search when modulus is large (e.g., 95 million)
        let small_candidates = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71];
        for candidate in small_candidates {
            let big_candidate = BigInt::from(candidate);
            if &big_candidate >= start && Self::symbol(&big_candidate, modulus) == goal.to_i32().unwrap() {
                return big_candidate;
            }
        }

        // Fallback to linear search if no small prime works
        let mut counter = start.clone();
        let max = &counter + modulus + 1;

        loop {
            if Self::symbol(&counter, modulus) == goal.to_i32().unwrap() {
                return counter;
            }
            counter += 1;
            if counter > max {
                break;
            }
        }

        panic!("Legendre symbol matching criteria not found.");
    }
}