// src/core/sieve_range.rs

use num::BigInt;

pub struct SieveRange;

impl SieveRange {
    pub fn get_sieve_range(maximum_range: &BigInt) -> Vec<BigInt> {
        Self::get_sieve_range_continuation(&BigInt::from(1), maximum_range)
    }

    pub fn get_sieve_range_continuation(current_value: &BigInt, maximum_range: &BigInt) -> Vec<BigInt> {
        let mut result = Vec::new();
        let mut counter = current_value.abs();
        let mut flip_flop = current_value.sign() != num::bigint::Sign::Minus;

        while &counter <= maximum_range {
            if flip_flop {
                result.push(counter.clone());
                flip_flop = false;
            } else {
                result.push(-&counter);
                counter += 1;
                flip_flop = true;
            }
        }

        result
    }
}