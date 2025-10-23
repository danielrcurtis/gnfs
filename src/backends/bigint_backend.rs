// src/backends/bigint_backend.rs

use num::{BigInt, Integer, ToPrimitive, One, Zero};
use std::fmt;
use crate::core::gnfs_integer::GnfsInteger;

/// BigInt backend for GNFS arithmetic (arbitrary precision)
///
/// Used for numbers 155+ digits where algebraic norms exceed 500 bits.
/// Provides arbitrary precision at the cost of heap allocation and slower performance.
///
/// This is a wrapper around num::BigInt that implements the GnfsInteger trait,
/// allowing the existing BigInt-based code to work seamlessly with the new
/// adaptive backend system.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BigIntBackend(BigInt);

impl BigIntBackend {
    pub fn new(value: BigInt) -> Self {
        BigIntBackend(value)
    }

    pub fn value(&self) -> &BigInt {
        &self.0
    }

    pub fn into_inner(self) -> BigInt {
        self.0
    }
}

impl GnfsInteger for BigIntBackend {
    fn from_bigint(n: &BigInt) -> Option<Self> {
        Some(BigIntBackend(n.clone()))
    }

    fn to_bigint(&self) -> BigInt {
        self.0.clone()
    }

    fn from_i64(n: i64) -> Option<Self> {
        Some(BigIntBackend(BigInt::from(n)))
    }

    fn from_u64(n: u64) -> Option<Self> {
        Some(BigIntBackend(BigInt::from(n)))
    }

    fn to_u32(&self) -> Option<u32> {
        self.0.to_u32()
    }

    fn to_u64(&self) -> Option<u64> {
        self.0.to_u64()
    }

    fn zero() -> Self {
        BigIntBackend(BigInt::zero())
    }

    fn one() -> Self {
        BigIntBackend(BigInt::one())
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    fn is_one(&self) -> bool {
        self.0.is_one()
    }

    fn is_even(&self) -> bool {
        self.0.is_even()
    }

    fn pow(&self, exp: u32) -> Self {
        use num::pow::Pow;
        BigIntBackend(self.0.clone().pow(exp))
    }

    fn checked_add(&self, other: &Self) -> Option<Self> {
        Some(BigIntBackend(&self.0 + &other.0))
    }

    fn checked_sub(&self, other: &Self) -> Option<Self> {
        Some(BigIntBackend(&self.0 - &other.0))
    }

    fn checked_mul(&self, other: &Self) -> Option<Self> {
        Some(BigIntBackend(&self.0 * &other.0))
    }

    fn checked_div(&self, other: &Self) -> Option<Self> {
        if other.0.is_zero() {
            None
        } else {
            Some(BigIntBackend(&self.0 / &other.0))
        }
    }

    fn gcd(&self, other: &Self) -> Self {
        BigIntBackend(self.0.gcd(&other.0))
    }

    fn abs(&self) -> Self {
        use num::Signed;
        BigIntBackend(self.0.abs())
    }

    fn modpow(&self, exp: &Self, m: &Self) -> Self {
        if m.0 <= BigInt::one() {
            return BigIntBackend(BigInt::zero());
        }

        // Convert to BigUint for modpow (only works with non-negative numbers)
        let base = self.0.to_biguint().unwrap();
        let exp = exp.0.to_biguint().unwrap();
        let m = m.0.to_biguint().unwrap();

        let result = base.modpow(&exp, &m);
        BigIntBackend(BigInt::from(result))
    }

    fn bit(&self, position: usize) -> bool {
        if let Some(uint) = self.0.to_biguint() {
            // BigUint doesn't have a bit() method, so we shift and check LSB
            let shifted = &uint >> position;
            (&shifted & num::BigUint::one()) == num::BigUint::one()
        } else {
            false
        }
    }

    fn bits(&self) -> usize {
        self.0.bits() as usize
    }

    fn max_value() -> Option<Self> {
        None // Arbitrary precision has no max value
    }

    fn backend_name() -> &'static str {
        "BigInt"
    }
}

// Arithmetic operator implementations
impl std::ops::Add for BigIntBackend {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        BigIntBackend(self.0 + other.0)
    }
}

impl std::ops::Sub for BigIntBackend {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        BigIntBackend(self.0 - other.0)
    }
}

impl std::ops::Mul for BigIntBackend {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        BigIntBackend(self.0 * other.0)
    }
}

impl std::ops::Div for BigIntBackend {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        BigIntBackend(self.0 / other.0)
    }
}

impl std::ops::Rem for BigIntBackend {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        BigIntBackend(self.0 % other.0)
    }
}

// Assignment operator implementations
impl std::ops::AddAssign for BigIntBackend {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl std::ops::SubAssign for BigIntBackend {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl std::ops::MulAssign for BigIntBackend {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
    }
}

impl std::ops::DivAssign for BigIntBackend {
    fn div_assign(&mut self, other: Self) {
        self.0 /= other.0;
    }
}

impl std::ops::RemAssign for BigIntBackend {
    fn rem_assign(&mut self, other: Self) {
        self.0 %= other.0;
    }
}

// Display and Debug implementations
impl fmt::Display for BigIntBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for BigIntBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BigIntBackend({})", self.0)
    }
}

// Conversion from BigInt
impl From<BigInt> for BigIntBackend {
    fn from(n: BigInt) -> Self {
        BigIntBackend(n)
    }
}

// Conversion to BigInt
impl From<BigIntBackend> for BigInt {
    fn from(b: BigIntBackend) -> Self {
        b.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let a = BigIntBackend::from_u64(100).unwrap();
        let b = BigIntBackend::from_u64(50).unwrap();

        assert_eq!(a.clone() + b.clone(), BigIntBackend::from_u64(150).unwrap());
        assert_eq!(a.clone() - b.clone(), BigIntBackend::from_u64(50).unwrap());
        assert_eq!(a.clone() * b.clone(), BigIntBackend::from_u64(5000).unwrap());
        assert_eq!(a.clone() / b.clone(), BigIntBackend::from_u64(2).unwrap());
        assert_eq!(a % b, BigIntBackend::from_u64(0).unwrap());
    }

    #[test]
    fn test_large_values() {
        let large = BigInt::parse_bytes(b"123456789012345678901234567890", 10).unwrap();
        let backend = BigIntBackend::new(large.clone());
        assert_eq!(backend.to_bigint(), large);
    }

    #[test]
    fn test_gcd() {
        let a = BigIntBackend::from_u64(48).unwrap();
        let b = BigIntBackend::from_u64(18).unwrap();
        assert_eq!(a.gcd(&b), BigIntBackend::from_u64(6).unwrap());
    }

    #[test]
    fn test_modpow() {
        let base = BigIntBackend::from_u64(3).unwrap();
        let exp = BigIntBackend::from_u64(5).unwrap();
        let m = BigIntBackend::from_u64(13).unwrap();
        // 3^5 mod 13 = 243 mod 13 = 9
        assert_eq!(base.modpow(&exp, &m), BigIntBackend::from_u64(9).unwrap());
    }

    #[test]
    fn test_bits() {
        let a = BigIntBackend::from_u64(255).unwrap();
        assert_eq!(a.bits(), 8);

        let large = BigInt::parse_bytes(b"123456789012345678901234567890", 10).unwrap();
        let backend = BigIntBackend::new(large);
        assert!(backend.bits() > 96); // Should be around 97 bits
    }

    #[test]
    fn test_no_max_value() {
        assert!(BigIntBackend::max_value().is_none());
    }
}
