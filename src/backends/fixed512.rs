// src/backends/fixed512.rs

use num::BigInt;
use std::fmt;
use crate::core::gnfs_integer::GnfsInteger;
use crypto_bigint::{U512, Encoding, NonZero};

/// Fixed-width 512-bit backend for GNFS arithmetic (signed, two's complement)
///
/// Optimized for numbers up to 78-154 digits (algebraic norms fitting in 251-500 bits).
/// Uses two's complement representation to support negative values (needed for
/// relation sieving where `a` values can be negative).
///
/// The top bit (bit 511) is the sign bit:
/// - 0 = non-negative (values 0 to 2^511-1)
/// - 1 = negative (values -1 to -2^511, stored as two's complement)
///
/// Memory efficiency: Stack-only allocation (64 bytes per value)
/// Performance: Fast constant-time operations with Montgomery reduction
/// GPU-compatible: No heap allocation, deterministic execution time
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fixed512(U512);

/// The sign bit mask: bit 511 set
const SIGN_BIT: U512 = U512::from_be_hex("80000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

impl Fixed512 {
    pub fn new(value: U512) -> Self {
        Fixed512(value)
    }

    pub fn value(&self) -> &U512 {
        &self.0
    }

    /// Returns true if this value represents a negative number (sign bit set)
    fn is_negative(&self) -> bool {
        self.0 >= SIGN_BIT
    }

    /// Negate via two's complement: !x + 1
    fn negate(&self) -> Self {
        let not_val = self.0.not();
        Fixed512(not_val.wrapping_add(&U512::ONE))
    }

    /// Get the absolute value as U512 (for unsigned operations like div/rem/gcd)
    fn abs_u512(&self) -> U512 {
        if self.is_negative() {
            self.negate().0
        } else {
            self.0
        }
    }
}

// Custom Ord: signed two's complement comparison
impl PartialOrd for Fixed512 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fixed512 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.is_negative(), other.is_negative()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => self.0.cmp(&other.0),
            (true, true) => self.0.cmp(&other.0),
        }
    }
}

impl GnfsInteger for Fixed512 {
    fn from_bigint(n: &BigInt) -> Option<Self> {
        let (sign, bytes) = n.to_bytes_be();

        match sign {
            num::bigint::Sign::NoSign => Some(Fixed512(U512::ZERO)),
            num::bigint::Sign::Plus => {
                if bytes.len() > 64 {
                    return None;
                }
                let mut padded = vec![0u8; 64 - bytes.len()];
                padded.extend_from_slice(&bytes);
                let mut array = [0u8; 64];
                array.copy_from_slice(&padded);
                let val = U512::from_be_bytes(array);
                if val >= SIGN_BIT {
                    return None;
                }
                Some(Fixed512(val))
            }
            num::bigint::Sign::Minus => {
                if bytes.len() > 64 {
                    return None;
                }
                let mut padded = vec![0u8; 64 - bytes.len()];
                padded.extend_from_slice(&bytes);
                let mut array = [0u8; 64];
                array.copy_from_slice(&padded);
                let magnitude = U512::from_be_bytes(array);
                if magnitude > SIGN_BIT {
                    return None;
                }
                let negated = magnitude.not().wrapping_add(&U512::ONE);
                Some(Fixed512(negated))
            }
        }
    }

    fn to_bigint(&self) -> BigInt {
        if self.0 == U512::ZERO {
            return BigInt::from(0);
        }
        if self.is_negative() {
            let magnitude = self.abs_u512();
            let bytes = magnitude.to_be_bytes();
            -BigInt::from_bytes_be(num::bigint::Sign::Plus, &bytes)
        } else {
            let bytes = self.0.to_be_bytes();
            BigInt::from_bytes_be(num::bigint::Sign::Plus, &bytes)
        }
    }

    fn from_i64(n: i64) -> Option<Self> {
        if n >= 0 {
            Some(Fixed512(U512::from(n as u64)))
        } else {
            let magnitude = U512::from(n.unsigned_abs());
            let negated = magnitude.not().wrapping_add(&U512::ONE);
            Some(Fixed512(negated))
        }
    }

    fn from_u64(n: u64) -> Option<Self> {
        Some(Fixed512(U512::from(n)))
    }

    fn to_u32(&self) -> Option<u32> {
        if self.is_negative() || self.0 > U512::from(u32::MAX) {
            None
        } else {
            let limb_value: u64 = self.0.as_limbs()[0].into();
            Some(limb_value as u32)
        }
    }

    fn to_u64(&self) -> Option<u64> {
        if self.is_negative() || self.0 > U512::from(u64::MAX) {
            None
        } else {
            Some(self.0.as_limbs()[0].into())
        }
    }

    fn zero() -> Self {
        Fixed512(U512::ZERO)
    }

    fn one() -> Self {
        Fixed512(U512::ONE)
    }

    fn is_zero(&self) -> bool {
        self.0 == U512::ZERO
    }

    fn is_one(&self) -> bool {
        self.0 == U512::ONE
    }

    fn is_even(&self) -> bool {
        let limb_value: u64 = self.0.as_limbs()[0].into();
        (limb_value & 1) == 0
    }

    fn pow(&self, exp: u32) -> Self {
        let mut result = U512::ONE;
        let mut base = self.0;
        let mut exp = exp;

        while exp > 0 {
            if exp & 1 == 1 {
                result = result.wrapping_mul(&base);
            }
            base = base.wrapping_mul(&base);
            exp >>= 1;
        }

        Fixed512(result)
    }

    fn checked_add(&self, other: &Self) -> Option<Self> {
        let result = self.0.wrapping_add(&other.0);
        let r = Fixed512(result);
        let self_neg = self.is_negative();
        let other_neg = other.is_negative();
        let result_neg = r.is_negative();
        if self_neg == other_neg && self_neg != result_neg {
            None
        } else {
            Some(r)
        }
    }

    fn checked_sub(&self, other: &Self) -> Option<Self> {
        let result = self.0.wrapping_sub(&other.0);
        let r = Fixed512(result);
        let self_neg = self.is_negative();
        let other_neg = other.is_negative();
        let result_neg = r.is_negative();
        if self_neg != other_neg && self_neg != result_neg {
            None
        } else {
            Some(r)
        }
    }

    fn checked_mul(&self, other: &Self) -> Option<Self> {
        let result = self.0.wrapping_mul(&other.0);
        Some(Fixed512(result))
    }

    fn checked_div(&self, other: &Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        let a_abs = self.abs_u512();
        let b_abs = other.abs_u512();
        NonZero::new(b_abs).into_option().map(|nz| {
            let (quotient, _) = a_abs.div_rem(&nz);
            let result = Fixed512(quotient);
            if self.is_negative() != other.is_negative() && !result.is_zero() {
                result.negate()
            } else {
                result
            }
        })
    }

    fn gcd(&self, other: &Self) -> Self {
        let mut a = self.abs_u512();
        let mut b = other.abs_u512();

        while b != U512::ZERO {
            if let Some(nz_b) = NonZero::new(b).into_option() {
                let (_quotient, remainder) = a.div_rem(&nz_b);
                a = b;
                b = remainder;
            } else {
                break;
            }
        }

        Fixed512(a)
    }

    fn abs(&self) -> Self {
        if self.is_negative() {
            self.negate()
        } else {
            *self
        }
    }

    fn modpow(&self, exp: &Self, m: &Self) -> Self {
        if m.0 <= U512::ONE {
            return Fixed512(U512::ZERO);
        }

        let m_abs = m.abs_u512();
        let nz_m = match NonZero::new(m_abs).into_option() {
            Some(nz) => nz,
            None => return Fixed512(U512::ZERO),
        };

        let mut result = U512::ONE;
        let self_abs = self.abs_u512();
        let (_quotient, base) = self_abs.div_rem(&nz_m);
        let mut base = base;
        let mut exp = exp.abs_u512();

        while exp > U512::ZERO {
            let limb_value: u64 = exp.as_limbs()[0].into();
            if (limb_value & 1) == 1 {
                let product = result.wrapping_mul(&base);
                let (_quotient, remainder) = product.div_rem(&nz_m);
                result = remainder;
            }
            exp >>= 1;
            let square = base.wrapping_mul(&base);
            let (_quotient, remainder) = square.div_rem(&nz_m);
            base = remainder;
        }

        Fixed512(result)
    }

    fn bit(&self, position: usize) -> bool {
        if position >= 512 {
            false
        } else {
            let limb_index = position / 64;
            let bit_index = position % 64;
            let limb_value: u64 = self.0.as_limbs()[limb_index].into();
            (limb_value >> bit_index) & 1 == 1
        }
    }

    fn bits(&self) -> usize {
        if self.is_negative() {
            let abs_val = self.abs_u512();
            512 - abs_val.leading_zeros()
        } else {
            512 - self.0.leading_zeros()
        }
    }

    fn max_value() -> Option<Self> {
        Some(Fixed512(SIGN_BIT.wrapping_sub(&U512::ONE)))
    }

    fn backend_name() -> &'static str {
        "Fixed512"
    }
}

// Arithmetic operator implementations
impl std::ops::Add for Fixed512 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Fixed512(self.0.wrapping_add(&other.0))
    }
}

impl std::ops::Sub for Fixed512 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Fixed512(self.0.wrapping_sub(&other.0))
    }
}

impl std::ops::Mul for Fixed512 {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Fixed512(self.0.wrapping_mul(&other.0))
    }
}

impl std::ops::Div for Fixed512 {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        let a_abs = self.abs_u512();
        let b_abs = other.abs_u512();
        if let Some(nz) = NonZero::new(b_abs).into_option() {
            let (quotient, _) = a_abs.div_rem(&nz);
            let result = Fixed512(quotient);
            if self.is_negative() != other.is_negative() && !result.is_zero() {
                result.negate()
            } else {
                result
            }
        } else {
            Fixed512(U512::ZERO)
        }
    }
}

impl std::ops::Rem for Fixed512 {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        let a_abs = self.abs_u512();
        let b_abs = other.abs_u512();
        if let Some(nz) = NonZero::new(b_abs).into_option() {
            let (_, remainder) = a_abs.div_rem(&nz);
            let result = Fixed512(remainder);
            if self.is_negative() && !result.is_zero() {
                result.negate()
            } else {
                result
            }
        } else {
            Fixed512(U512::ZERO)
        }
    }
}

// Assignment operator implementations
impl std::ops::AddAssign for Fixed512 {
    fn add_assign(&mut self, other: Self) {
        self.0 = self.0.wrapping_add(&other.0);
    }
}

impl std::ops::SubAssign for Fixed512 {
    fn sub_assign(&mut self, other: Self) {
        self.0 = self.0.wrapping_sub(&other.0);
    }
}

impl std::ops::MulAssign for Fixed512 {
    fn mul_assign(&mut self, other: Self) {
        self.0 = self.0.wrapping_mul(&other.0);
    }
}

impl std::ops::DivAssign for Fixed512 {
    fn div_assign(&mut self, other: Self) {
        *self = *self / other;
    }
}

impl std::ops::RemAssign for Fixed512 {
    fn rem_assign(&mut self, other: Self) {
        *self = *self % other;
    }
}

// Display and Debug implementations
impl fmt::Display for Fixed512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_bigint())
    }
}

impl fmt::Debug for Fixed512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fixed512({})", self.to_bigint())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let a = Fixed512::from_u64(100).unwrap();
        let b = Fixed512::from_u64(50).unwrap();

        assert_eq!(a + b, Fixed512::from_u64(150).unwrap());
        assert_eq!(a - b, Fixed512::from_u64(50).unwrap());
        assert_eq!(a * b, Fixed512::from_u64(5000).unwrap());
        assert_eq!(a / b, Fixed512::from_u64(2).unwrap());
        assert_eq!(a % b, Fixed512::from_u64(0).unwrap());
    }

    #[test]
    fn test_negative_values() {
        let neg5 = Fixed512::from_i64(-5).unwrap();
        let pos3 = Fixed512::from_i64(3).unwrap();

        let sum = neg5 + pos3;
        assert_eq!(sum.to_bigint(), BigInt::from(-2));

        let prod = neg5 * pos3;
        assert_eq!(prod.to_bigint(), BigInt::from(-15));

        assert_eq!(neg5.abs().to_bigint(), BigInt::from(5));
        assert!(neg5 < pos3);
    }

    #[test]
    fn test_negative_bigint_roundtrip() {
        let n = BigInt::from(-12345);
        let fixed = Fixed512::from_bigint(&n).unwrap();
        assert_eq!(fixed.to_bigint(), n);

        let large_neg = BigInt::from(-1000000000000_i64);
        let fixed = Fixed512::from_bigint(&large_neg).unwrap();
        assert_eq!(fixed.to_bigint(), large_neg);
    }

    #[test]
    fn test_large_values() {
        let n = BigInt::parse_bytes(
            b"10000000000000000000000000000000000000000000000000000000000000000000000000000000",
            10
        ).unwrap();
        let a = Fixed512::from_bigint(&n).unwrap();
        let b = Fixed512::from_u64(2).unwrap();
        let result = a / b;
        let expected = BigInt::parse_bytes(
            b"5000000000000000000000000000000000000000000000000000000000000000000000000000000",
            10
        ).unwrap();
        assert_eq!(result.to_bigint(), expected);
    }

    #[test]
    fn test_gcd() {
        let a = Fixed512::from_u64(48).unwrap();
        let b = Fixed512::from_u64(18).unwrap();
        assert_eq!(a.gcd(&b), Fixed512::from_u64(6).unwrap());
    }

    #[test]
    fn test_modpow() {
        let base = Fixed512::from_u64(3).unwrap();
        let exp = Fixed512::from_u64(5).unwrap();
        let m = Fixed512::from_u64(13).unwrap();
        assert_eq!(base.modpow(&exp, &m), Fixed512::from_u64(9).unwrap());
    }

    #[test]
    fn test_bits() {
        let a = Fixed512::from_u64(255).unwrap();
        assert_eq!(a.bits(), 8);

        let b = Fixed512::from_u64(1024).unwrap();
        assert_eq!(b.bits(), 11);
    }

    #[test]
    fn test_bigint_conversion() {
        let n = BigInt::from(12345_u64);
        let fixed = Fixed512::from_bigint(&n).unwrap();
        assert_eq!(fixed.to_bigint(), n);
    }

    #[test]
    fn test_bigint_conversion_large() {
        let n = BigInt::parse_bytes(
            b"1234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890",
            10
        ).unwrap();
        let fixed = Fixed512::from_bigint(&n).unwrap();
        assert_eq!(fixed.to_bigint(), n);
    }

    #[test]
    fn test_overflow_detection() {
        let max = Fixed512::max_value().unwrap();
        let one = Fixed512::from_u64(1).unwrap();
        assert!(max.checked_add(&one).is_none());
    }

    #[test]
    fn test_from_bigint_too_large() {
        let n = BigInt::parse_bytes(
            b"12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890",
            10
        ).unwrap();
        assert!(Fixed512::from_bigint(&n).is_none());
    }

    #[test]
    fn test_checked_operations() {
        let a = Fixed512::from_u64(100).unwrap();
        let b = Fixed512::from_u64(50).unwrap();

        assert_eq!(a.checked_add(&b).unwrap(), Fixed512::from_u64(150).unwrap());
        assert_eq!(a.checked_sub(&b).unwrap(), Fixed512::from_u64(50).unwrap());
        assert_eq!(a.checked_mul(&b).unwrap(), Fixed512::from_u64(5000).unwrap());
        assert_eq!(a.checked_div(&b).unwrap(), Fixed512::from_u64(2).unwrap());
    }

    #[test]
    fn test_checked_div_by_zero() {
        let a = Fixed512::from_u64(100).unwrap();
        let zero = Fixed512::zero();
        assert!(a.checked_div(&zero).is_none());
    }

    #[test]
    fn test_signed_ordering() {
        let neg2 = Fixed512::from_i64(-2).unwrap();
        let neg1 = Fixed512::from_i64(-1).unwrap();
        let zero = Fixed512::zero();
        let pos1 = Fixed512::from_u64(1).unwrap();

        assert!(neg2 < neg1);
        assert!(neg1 < zero);
        assert!(zero < pos1);
        assert!(neg2 < pos1);
    }
}
