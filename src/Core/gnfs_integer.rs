// src/core/gnfs_integer.rs

use num::BigInt;
use std::ops::{Add, Sub, Mul, Div, Rem, AddAssign, SubAssign, MulAssign, DivAssign, RemAssign};
use std::cmp::Ord;
use std::fmt::{Debug, Display};

/// Core trait for GNFS integer arithmetic
///
/// This trait abstracts over different integer representations, enabling
/// the GNFS algorithm to adaptively select the most efficient backend
/// based on the size of the numbers being factored.
///
/// Implementations:
/// - Native64: u64 for numbers up to 11-14 digits
/// - Native128: u128 for numbers up to 15-30 digits
/// - Fixed256: crypto_bigint::U256 for numbers up to 31-77 digits
/// - Fixed512: crypto_bigint::U512 for numbers up to 78-154 digits
/// - Arbitrary: num::BigInt for numbers 155+ digits
pub trait GnfsInteger:
    Clone +
    Debug +
    Display +
    Eq +
    Ord +
    Add<Output = Self> +
    Sub<Output = Self> +
    Mul<Output = Self> +
    Div<Output = Self> +
    Rem<Output = Self> +
    AddAssign +
    SubAssign +
    MulAssign +
    DivAssign +
    RemAssign +
    Sized +
    Send +
    Sync
{
    /// Create from BigInt, returning None if value exceeds backend capacity
    fn from_bigint(n: &BigInt) -> Option<Self>;

    /// Convert to BigInt (always succeeds)
    fn to_bigint(&self) -> BigInt;

    /// Create from i64
    fn from_i64(n: i64) -> Option<Self>;

    /// Create from u64
    fn from_u64(n: u64) -> Option<Self>;

    /// Try to convert to u32 (for optimization fast paths)
    fn to_u32(&self) -> Option<u32>;

    /// Try to convert to u64 (for optimization fast paths)
    fn to_u64(&self) -> Option<u64>;

    /// Create zero value
    fn zero() -> Self;

    /// Create one value
    fn one() -> Self;

    /// Check if value is zero
    fn is_zero(&self) -> bool;

    /// Check if value is one
    fn is_one(&self) -> bool;

    /// Check if value is even
    fn is_even(&self) -> bool;

    /// Compute self^exp
    fn pow(&self, exp: u32) -> Self;

    /// Checked addition (returns None on overflow)
    fn checked_add(&self, other: &Self) -> Option<Self>;

    /// Checked subtraction (returns None on underflow)
    fn checked_sub(&self, other: &Self) -> Option<Self>;

    /// Checked multiplication (returns None on overflow)
    fn checked_mul(&self, other: &Self) -> Option<Self>;

    /// Checked division (returns None if divisor is zero)
    fn checked_div(&self, other: &Self) -> Option<Self>;

    /// Greatest common divisor
    fn gcd(&self, other: &Self) -> Self;

    /// Absolute value
    fn abs(&self) -> Self;

    /// Modular exponentiation: self^exp mod m
    fn modpow(&self, exp: &Self, m: &Self) -> Self;

    /// Get bit at position (0 = LSB)
    fn bit(&self, position: usize) -> bool;

    /// Number of bits required to represent this value
    fn bits(&self) -> usize;

    /// Maximum value representable by this type (None for arbitrary precision)
    fn max_value() -> Option<Self>;

    /// Backend type name for debugging/logging
    fn backend_name() -> &'static str;
}

/// Estimate the number of bits required for algebraic norms
/// Formula: bits(N) / degree + 40
pub fn estimate_algebraic_norm_bits(n: &BigInt, degree: usize) -> usize {
    let n_bits = n.bits() as usize;
    n_bits / degree + 40
}

/// Backend type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Native64Signed,
    Native128Signed,
    Fixed256,
    Fixed512,
    Arbitrary,
}

impl BackendType {
    pub fn name(&self) -> &'static str {
        match self {
            BackendType::Native64Signed => "Native64Signed",
            BackendType::Native128Signed => "Native128Signed",
            BackendType::Fixed256 => "Fixed256",
            BackendType::Fixed512 => "Fixed512",
            BackendType::Arbitrary => "Arbitrary (BigInt)",
        }
    }
}

/// Select the appropriate backend based on input number size and polynomial degree
pub fn select_backend(n: &BigInt, degree: usize) -> BackendType {
    let digit_count = n.to_string().len();
    let norm_bits = estimate_algebraic_norm_bits(n, degree);

    // Be conservative: use the more restrictive of digit count or norm bits
    // i64 max is ~9.2e18 (19 digits), but we need headroom for intermediate calculations
    // Safe ranges (with 2x safety margin for intermediate calculations):
    // - Native64Signed: 11-13 digits
    // - Native128Signed: 14-19 digits
    // - Fixed256: 20-38 digits
    // - Fixed512: 39-77 digits

    if digit_count <= 13 && norm_bits <= 60 {
        BackendType::Native64Signed
    } else if digit_count <= 19 && norm_bits <= 120 {
        BackendType::Native128Signed
    } else if digit_count <= 38 && norm_bits <= 250 {
        BackendType::Fixed256
    } else if digit_count <= 77 && norm_bits <= 500 {
        BackendType::Fixed512
    } else {
        BackendType::Arbitrary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num::BigInt;

    #[test]
    fn test_backend_selection() {
        // 11 digits: 10^10 = 2^33 bits, degree 3 → 33/3 + 40 = 51 bits → Native64Signed
        let n = BigInt::from(10_000_000_000_u64);
        assert_eq!(select_backend(&n, 3), BackendType::Native64Signed);

        // 20 digits: 10^19 = 2^63 bits, degree 3 → 63/3 + 40 = 61 bits → Native128Signed
        let n = BigInt::parse_bytes(b"10000000000000000000", 10).unwrap();
        assert_eq!(select_backend(&n, 3), BackendType::Native128Signed);

        // 40 digits: 10^39 = 2^130 bits, degree 3 → 130/3 + 40 = 83 bits → Native128Signed
        let n = BigInt::parse_bytes(b"1000000000000000000000000000000000000000", 10).unwrap();
        assert_eq!(select_backend(&n, 3), BackendType::Native128Signed);

        // 80 digits: 10^79 = 2^263 bits, degree 3 → 263/3 + 40 = 128 bits → Fixed256
        let n = BigInt::parse_bytes(b"10000000000000000000000000000000000000000000000000000000000000000000000000000000", 10).unwrap();
        assert_eq!(select_backend(&n, 3), BackendType::Fixed256);
    }

    #[test]
    fn test_norm_bits_estimation() {
        // 11 digits, degree 3
        let n = BigInt::from(10_000_000_000_u64);
        let bits = estimate_algebraic_norm_bits(&n, 3);
        assert!(bits >= 40 && bits <= 60, "Expected 40-60 bits, got {}", bits);

        // 20 digits, degree 4
        let n = BigInt::parse_bytes(b"10000000000000000000", 10).unwrap();
        let bits = estimate_algebraic_norm_bits(&n, 4);
        assert!(bits >= 55 && bits <= 65, "Expected 55-65 bits, got {}", bits);
    }
}
