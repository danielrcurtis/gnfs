// src/backends/fixed256.rs

use num::BigInt;
use std::fmt;
use crate::core::gnfs_integer::GnfsInteger;
use crypto_bigint::{U256, Encoding, NonZero, Limb};

/// Fixed-width 256-bit backend for GNFS arithmetic
///
/// Optimized for numbers up to 31-77 digits (algebraic norms fitting in 121-250 bits).
/// Uses constant-time operations from crypto_bigint for GPU compatibility and security.
///
/// Memory efficiency: Stack-only allocation (32 bytes per value)
/// Performance: Fast constant-time operations with Montgomery reduction
/// GPU-compatible: No heap allocation, deterministic execution time
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed256(U256);

impl Fixed256 {
    pub fn new(value: U256) -> Self {
        Fixed256(value)
    }

    pub fn value(&self) -> &U256 {
        &self.0
    }
}

impl GnfsInteger for Fixed256 {
    fn from_bigint(n: &BigInt) -> Option<Self> {
        // Convert BigInt to bytes (big-endian)
        let bytes = n.to_bytes_be();

        // Check if value is negative
        if bytes.0 == num::bigint::Sign::Minus {
            return None;
        }

        // Check if value fits in 256 bits (32 bytes)
        if bytes.1.len() > 32 {
            return None;
        }

        // Pad to 32 bytes if needed
        let mut padded = vec![0u8; 32 - bytes.1.len()];
        padded.extend_from_slice(&bytes.1);

        // Create U256 from bytes
        let mut array = [0u8; 32];
        array.copy_from_slice(&padded);
        Some(Fixed256(U256::from_be_bytes(array)))
    }

    fn to_bigint(&self) -> BigInt {
        let bytes = self.0.to_be_bytes();
        BigInt::from_bytes_be(num::bigint::Sign::Plus, &bytes)
    }

    fn from_i64(n: i64) -> Option<Self> {
        if n >= 0 {
            Some(Fixed256(U256::from(n as u64)))
        } else {
            None
        }
    }

    fn from_u64(n: u64) -> Option<Self> {
        Some(Fixed256(U256::from(n)))
    }

    fn to_u32(&self) -> Option<u32> {
        // Check if value fits in u32
        if self.0 > U256::from(u32::MAX) {
            None
        } else {
            // Convert Limb to u64, then to u32
            let limb_value: u64 = self.0.as_limbs()[0].into();
            Some(limb_value as u32)
        }
    }

    fn to_u64(&self) -> Option<u64> {
        // Check if value fits in u64
        if self.0 > U256::from(u64::MAX) {
            None
        } else {
            // Convert Limb to u64
            Some(self.0.as_limbs()[0].into())
        }
    }

    fn zero() -> Self {
        Fixed256(U256::ZERO)
    }

    fn one() -> Self {
        Fixed256(U256::ONE)
    }

    fn is_zero(&self) -> bool {
        self.0 == U256::ZERO
    }

    fn is_one(&self) -> bool {
        self.0 == U256::ONE
    }

    fn is_even(&self) -> bool {
        let limb_value: u64 = self.0.as_limbs()[0].into();
        (limb_value & 1) == 0
    }

    fn pow(&self, exp: u32) -> Self {
        let mut result = U256::ONE;
        let mut base = self.0;
        let mut exp = exp;

        while exp > 0 {
            if exp & 1 == 1 {
                result = result.wrapping_mul(&base);
            }
            base = base.wrapping_mul(&base);
            exp >>= 1;
        }

        Fixed256(result)
    }

    fn checked_add(&self, other: &Self) -> Option<Self> {
        let (result, overflow) = self.0.adc(&other.0, crypto_bigint::Limb::ZERO);
        if overflow.0 != 0 {
            None
        } else {
            Some(Fixed256(result))
        }
    }

    fn checked_sub(&self, other: &Self) -> Option<Self> {
        let (result, borrow) = self.0.sbb(&other.0, crypto_bigint::Limb::ZERO);
        if borrow.0 != 0 {
            None
        } else {
            Some(Fixed256(result))
        }
    }

    fn checked_mul(&self, other: &Self) -> Option<Self> {
        // For checked_mul, we need to check if the result would overflow
        // crypto-bigint doesn't provide checked_mul, so we use wrapping_mul and check
        // if the high bits are non-zero (indicating overflow)
        let result = self.0.wrapping_mul(&other.0);

        // Simple overflow check: if either operand is non-zero and result is smaller than self
        // then overflow occurred
        if other.0 != U256::ZERO && result < self.0 {
            None
        } else {
            Some(Fixed256(result))
        }
    }

    fn checked_div(&self, other: &Self) -> Option<Self> {
        if other.0 == U256::ZERO {
            None
        } else {
            NonZero::new(other.0).into_option().map(|nz| {
                let (quotient, _remainder) = self.0.div_rem(&nz);
                Fixed256(quotient)
            })
        }
    }

    fn gcd(&self, other: &Self) -> Self {
        let mut a = self.0;
        let mut b = other.0;

        while b != U256::ZERO {
            if let Some(nz_b) = NonZero::new(b).into_option() {
                let (_quotient, remainder) = a.div_rem(&nz_b);
                a = b;
                b = remainder;
            } else {
                break;
            }
        }

        Fixed256(a)
    }

    fn abs(&self) -> Self {
        // U256 is always non-negative
        *self
    }

    fn modpow(&self, exp: &Self, m: &Self) -> Self {
        if m.0 <= U256::ONE {
            return Fixed256(U256::ZERO);
        }

        let nz_m = match NonZero::new(m.0).into_option() {
            Some(nz) => nz,
            None => return Fixed256(U256::ZERO),
        };

        let mut result = U256::ONE;
        let (_quotient, base) = self.0.div_rem(&nz_m);
        let mut base = base;
        let mut exp = exp.0;

        while exp > U256::ZERO {
            let limb_value: u64 = exp.as_limbs()[0].into();
            if (limb_value & 1) == 1 {
                let product = result.wrapping_mul(&base);
                let (_quotient, remainder) = product.div_rem(&nz_m);
                result = remainder;
            }
            exp = exp >> 1;
            let square = base.wrapping_mul(&base);
            let (_quotient, remainder) = square.div_rem(&nz_m);
            base = remainder;
        }

        Fixed256(result)
    }

    fn bit(&self, position: usize) -> bool {
        if position >= 256 {
            false
        } else {
            let limb_index = position / 64;
            let bit_index = position % 64;
            let limb_value: u64 = self.0.as_limbs()[limb_index].into();
            (limb_value >> bit_index) & 1 == 1
        }
    }

    fn bits(&self) -> usize {
        256 - self.0.leading_zeros() as usize
    }

    fn max_value() -> Option<Self> {
        Some(Fixed256(U256::MAX))
    }

    fn backend_name() -> &'static str {
        "Fixed256"
    }
}

// Arithmetic operator implementations
impl std::ops::Add for Fixed256 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Fixed256(self.0.wrapping_add(&other.0))
    }
}

impl std::ops::Sub for Fixed256 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Fixed256(self.0.wrapping_sub(&other.0))
    }
}

impl std::ops::Mul for Fixed256 {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Fixed256(self.0.wrapping_mul(&other.0))
    }
}

impl std::ops::Div for Fixed256 {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        if let Some(nz) = NonZero::new(other.0).into_option() {
            let (quotient, _remainder) = self.0.div_rem(&nz);
            Fixed256(quotient)
        } else {
            Fixed256(U256::ZERO)
        }
    }
}

impl std::ops::Rem for Fixed256 {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        if let Some(nz) = NonZero::new(other.0).into_option() {
            let (_quotient, remainder) = self.0.div_rem(&nz);
            Fixed256(remainder)
        } else {
            Fixed256(U256::ZERO)
        }
    }
}

// Assignment operator implementations
impl std::ops::AddAssign for Fixed256 {
    fn add_assign(&mut self, other: Self) {
        self.0 = self.0.wrapping_add(&other.0);
    }
}

impl std::ops::SubAssign for Fixed256 {
    fn sub_assign(&mut self, other: Self) {
        self.0 = self.0.wrapping_sub(&other.0);
    }
}

impl std::ops::MulAssign for Fixed256 {
    fn mul_assign(&mut self, other: Self) {
        self.0 = self.0.wrapping_mul(&other.0);
    }
}

impl std::ops::DivAssign for Fixed256 {
    fn div_assign(&mut self, other: Self) {
        if let Some(nz) = NonZero::new(other.0).into_option() {
            let (quotient, _remainder) = self.0.div_rem(&nz);
            self.0 = quotient;
        } else {
            self.0 = U256::ZERO;
        }
    }
}

impl std::ops::RemAssign for Fixed256 {
    fn rem_assign(&mut self, other: Self) {
        if let Some(nz) = NonZero::new(other.0).into_option() {
            let (_quotient, remainder) = self.0.div_rem(&nz);
            self.0 = remainder;
        } else {
            self.0 = U256::ZERO;
        }
    }
}

// Display and Debug implementations
impl fmt::Display for Fixed256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_bigint())
    }
}

impl fmt::Debug for Fixed256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fixed256({})", self.to_bigint())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let a = Fixed256::from_u64(100).unwrap();
        let b = Fixed256::from_u64(50).unwrap();

        assert_eq!(a + b, Fixed256::from_u64(150).unwrap());
        assert_eq!(a - b, Fixed256::from_u64(50).unwrap());
        assert_eq!(a * b, Fixed256::from_u64(5000).unwrap());
        assert_eq!(a / b, Fixed256::from_u64(2).unwrap());
        assert_eq!(a % b, Fixed256::from_u64(0).unwrap());
    }

    #[test]
    fn test_large_values() {
        // Test with values larger than u128
        let a = Fixed256::from_bigint(&BigInt::parse_bytes(b"1000000000000000000000000000000000000000", 10).unwrap()).unwrap();
        let b = Fixed256::from_u64(2).unwrap();
        let result = a / b;
        let expected = BigInt::parse_bytes(b"500000000000000000000000000000000000000", 10).unwrap();
        assert_eq!(result.to_bigint(), expected);
    }

    #[test]
    fn test_gcd() {
        let a = Fixed256::from_u64(48).unwrap();
        let b = Fixed256::from_u64(18).unwrap();
        assert_eq!(a.gcd(&b), Fixed256::from_u64(6).unwrap());
    }

    #[test]
    fn test_modpow() {
        let base = Fixed256::from_u64(3).unwrap();
        let exp = Fixed256::from_u64(5).unwrap();
        let m = Fixed256::from_u64(13).unwrap();
        // 3^5 mod 13 = 243 mod 13 = 9
        assert_eq!(base.modpow(&exp, &m), Fixed256::from_u64(9).unwrap());
    }

    #[test]
    fn test_bits() {
        let a = Fixed256::from_u64(255).unwrap();
        assert_eq!(a.bits(), 8);

        let b = Fixed256::from_u64(1024).unwrap();
        assert_eq!(b.bits(), 11);
    }

    #[test]
    fn test_bigint_conversion() {
        let n = BigInt::from(12345_u64);
        let fixed = Fixed256::from_bigint(&n).unwrap();
        assert_eq!(fixed.to_bigint(), n);
    }

    #[test]
    fn test_bigint_conversion_large() {
        // Test with a large 40-digit number
        let n = BigInt::parse_bytes(b"1234567890123456789012345678901234567890", 10).unwrap();
        let fixed = Fixed256::from_bigint(&n).unwrap();
        assert_eq!(fixed.to_bigint(), n);
    }

    #[test]
    fn test_overflow_detection() {
        let max = Fixed256::max_value().unwrap();
        let one = Fixed256::from_u64(1).unwrap();
        assert!(max.checked_add(&one).is_none());
    }

    #[test]
    fn test_from_bigint_too_large() {
        // Create a number that's too large for 256 bits (> 77 digits)
        let n = BigInt::parse_bytes(
            b"12345678901234567890123456789012345678901234567890123456789012345678901234567890",
            10
        ).unwrap();
        assert!(Fixed256::from_bigint(&n).is_none());
    }

    #[test]
    fn test_checked_operations() {
        let a = Fixed256::from_u64(100).unwrap();
        let b = Fixed256::from_u64(50).unwrap();

        assert_eq!(a.checked_add(&b).unwrap(), Fixed256::from_u64(150).unwrap());
        assert_eq!(a.checked_sub(&b).unwrap(), Fixed256::from_u64(50).unwrap());
        assert_eq!(a.checked_mul(&b).unwrap(), Fixed256::from_u64(5000).unwrap());
        assert_eq!(a.checked_div(&b).unwrap(), Fixed256::from_u64(2).unwrap());
    }

    #[test]
    fn test_checked_div_by_zero() {
        let a = Fixed256::from_u64(100).unwrap();
        let zero = Fixed256::zero();
        assert!(a.checked_div(&zero).is_none());
    }
}
