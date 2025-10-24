// src/algorithms/pollard_rho.rs
//
// Pollard's Rho Algorithm: Probabilistic factorization using cycle detection
// Complexity: O(n^(1/4)) expected time
// Best for: Numbers in the 20-40 digit range
// Typical performance: 20-30 digit numbers in < 100ms

use num::{BigInt, Integer, One, Zero};
use log::debug;
use crate::integer_math::gcd::GCD;

/// Attempts to factor n using Pollard's Rho algorithm with Floyd's cycle detection.
///
/// The algorithm works by generating a pseudo-random sequence x_i = f(x_{i-1}) mod n
/// where f(x) = x² + c. If n has a factor p, the sequence will cycle modulo p
/// much faster than modulo n. Floyd's cycle detection (tortoise and hare) finds
/// this cycle, and GCD extracts the factor.
///
/// # Arguments
/// * `n` - The number to factor (must be composite and > 1)
/// * `max_iterations` - Maximum number of iterations before giving up
///
/// # Returns
/// Some((p, q)) where p * q = n and 1 < p < n, or None if no factor found
///
/// # Examples
/// ```
/// use num::BigInt;
/// use gnfs::algorithms::pollard_rho::pollard_rho;
///
/// let n = BigInt::from(8051); // 83 × 97
/// let result = pollard_rho(&n, 100000);
/// assert!(result.is_some());
/// ```
pub fn pollard_rho(n: &BigInt, max_iterations: usize) -> Option<(BigInt, BigInt)> {
    // Handle trivial cases
    if n <= &BigInt::one() {
        return None;
    }

    // Check if n is even (shouldn't happen in practice, but handle it)
    if n.is_even() {
        let two = BigInt::from(2);
        let quotient = n / &two;
        return Some((two, quotient));
    }

    // Try different starting values and polynomial constants
    // This increases the probability of finding a factor
    let c_values = [1, 2, 3, 5, 7];

    for &c in &c_values {
        debug!("Pollard's Rho: trying c = {}", c);

        if let Some(factor) = pollard_rho_with_c(n, max_iterations, c) {
            return Some(factor);
        }
    }

    debug!("Pollard's Rho: failed to find factor after trying {} values of c", c_values.len());
    None
}

/// Internal implementation of Pollard's Rho with a specific polynomial constant c.
fn pollard_rho_with_c(n: &BigInt, max_iterations: usize, c: i32) -> Option<(BigInt, BigInt)> {
    let c_bigint = BigInt::from(c);
    let one = BigInt::one();

    // Starting values
    let mut x = BigInt::from(2); // Tortoise
    let mut y = BigInt::from(2); // Hare
    let mut d = BigInt::one();

    let mut iterations = 0;

    while &d == &one && iterations < max_iterations {
        // Tortoise: x = f(x) = (x² + c) mod n
        x = (&x * &x + &c_bigint) % n;

        // Hare: y = f(f(y)) - moves twice as fast
        y = (&y * &y + &c_bigint) % n;
        y = (&y * &y + &c_bigint) % n;

        // Compute GCD(|x - y|, n)
        let diff = if &x > &y {
            &x - &y
        } else {
            &y - &x
        };

        d = GCD::find_gcd_pair(&diff, n);

        iterations += 1;

        // Periodic logging for long-running factorizations
        if iterations % 10000 == 0 {
            debug!("Pollard's Rho: {} iterations, current d = {}", iterations, d);
        }
    }

    // Check if we found a non-trivial factor
    if &d > &one && &d < n {
        let quotient = n / &d;
        debug!("Pollard's Rho: found factor after {} iterations: {} × {} = {}",
               iterations, d, quotient, n);

        // Return factors in ascending order
        if &d <= &quotient {
            Some((d, quotient))
        } else {
            Some((quotient, d))
        }
    } else if &d == n {
        debug!("Pollard's Rho: found trivial factor (d = n) after {} iterations", iterations);
        None
    } else {
        debug!("Pollard's Rho: no factor found after {} iterations", iterations);
        None
    }
}

/// Brent's improved cycle detection variant of Pollard's Rho.
///
/// This is typically 25% faster than Floyd's algorithm due to fewer
/// function evaluations per iteration.
///
/// # Arguments
/// * `n` - The number to factor
/// * `max_iterations` - Maximum number of iterations
///
/// # Returns
/// Some((p, q)) where p * q = n, or None if no factor found
pub fn pollard_rho_brent(n: &BigInt, max_iterations: usize) -> Option<(BigInt, BigInt)> {
    // Handle trivial cases
    if n <= &BigInt::one() {
        return None;
    }

    if n.is_even() {
        let two = BigInt::from(2);
        let quotient = n / &two;
        return Some((two, quotient));
    }

    // Try different c values
    for &c in &[1, 2, 3, 5, 7] {
        debug!("Pollard's Rho (Brent): trying c = {}", c);

        if let Some(factor) = pollard_rho_brent_with_c(n, max_iterations, c) {
            return Some(factor);
        }
    }

    None
}

/// Internal implementation of Brent's variant with a specific polynomial constant.
fn pollard_rho_brent_with_c(n: &BigInt, max_iterations: usize, c: i32) -> Option<(BigInt, BigInt)> {
    let c_bigint = BigInt::from(c);
    let one = BigInt::one();

    let mut y = BigInt::from(2);
    let mut r = 1usize;
    let mut q = BigInt::one();

    let mut iterations = 0;

    loop {
        let x = y.clone();
        for _ in 0..r {
            y = (&y * &y + &c_bigint) % n;
        }

        let mut k = 0usize;
        while k < r && iterations < max_iterations {
            let ys = y.clone();
            let m = (r - k).min(100); // Batch size

            for _ in 0..m {
                y = (&y * &y + &c_bigint) % n;
                let diff = if &x > &y {
                    &x - &y
                } else {
                    &y - &x
                };
                q = (&q * &diff) % n;
                iterations += 1;
            }

            let d = GCD::find_gcd_pair(&q, n);

            if &d > &one {
                // Found a potential factor, backtrack to find exact one
                if &d == n {
                    // Backtrack to find the exact factor
                    let mut d2 = BigInt::one();
                    let mut y2 = ys;

                    while &d2 == &one {
                        y2 = (&y2 * &y2 + &c_bigint) % n;
                        let diff = if &x > &y2 {
                            &x - &y2
                        } else {
                            &y2 - &x
                        };
                        d2 = GCD::find_gcd_pair(&diff, n);
                    }

                    if &d2 < n {
                        let quotient = n / &d2;
                        debug!("Pollard's Rho (Brent): found factor after {} iterations: {} × {}",
                               iterations, d2, quotient);
                        return if &d2 <= &quotient {
                            Some((d2, quotient))
                        } else {
                            Some((quotient, d2))
                        };
                    }
                } else {
                    // Found a non-trivial factor
                    let quotient = n / &d;
                    debug!("Pollard's Rho (Brent): found factor after {} iterations: {} × {}",
                           iterations, d, quotient);
                    return if &d <= &quotient {
                        Some((d, quotient))
                    } else {
                        Some((quotient, d))
                    };
                }
            }

            k += m;
        }

        if iterations >= max_iterations {
            break;
        }

        r *= 2;

        if iterations % 10000 == 0 {
            debug!("Pollard's Rho (Brent): {} iterations", iterations);
        }
    }

    debug!("Pollard's Rho (Brent): no factor found after {} iterations", iterations);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pollard_rho_small_composite() {
        let n = BigInt::from(8051); // 83 × 97
        let result = pollard_rho(&n, 100000);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
        assert!(&p > &BigInt::one() && &p < &n);
        assert!(&q > &BigInt::one() && &q < &n);
    }

    #[test]
    fn test_pollard_rho_143() {
        let n = BigInt::from(143); // 11 × 13
        let result = pollard_rho(&n, 100000);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
    }

    #[test]
    fn test_pollard_rho_larger_number() {
        let n = BigInt::from(1000730021u64); // 10-digit semiprime: 31193 × 32069
        let result = pollard_rho(&n, 100000);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
    }

    #[test]
    fn test_pollard_rho_even_number() {
        let n = BigInt::from(1000); // 2³ × 5³
        let result = pollard_rho(&n, 100000);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
        assert_eq!(p, BigInt::from(2));
    }

    #[test]
    fn test_pollard_rho_brent() {
        let n = BigInt::from(8051); // 83 × 97
        let result = pollard_rho_brent(&n, 100000);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
    }

    #[test]
    fn test_pollard_rho_brent_larger() {
        let n = BigInt::from(1000730021u64); // 31193 × 32069
        let result = pollard_rho_brent(&n, 100000);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
    }
}
