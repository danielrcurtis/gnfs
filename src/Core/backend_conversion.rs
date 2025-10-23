// src/core/backend_conversion.rs

use num::BigInt;
use crate::core::gnfs_integer::GnfsInteger;

/// Utilities for converting between different GnfsInteger backends
/// and handling BigInt I/O for display and persistence

/// Convert from BigInt to a specific GnfsInteger backend
/// Returns None if the value exceeds the backend's capacity
pub fn from_bigint<T: GnfsInteger>(n: &BigInt) -> Option<T> {
    T::from_bigint(n)
}

/// Convert from a GnfsInteger backend to BigInt
/// This always succeeds as BigInt has arbitrary precision
pub fn to_bigint<T: GnfsInteger>(value: &T) -> BigInt {
    value.to_bigint()
}

/// Convert from i64 to a specific GnfsInteger backend
pub fn from_i64<T: GnfsInteger>(n: i64) -> Option<T> {
    T::from_i64(n)
}

/// Convert from u64 to a specific GnfsInteger backend
pub fn from_u64<T: GnfsInteger>(n: u64) -> Option<T> {
    T::from_u64(n)
}

/// Try to convert a GnfsInteger to u32 (for fast paths)
pub fn to_u32<T: GnfsInteger>(value: &T) -> Option<u32> {
    value.to_u32()
}

/// Try to convert a GnfsInteger to u64 (for fast paths)
pub fn to_u64<T: GnfsInteger>(value: &T) -> Option<u64> {
    value.to_u64()
}

/// Convert a vector of BigInt to a vector of GnfsInteger backend
/// Returns None if any value exceeds the backend's capacity
pub fn vec_from_bigint<T: GnfsInteger>(values: &[BigInt]) -> Option<Vec<T>> {
    values.iter().map(|v| T::from_bigint(v)).collect()
}

/// Convert a vector of GnfsInteger backend to a vector of BigInt
pub fn vec_to_bigint<T: GnfsInteger>(values: &[T]) -> Vec<BigInt> {
    values.iter().map(|v| v.to_bigint()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::bigint_backend::BigIntBackend;
    use crate::backends::native64::Native64;

    #[test]
    fn test_bigint_backend_conversion() {
        let n = BigInt::from(12345);
        let backend: BigIntBackend = from_bigint(&n).unwrap();
        assert_eq!(to_bigint(&backend), n);
    }

    #[test]
    fn test_native64_conversion() {
        let n = BigInt::from(12345_u64);
        let backend: Native64 = from_bigint(&n).unwrap();
        assert_eq!(to_bigint(&backend), n);
    }

    #[test]
    fn test_native64_overflow() {
        let n = BigInt::parse_bytes(b"18446744073709551616", 10).unwrap(); // u64::MAX + 1
        let backend: Option<Native64> = from_bigint(&n);
        assert!(backend.is_none());
    }

    #[test]
    fn test_vec_conversion() {
        let values = vec![BigInt::from(1), BigInt::from(2), BigInt::from(3)];
        let backends: Vec<Native64> = vec_from_bigint(&values).unwrap();
        let recovered = vec_to_bigint(&backends);
        assert_eq!(recovered, values);
    }
}
