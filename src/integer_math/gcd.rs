// src/integer_math/gcd.rs

use num::BigInt;
use num::Signed;
use num::Integer;

pub struct GCD;

impl GCD {
    pub fn find_lcm(numbers: &[BigInt]) -> BigInt {
        numbers.iter().fold(BigInt::from(1), |acc, x| Self::find_lcm_pair(&acc, x))
    }

    pub fn find_lcm_pair(left: &BigInt, right: &BigInt) -> BigInt {
        let abs_value1 = left.abs();
        let abs_value2 = right.abs();
        &(&abs_value1 * &abs_value2) / Self::find_gcd_pair(&abs_value1, &abs_value2)
    }

    pub fn find_gcd(numbers: &[BigInt]) -> BigInt {
        numbers.iter().fold(BigInt::from(0), |acc, x| Self::find_gcd_pair(&acc, x))
    }

    pub fn find_gcd_pair(left: &BigInt, right: &BigInt) -> BigInt {
        left.gcd(right)
    }

    pub fn are_coprime(numbers: &[BigInt]) -> bool {
        Self::find_gcd(numbers) == BigInt::from(1)
    }
}