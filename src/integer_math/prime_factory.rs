// src/integer_math/prime_factory.rs

use num::{BigInt, BigUint, FromPrimitive, Integer, Signed};
use num::bigint::{ToBigInt, ToBigUint};
use std::cmp::{max, min};
use std::ops::Range;
use crate::integer_math::fast_prime_sieve::FastPrimeSieve;
use crate::integer_math::factorization_factory::FactorizationFactory;

pub struct PrimeFactory {
    max_value: BigInt,
    primes_count: usize,
    primes_last: BigInt,
    primes: Vec<BigInt>,
}

impl PrimeFactory {
    pub fn new() -> Self {
        let mut factory = PrimeFactory {
            max_value: BigInt::from(10),
            primes_count: 0,
            primes_last: BigInt::from(0),
            primes: vec![],
        };
        factory.set_primes();
        factory
    }

    fn set_primes(&mut self) {
        self.primes = FastPrimeSieve::get_range(&BigUint::from(2u32), &self.max_value.to_biguint().unwrap())
            .map(|bi| bi.to_bigint().unwrap())
            .collect();
        self.primes_count = self.primes.len();
        self.primes_last = self.primes.last().unwrap().clone();
    }

    pub fn get_prime_enumerator(&self, start_index: usize, stop_index: Option<usize>) -> Range<usize> {
        let max_index = stop_index.unwrap_or(self.primes_count - 1);
        start_index..max_index
    }

    pub fn increase_max_value(&mut self, new_max_value: &BigInt) {
        let temp = max(new_max_value + 1000, &self.max_value + 100000);
        self.max_value = min(temp, BigInt::from(i32::max_value() - 1));
        self.set_primes();
    }

    pub fn get_index_from_value(&mut self, value: &BigInt) -> i32 {
        if value == &BigInt::from(-1) {
            return -1;
        }
        if &self.primes_last < value {
            self.increase_max_value(value);
        }
        let prime_value = self.primes.iter().find(|&&p| &p >= value).unwrap();
        self.primes.iter().position(|&p| p == *prime_value).unwrap() as i32 + 1
    }

    pub fn get_approximate_value_from_index(n: u64) -> BigUint {
        if n < 6 {
            return BigUint::from_u64(n).unwrap();
        }
        let fn_ = n as f64;
        let flogn = (n as f64).ln();
        let flog2n = flogn.ln();
        let upper = if n >= 688383 {
            fn_ * (flogn + flog2n - 1.0 + ((flog2n - 2.00) / flogn))
        } else if n >= 178974 {
            fn_ * (flogn + flog2n - 1.0 + ((flog2n - 1.95) / flogn))
        } else if n >= 39017 {
            fn_ * (flogn + flog2n - 0.9484)
        } else {
            fn_ * (flogn + 0.6000 * flog2n)
        };
        if upper >= u64::max_value() as f64 {
            panic!("{} > {}", upper, u64::max_value());
        }
        BigUint::from_f64(upper.ceil()).unwrap()
    }

    pub fn get_primes_from<'a>(&'a mut self, min_value: &'a BigInt) -> impl Iterator<Item = BigInt> + 'a {
        let start_index = self.get_index_from_value(min_value) as usize;
        self.get_prime_enumerator(start_index, None)
            .map(move |i| self.primes[i].clone())
    }

    pub fn get_primes_to<'a>(&'a mut self, max_value: &'a BigInt) -> impl Iterator<Item = BigInt> + 'a {
        if &self.primes_last < max_value {
            self.increase_max_value(max_value);
        }
        let primes = &self.primes;
        self.get_prime_enumerator(0, None)
            .take_while(move |&i| &primes[i] < max_value)
            .map(move |i| primes[i].clone())
    }

    pub fn is_prime(&self, value: &BigInt) -> bool {
        let abs_value = value.abs();
        self.primes.contains(&abs_value)
    }

    pub fn get_next_prime(from_value: &BigInt) -> BigInt {
        let mut result: BigUint = from_value.to_biguint().unwrap() + 1u32;
        if result.is_even() {
            result += 1u32;
        }
        while !FactorizationFactory::is_probable_prime(&result.to_bigint().unwrap()) {
            result += 2u32;
        }
        result.to_bigint().unwrap()
    }

    pub fn get_next_prime_from_i128(from_value: i128) -> BigInt {
        let mut result: BigUint = from_value.to_biguint().unwrap() + 1u32;
        if result.is_even() {
            result += 1u32;
        }
        while !FactorizationFactory::is_probable_prime(&result.to_bigint().unwrap()) {
            result += 2u32;
        }
        result.to_bigint().unwrap()
    }
}