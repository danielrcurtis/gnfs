// src/algorithms/trial_division.rs
//
// Trial Division: The simplest factorization algorithm
// Complexity: O(sqrt(n))
// Best for: Numbers < 20 digits (< 10^20)
// Typical performance: 10-digit numbers in < 1ms

use num::{BigInt, Integer, One, ToPrimitive, Zero};
use log::debug;

/// Attempts to factor n using trial division up to sqrt(n) or the specified limit.
///
/// Returns Some((p, q)) where p * q = n and p <= q, or None if n is prime
/// or no factors found within the limit.
///
/// # Arguments
/// * `n` - The number to factor (must be > 1)
/// * `limit` - Optional upper bound for trial division. If None, uses sqrt(n)
///
/// # Examples
/// ```
/// use num::BigInt;
/// use gnfs::algorithms::trial_division::trial_division;
///
/// let n = BigInt::from(143);
/// let result = trial_division(&n, None);
/// assert_eq!(result, Some((BigInt::from(11), BigInt::from(13))));
/// ```
pub fn trial_division(n: &BigInt, limit: Option<u64>) -> Option<(BigInt, BigInt)> {
    // Handle trivial cases
    if n <= &BigInt::one() {
        return None;
    }

    // Check if n is even
    if n.is_even() {
        let two = BigInt::from(2);
        let quotient = n / &two;
        return Some((two, quotient));
    }

    // Determine upper bound for trial division
    let sqrt_n = n.sqrt();
    let upper_bound = if let Some(lim) = limit {
        BigInt::from(lim).min(sqrt_n.clone())
    } else {
        sqrt_n.clone()
    };

    // Convert to u64 for faster iteration if possible
    let upper_bound_u64 = upper_bound.to_u64();

    // Try odd divisors starting from 3
    if let Some(bound) = upper_bound_u64 {
        // Fast path: iterate using u64 arithmetic
        debug!("Trial division: checking divisors up to {}", bound);

        let mut divisor = 3u64;
        while divisor <= bound {
            let divisor_bigint = BigInt::from(divisor);
            if n.is_multiple_of(&divisor_bigint) {
                let quotient = n / &divisor_bigint;
                debug!("Found factor: {} × {} = {}", divisor_bigint, quotient, n);
                return Some((divisor_bigint, quotient));
            }
            divisor += 2;
        }
    } else {
        // Slow path: BigInt arithmetic
        debug!("Trial division: checking divisors up to {} (BigInt)", upper_bound);

        let mut divisor = BigInt::from(3);
        let two = BigInt::from(2);

        while &divisor <= &upper_bound {
            if n.is_multiple_of(&divisor) {
                let quotient = n / &divisor;
                debug!("Found factor: {} × {} = {}", divisor, quotient, n);
                return Some((divisor, quotient));
            }
            divisor += &two;
        }
    }

    // No factors found - n is prime
    debug!("No factors found - {} is prime", n);
    None
}

/// Attempts to completely factor n using trial division.
/// Returns a vector of all prime factors (with multiplicity).
///
/// This is useful for small numbers where complete factorization is feasible.
///
/// # Arguments
/// * `n` - The number to factor
/// * `limit` - Optional upper bound for trial division
///
/// # Returns
/// A vector of prime factors in ascending order, or None if unable to completely factor
pub fn complete_factorization(n: &BigInt, limit: Option<u64>) -> Option<Vec<BigInt>> {
    if n <= &BigInt::one() {
        return None;
    }

    let mut factors = Vec::new();
    let mut remaining = n.clone();

    // Factor out all 2s
    let two = BigInt::from(2);
    while remaining.is_even() {
        factors.push(two.clone());
        remaining /= &two;
    }

    // Determine upper bound
    let upper_bound = if let Some(lim) = limit {
        BigInt::from(lim)
    } else {
        remaining.sqrt()
    };

    // Try odd divisors
    let mut divisor = BigInt::from(3);
    let step = BigInt::from(2);

    while &divisor <= &upper_bound && &remaining > &BigInt::one() {
        while remaining.is_multiple_of(&divisor) {
            factors.push(divisor.clone());
            remaining /= &divisor;
        }
        divisor += &step;
    }

    // If remaining > 1, it's a prime factor
    if remaining > BigInt::one() {
        factors.push(remaining);
    }

    Some(factors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trial_division_small_composite() {
        let n = BigInt::from(143); // 11 × 13
        let result = trial_division(&n, None);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
        assert_eq!(p, BigInt::from(11));
        assert_eq!(q, BigInt::from(13));
    }

    #[test]
    fn test_trial_division_even_number() {
        let n = BigInt::from(100); // 2 × 50
        let result = trial_division(&n, None);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
        assert_eq!(p, BigInt::from(2));
        assert_eq!(q, BigInt::from(50));
    }

    #[test]
    fn test_trial_division_prime() {
        let n = BigInt::from(97); // prime
        let result = trial_division(&n, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_trial_division_with_limit() {
        let n = BigInt::from(143); // 11 × 13
        let result = trial_division(&n, Some(20)); // limit includes 11
        assert!(result.is_some());

        let result2 = trial_division(&n, Some(5)); // limit excludes both factors
        assert!(result2.is_none());
    }

    #[test]
    fn test_complete_factorization() {
        let n = BigInt::from(60); // 2² × 3 × 5
        let factors = complete_factorization(&n, None).unwrap();
        assert_eq!(factors, vec![
            BigInt::from(2),
            BigInt::from(2),
            BigInt::from(3),
            BigInt::from(5),
        ]);
    }

    #[test]
    fn test_complete_factorization_prime() {
        let n = BigInt::from(97);
        let factors = complete_factorization(&n, None).unwrap();
        assert_eq!(factors, vec![BigInt::from(97)]);
    }

    #[test]
    fn test_complete_factorization_power_of_two() {
        let n = BigInt::from(64); // 2^6
        let factors = complete_factorization(&n, None).unwrap();
        assert_eq!(factors.len(), 6);
        assert!(factors.iter().all(|f| f == &BigInt::from(2)));
    }
}
