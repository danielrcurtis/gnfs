// src/algorithms/mod.rs
//
// Algorithm Dispatcher: Automatic selection of optimal factorization algorithm
//
// This module provides a unified interface for factoring integers using the most
// appropriate algorithm based on the input size. It implements a multi-tier
// factorization pipeline:
//
// Input Size          Algorithm           Expected Time     Status
// ─────────────────────────────────────────────────────────────────────────
// < 20 digits         Trial Division      < 1ms             ✓ Implemented
// 20-40 digits        Pollard's Rho       < 100ms           ✓ Implemented
// 40-100 digits       Quadratic Sieve     < 10s             ✗ Stub only
// 100+ digits         GNFS                minutes-hours     ✓ Existing code
//
// This approach ensures that:
// - Small numbers are factored instantly (no GNFS overhead)
// - Medium numbers use appropriate algorithms (not overkill)
// - Large numbers still benefit from GNFS (when truly necessary)
//
// Usage:
//   let n = BigInt::from(143);
//   let algorithm = choose_algorithm(&n);  // Returns TrialDivision
//   let result = factor(&n)?;              // Returns Some((11, 13))

pub mod trial_division;
pub mod pollard_rho;
pub mod quadratic_sieve;

use num::BigInt;
use log::info;

/// Enumeration of available factorization algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactorizationAlgorithm {
    /// Trial Division: O(sqrt(n))
    /// Best for: < 20 digits (< 10^20)
    TrialDivision,

    /// Pollard's Rho: O(n^(1/4)) expected
    /// Best for: 20-40 digits (10^20 to 10^40)
    PollardRho,

    /// Quadratic Sieve: L_n[1/2, 1]
    /// Best for: 40-100 digits (10^40 to 10^100)
    QuadraticSieve,

    /// General Number Field Sieve: L_n[1/3, (64/9)^(1/3)]
    /// Best for: 100+ digits (> 10^100)
    GNFS,
}

impl FactorizationAlgorithm {
    /// Returns a human-readable name for the algorithm
    pub fn name(&self) -> &str {
        match self {
            Self::TrialDivision => "Trial Division",
            Self::PollardRho => "Pollard's Rho",
            Self::QuadraticSieve => "Quadratic Sieve",
            Self::GNFS => "General Number Field Sieve (GNFS)",
        }
    }

    /// Returns the expected complexity class
    pub fn complexity(&self) -> &str {
        match self {
            Self::TrialDivision => "O(sqrt(n))",
            Self::PollardRho => "O(n^(1/4)) expected",
            Self::QuadraticSieve => "L_n[1/2, 1] ≈ exp(sqrt(ln n ln ln n))",
            Self::GNFS => "L_n[1/3, (64/9)^(1/3)] ≈ exp(1.923 (ln n)^(1/3) (ln ln n)^(2/3))",
        }
    }

    /// Returns the typical use case
    pub fn use_case(&self) -> &str {
        match self {
            Self::TrialDivision => "< 20 digits (< 10^20)",
            Self::PollardRho => "20-40 digits (10^20 to 10^40)",
            Self::QuadraticSieve => "40-100 digits (10^40 to 10^100)",
            Self::GNFS => "100+ digits (> 10^100)",
        }
    }
}

/// Automatically selects the optimal factorization algorithm based on input size.
///
/// The selection is based on empirical performance characteristics:
/// - Trial Division is fastest for small numbers (< 20 digits)
/// - Pollard's Rho is optimal for medium numbers (20-40 digits)
/// - Quadratic Sieve is best for large numbers (40-100 digits)
/// - GNFS is necessary for very large numbers (100+ digits)
///
/// # Arguments
/// * `n` - The number to be factored
///
/// # Returns
/// The recommended algorithm for this input size
///
/// # Examples
/// ```
/// use num::BigInt;
/// use gnfs::algorithms::{choose_algorithm, FactorizationAlgorithm};
///
/// let small = BigInt::from(143);
/// assert_eq!(choose_algorithm(&small), FactorizationAlgorithm::TrialDivision);
///
/// let medium = BigInt::parse_bytes(b"12345678901234567890", 10).unwrap();
/// assert_eq!(choose_algorithm(&medium), FactorizationAlgorithm::PollardRho);
/// ```
pub fn choose_algorithm(n: &BigInt) -> FactorizationAlgorithm {
    let digits = n.to_string().len();

    let algorithm = match digits {
        0..=19 => FactorizationAlgorithm::TrialDivision,
        20..=39 => FactorizationAlgorithm::PollardRho,
        40..=99 => FactorizationAlgorithm::QuadraticSieve,
        _ => FactorizationAlgorithm::GNFS,
    };

    info!("========================================");
    info!("ALGORITHM SELECTION");
    info!("========================================");
    info!("Number size: {} digits", digits);
    info!("Selected algorithm: {}", algorithm.name());
    info!("Complexity: {}", algorithm.complexity());
    info!("Typical use case: {}", algorithm.use_case());
    info!("========================================");
    info!("");

    algorithm
}

/// Attempts to factor n using the automatically-selected optimal algorithm.
///
/// This function tries to find a non-trivial factorization of n by:
/// 1. Selecting the appropriate algorithm based on n's size
/// 2. Attempting factorization with that algorithm
/// 3. For Trial Division and Pollard's Rho, falling back to the next algorithm if needed
/// 4. For Quadratic Sieve and GNFS, returning an error if not applicable
///
/// # Arguments
/// * `n` - The number to factor (must be > 1)
///
/// # Returns
/// * `Ok((p, q))` - A factorization where p * q = n and 1 < p <= q < n
/// * `Err(String)` - An error message if factorization failed or requires GNFS
///
/// # Examples
/// ```
/// use num::BigInt;
/// use gnfs::algorithms::factor;
///
/// let n = BigInt::from(143);
/// let (p, q) = factor(&n).unwrap();
/// assert_eq!(p * q, n);
/// assert_eq!(p, BigInt::from(11));
/// assert_eq!(q, BigInt::from(13));
/// ```
pub fn factor(n: &BigInt) -> Result<(BigInt, BigInt), String> {
    let algorithm = choose_algorithm(n);

    match algorithm {
        FactorizationAlgorithm::TrialDivision => {
            info!("Attempting Trial Division...");
            match trial_division::trial_division(n, None) {
                Some(factors) => {
                    info!("✓ Trial Division succeeded");
                    Ok(factors)
                }
                None => {
                    info!("✗ Trial Division found no factors (n may be prime)");
                    info!("Falling back to Pollard's Rho...");

                    match pollard_rho::pollard_rho(n, 100000) {
                        Some(factors) => {
                            info!("✓ Pollard's Rho succeeded");
                            Ok(factors)
                        }
                        None => {
                            Err(format!("No factors found - {} may be prime", n))
                        }
                    }
                }
            }
        }

        FactorizationAlgorithm::PollardRho => {
            info!("Attempting Pollard's Rho...");
            match pollard_rho::pollard_rho(n, 100000) {
                Some(factors) => {
                    info!("✓ Pollard's Rho succeeded");
                    Ok(factors)
                }
                None => {
                    info!("✗ Pollard's Rho failed after 100000 iterations");
                    Err(format!("Pollard's Rho failed - try increasing iterations or use GNFS"))
                }
            }
        }

        FactorizationAlgorithm::QuadraticSieve => {
            info!("Attempting Quadratic Sieve...");
            match quadratic_sieve::quadratic_sieve(n) {
                Some(factors) => {
                    info!("✓ Quadratic Sieve succeeded");
                    Ok(factors)
                }
                None => {
                    info!("✗ Quadratic Sieve not yet implemented");
                    info!("Falling back to Pollard's Rho (may be slow)...");

                    match pollard_rho::pollard_rho(n, 1000000) {
                        Some(factors) => {
                            info!("✓ Pollard's Rho succeeded (with extended iterations)");
                            Ok(factors)
                        }
                        None => {
                            Err("Quadratic Sieve not implemented, Pollard's Rho failed. Use GNFS for this number size.".to_string())
                        }
                    }
                }
            }
        }

        FactorizationAlgorithm::GNFS => {
            info!("Number requires GNFS (100+ digits)");
            Err("GNFS requires full context - use main.rs GNFS pipeline".to_string())
        }
    }
}

/// Attempts to factor n using a specific algorithm (for testing/benchmarking).
///
/// This bypasses the automatic algorithm selection and directly invokes
/// the requested algorithm. Useful for performance comparisons or when
/// you know the optimal algorithm for your use case.
///
/// # Arguments
/// * `n` - The number to factor
/// * `algorithm` - The specific algorithm to use
///
/// # Returns
/// * `Ok((p, q))` - A factorization where p * q = n
/// * `Err(String)` - An error message if factorization failed
pub fn factor_with(n: &BigInt, algorithm: FactorizationAlgorithm) -> Result<(BigInt, BigInt), String> {
    info!("Using forced algorithm: {}", algorithm.name());

    match algorithm {
        FactorizationAlgorithm::TrialDivision => {
            trial_division::trial_division(n, None)
                .ok_or_else(|| "Trial division failed".to_string())
        }

        FactorizationAlgorithm::PollardRho => {
            pollard_rho::pollard_rho(n, 100000)
                .ok_or_else(|| "Pollard's rho failed".to_string())
        }

        FactorizationAlgorithm::QuadraticSieve => {
            quadratic_sieve::quadratic_sieve(n)
                .ok_or_else(|| "Quadratic sieve not yet implemented".to_string())
        }

        FactorizationAlgorithm::GNFS => {
            Err("GNFS requires full context - use main.rs".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_algorithm_small() {
        let n = BigInt::from(143);
        assert_eq!(choose_algorithm(&n), FactorizationAlgorithm::TrialDivision);
    }

    #[test]
    fn test_choose_algorithm_medium() {
        // 20-digit number
        let n = BigInt::parse_bytes(b"12345678901234567890", 10).unwrap();
        assert_eq!(choose_algorithm(&n), FactorizationAlgorithm::PollardRho);
    }

    #[test]
    fn test_choose_algorithm_large() {
        // 50-digit number
        let n = BigInt::parse_bytes(b"12345678901234567890123456789012345678901234567890", 10).unwrap();
        assert_eq!(choose_algorithm(&n), FactorizationAlgorithm::QuadraticSieve);
    }

    #[test]
    fn test_choose_algorithm_very_large() {
        // 100-digit number
        let n = BigInt::parse_bytes(
            b"1234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890",
            10
        ).unwrap();
        assert_eq!(choose_algorithm(&n), FactorizationAlgorithm::GNFS);
    }

    #[test]
    fn test_factor_small() {
        let n = BigInt::from(143);
        let result = factor(&n);
        assert!(result.is_ok());
        let (p, q) = result.unwrap();
        assert_eq!(p * q, n);
    }

    #[test]
    fn test_factor_with_trial_division() {
        let n = BigInt::from(8051);
        let result = factor_with(&n, FactorizationAlgorithm::TrialDivision);
        assert!(result.is_ok());
        let (p, q) = result.unwrap();
        assert_eq!(p * q, n);
    }

    #[test]
    fn test_factor_with_pollard_rho() {
        let n = BigInt::from(8051);
        let result = factor_with(&n, FactorizationAlgorithm::PollardRho);
        assert!(result.is_ok());
        let (p, q) = result.unwrap();
        assert_eq!(p * q, n);
    }
}
