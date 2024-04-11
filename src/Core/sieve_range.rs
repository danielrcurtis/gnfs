use num::BigInt;
use num::bigint::Sign;
use std::iter::Iterator;

pub struct SieveRange;

impl SieveRange {
    pub fn get_sieve_range(maximum_range: &BigInt) -> impl Iterator<Item = BigInt> {
        Self::get_sieve_range_continuation(&BigInt::from(1), maximum_range)
    }

    pub fn get_sieve_range_continuation(
        current_value: &BigInt,
        maximum_range: &BigInt,
    ) -> impl Iterator<Item = BigInt> {
        let max = maximum_range.clone();
        let mut counter = current_value.abs();
        let mut flip_flop = current_value.sign() != Sign::Minus;

        std::iter::from_fn(move || {
            if counter <= max {
                let result = if flip_flop {
                    Some(counter.clone())
                } else {
                    Some(-&counter)
                };

                if !flip_flop {
                    counter += 1;
                }

                flip_flop = !flip_flop;
                result
            } else {
                None
            }
        })
    }
}