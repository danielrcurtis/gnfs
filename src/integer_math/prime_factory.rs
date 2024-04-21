// src/integer_math/prime_factory.rs

use log::debug;
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
        factory.set_common_primes();
        factory
    }

    fn set_common_primes(&mut self) {
        self.primes = vec![BigInt::from(2), BigInt::from(3), BigInt::from(5), BigInt::from(7), BigInt::from(11), BigInt::from(13), BigInt::from(17), BigInt::from(19), BigInt::from(23), BigInt::from(29), BigInt::from(31), BigInt::from(37), BigInt::from(41), BigInt::from(43), BigInt::from(47), BigInt::from(53), BigInt::from(59), BigInt::from(61), BigInt::from(67), BigInt::from(71), BigInt::from(73), BigInt::from(79), BigInt::from(83), BigInt::from(89), BigInt::from(97), BigInt::from(101), BigInt::from(103), BigInt::from(107), BigInt::from(109), BigInt::from(113), BigInt::from(127), BigInt::from(131), BigInt::from(137), BigInt::from(139), BigInt::from(149), BigInt::from(151), BigInt::from(157), BigInt::from(163), BigInt::from(167), BigInt::from(173), BigInt::from(179), BigInt::from(181), BigInt::from(191), BigInt::from(193), BigInt::from(197), BigInt::from(199), BigInt::from(211), BigInt::from(223), BigInt::from(227), BigInt::from(229), BigInt::from(233), BigInt::from(239), BigInt::from(241), BigInt::from(251), BigInt::from(257), BigInt::from(263), BigInt::from(269), BigInt::from(271), BigInt::from(277), BigInt::from(281), BigInt::from(283), BigInt::from(293), BigInt::from(307), BigInt::from(311), BigInt::from(313), BigInt::from(317), BigInt::from(331), BigInt::from(337), BigInt::from(347), BigInt::from(349), BigInt::from(353), BigInt::from(359), BigInt::from(367), BigInt::from(373), BigInt::from(379), BigInt::from(383), BigInt::from(389), BigInt::from(397), BigInt::from(401), BigInt::from(409), BigInt::from(419), BigInt::from(421), BigInt::from(431), BigInt::from(433), BigInt::from(439), BigInt::from(443), BigInt::from(449)];
        self.primes_count = self.primes.len();
        self.primes_last = self.primes.last().unwrap().clone();
    }

    fn set_primes(&mut self) {
        self.primes = FastPrimeSieve::get_range(&BigUint::from(1u32), &BigUint::from(8192u32))
            .map(|bi| bi.to_bigint().unwrap())
            .collect();
        self.primes_count = self.primes.len();
        self.primes_last = self.primes.last().unwrap().clone();
    }

    pub fn get_prime_enumerator(&self, start_index: usize, stop_index: Option<usize>) -> Range<usize> {
        debug!("In prime_factory get_prime_enumerator with start_index: {}, stop_index: {:?}", start_index, stop_index);
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
        let prime_index = self.primes.iter().position(|p| p >= value).unwrap();
        prime_index as i32 + 1
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
        debug!("In prime_factory get_primes_to with max_value: {}", max_value);
        if &self.primes_last < max_value {
            self.increase_max_value(max_value);
        }
        debug!("In prime_factory get_primes_to after increase_max_value");
        let primes = &self.primes;
        debug!("In prime_factory get_primes_to after primes, starting enumerator");
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