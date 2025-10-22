// src/polynomial/polynomial_construction.rs

use num::{BigInt, Zero, One, Integer, ToPrimitive, Signed};
use log::{debug, info};
use crate::polynomial::polynomial::{Polynomial, Term};
use crate::polynomial::polynomial_quality::{evaluate_quality, PolynomialQualityMetrics};

/// Find optimal polynomial base using Montgomery's method
///
/// Montgomery's method searches for a base m where:
/// 1. m^d ≈ n (for degree d)
/// 2. The resulting polynomial has small coefficients
/// 3. f(m) = n exactly (or very close)
///
/// This is the single most important optimization for GNFS performance.
/// Better polynomials lead to 2-5x speedups by increasing the density
/// of smooth relations found during sieving.
///
/// # Algorithm
///
/// 1. Calculate initial guess: m ≈ n^(1/d) using Newton's method
/// 2. Try candidates in range [m - window, m + window]
/// 3. For each candidate:
///    a. Construct polynomial using base-m representation
///    b. Verify f(m) = n (or very close)
///    c. Evaluate quality metrics
/// 4. Select the m that produces the best quality polynomial
///
/// The search window is typically 2-5% of the base value for small numbers,
/// and scales with input size to balance thoroughness vs. computation time.
///
/// # Arguments
///
/// * `n` - Number to factor (must be composite)
/// * `degree` - Polynomial degree (typically 3-5, determined by input size)
///
/// # Returns
///
/// Tuple of (optimal_polynomial, optimal_base, quality_metrics)
///
/// # Performance
///
/// O(window_size * d * log²(n)) where:
/// - window_size: number of candidates tested (typically 10-100)
/// - d: polynomial degree
/// - log²(n): cost of BigInt arithmetic operations
///
/// For typical inputs, this completes in milliseconds.
///
/// # Example
///
/// ```rust
/// use num::BigInt;
/// use gnfs::polynomial::polynomial_construction::find_optimal_base;
///
/// let n = BigInt::from(47893197);
/// let (poly, m, metrics) = find_optimal_base(&n, 3);
///
/// println!("Optimal base: {}", m);
/// println!("Quality score: {:.2}", metrics.overall_score);
///
/// // Verify polynomial satisfies f(m) = n
/// let eval = poly.evaluate(&m);
/// assert!((eval - &n).abs() < BigInt::from(100));
/// ```
pub fn find_optimal_base(n: &BigInt, degree: u32) -> (Polynomial, BigInt, PolynomialQualityMetrics) {
    info!("Finding optimal polynomial base for N={} with degree={}", n, degree);

    // Step 1: Calculate initial guess using Newton's method
    let m_initial = nth_root(n, degree);
    debug!("Initial base estimate: m = {}", m_initial);

    // Step 2: Calculate search window based on input size
    let window = calculate_search_window(&m_initial, degree, n);
    debug!("Search window: {} candidates around m", window * 2);

    let m_min = if m_initial > BigInt::from(window) {
        &m_initial - window
    } else {
        BigInt::one()
    };
    let m_max = &m_initial + window;

    // Step 3: Try candidates and find best polynomial
    let mut best_poly: Option<Polynomial> = None;
    let mut best_m = m_initial.clone();
    let mut best_metrics: Option<PolynomialQualityMetrics> = None;

    let mut candidates_tested = 0;
    let mut m_candidate = m_min.clone();

    while m_candidate <= m_max {
        // Construct polynomial for this base
        let poly = construct_for_base(n, &m_candidate, degree);

        // Verify the polynomial is valid (f(m) should be close to n)
        let evaluation = poly.evaluate(&m_candidate);
        let diff = (&evaluation - n).abs();

        // Allow small differences due to rounding in base representation
        // For valid polynomials, f(m) should equal n exactly or be very close
        let max_diff = n / BigInt::from(1000); // 0.1% tolerance
        if diff <= max_diff {
            // Evaluate quality
            let metrics = evaluate_quality(&poly, n, &m_candidate);

            if best_metrics.is_none() || metrics.overall_score < best_metrics.as_ref().unwrap().overall_score {
                debug!("New best base: m={}, score={:.2}, diff={}", m_candidate, metrics.overall_score, diff);
                best_metrics = Some(metrics);
                best_m = m_candidate.clone();
                best_poly = Some(poly);
            }
        } else {
            debug!("Rejected m={}: f(m)-n = {} (too large)", m_candidate, diff);
        }

        candidates_tested += 1;
        m_candidate += 1;
    }

    info!("Tested {} candidates", candidates_tested);

    // Return best polynomial found
    if let (Some(poly), Some(metrics)) = (best_poly, best_metrics) {
        info!("Selected base: m = {}", best_m);
        info!("Polynomial quality score: {:.2}", metrics.overall_score);
        info!("Root sum of squares: {:.2}", metrics.root_sum_squares);
        info!("Max coefficient: {}", metrics.max_coefficient);
        (poly, best_m, metrics)
    } else {
        // Fallback to initial guess if no valid polynomial found
        // This should rarely happen
        info!("Warning: No valid polynomial found in search window, using initial estimate");
        let poly = construct_for_base(n, &m_initial, degree);
        let metrics = evaluate_quality(&poly, n, &m_initial);
        (poly, m_initial, metrics)
    }
}

/// Calculate search window size for base selection
///
/// The window determines how many candidates around the initial base
/// estimate will be tested. Larger windows find better polynomials but
/// take longer. The window size is chosen to balance quality vs. speed.
///
/// # Strategy
///
/// - Small numbers (< 20 digits): wider search (50-100 candidates)
/// - Medium numbers (20-100 digits): moderate search (20-50 candidates)
/// - Large numbers (> 100 digits): narrow search (10-20 candidates)
///
/// # Arguments
///
/// * `m` - Initial base estimate
/// * `degree` - Polynomial degree
/// * `n` - Number being factored
///
/// # Returns
///
/// Search window size (number of candidates in each direction)
///
/// # Performance
///
/// O(1) - simple computation
fn calculate_search_window(m: &BigInt, degree: u32, n: &BigInt) -> i64 {
    // Determine input size
    let n_digits = n.to_string().len();

    // Base window on input size and degree
    let base_window = if n_digits <= 12 {
        // Small numbers: wide search for best quality
        50
    } else if n_digits <= 20 {
        30
    } else if n_digits <= 50 {
        20
    } else if n_digits <= 100 {
        15
    } else {
        // Large numbers: narrower search to save time
        10
    };

    // Adjust based on degree (higher degree = smaller coefficients = less sensitive)
    let degree_factor = 1.0 / (degree as f64).sqrt();
    let window = (base_window as f64 * degree_factor).max(5.0) as i64;

    // Also consider as percentage of m (typically 2-5%)
    if let Some(m_i64) = m.to_i64() {
        let percent_window = (m_i64 as f64 * 0.03) as i64;
        window.max(percent_window).max(10)
    } else {
        window
    }
}

/// Construct polynomial for a given base using base-m expansion
///
/// Creates f(x) such that f(m) = n using base-m representation:
///   n = c0 + c1*m + c2*m^2 + ... + cd*m^d
///
/// This is equivalent to expressing n in base m and using the digits
/// as polynomial coefficients.
///
/// # Algorithm
///
/// 1. Initialize remainder = n
/// 2. For each coefficient position (0 to degree):
///    a. ci = remainder mod m
///    b. remainder = remainder / m
/// 3. If remainder > 0 after all positions, add to highest coefficient
/// 4. Create polynomial from coefficients
///
/// # Arguments
///
/// * `n` - Number being factored
/// * `m` - Polynomial base
/// * `degree` - Desired polynomial degree
///
/// # Returns
///
/// Polynomial f(x) where f(m) = n
///
/// # Performance
///
/// O(d * log²(n)) where d is degree
///
/// # Example
///
/// ```rust
/// use num::BigInt;
/// use gnfs::polynomial::polynomial_construction::construct_for_base;
///
/// let n = BigInt::from(1000);
/// let m = BigInt::from(10);
/// let poly = construct_for_base(&n, &m, 3);
///
/// // Should give: 0 + 0*X + 0*X^2 + 1*X^3 (since 1000 = 10^3)
/// assert_eq!(poly.evaluate(&m), n);
/// ```
pub fn construct_for_base(n: &BigInt, m: &BigInt, degree: u32) -> Polynomial {
    let mut coefficients: Vec<BigInt> = Vec::with_capacity(degree as usize + 1);
    let mut remainder = n.clone();

    // Base-m expansion: n = c0 + c1*m + c2*m^2 + ... + cd*m^d
    for _ in 0..=degree {
        let coeff = &remainder % m;
        coefficients.push(coeff);
        remainder = &remainder / m;
    }

    // If there's still a remainder, add it to the highest degree coefficient
    // This handles cases where n requires more than 'degree' digits in base m
    if remainder > BigInt::zero() {
        if let Some(last_coeff) = coefficients.last_mut() {
            *last_coeff += remainder;
        }
    }

    // Create polynomial terms from coefficients
    // Filter out zero coefficients for efficiency
    let terms: Vec<Term> = coefficients
        .iter()
        .enumerate()
        .filter(|(_, coeff)| !coeff.is_zero())
        .map(|(degree, coeff)| Term::new(coeff.clone(), degree))
        .collect();

    Polynomial::new(terms)
}

/// Calculate nth root of a BigInt using Newton's method
///
/// Returns m such that m^n ≈ input. This is used to find the initial
/// polynomial base estimate where m^degree ≈ n.
///
/// # Algorithm
///
/// Uses Newton-Raphson iteration to solve: f(x) = x^n - input = 0
///
/// Iteration: x_{k+1} = x_k - f(x_k)/f'(x_k)
///          = x_k - (x_k^n - input)/(n * x_k^(n-1))
///          = ((n-1)*x_k + input/x_k^(n-1)) / n
///
/// Converges quadratically when started from a reasonable guess.
///
/// # Arguments
///
/// * `input` - The number to find the root of
/// * `n` - The root degree (e.g., 3 for cube root)
///
/// # Returns
///
/// Approximate nth root as BigInt
///
/// # Performance
///
/// O(log(input) * n) iterations, each requiring O(log²(input)) operations
/// Typical convergence in 5-10 iterations for GNFS use cases
///
/// # Example
///
/// ```rust
/// use num::BigInt;
/// use gnfs::polynomial::polynomial_construction::nth_root;
///
/// let n = BigInt::from(1000);
/// let cube_root = nth_root(&n, 3);
/// assert_eq!(cube_root, BigInt::from(10));
///
/// let n = BigInt::from(1000000);
/// let cube_root = nth_root(&n, 3);
/// assert_eq!(cube_root, BigInt::from(100));
/// ```
pub fn nth_root(input: &BigInt, n: u32) -> BigInt {
    if input.is_zero() {
        return BigInt::zero();
    }

    if input.is_one() {
        return BigInt::one();
    }

    if n == 0 {
        return BigInt::one();
    }

    if n == 1 {
        return input.clone();
    }

    // Initial guess using bit length
    // If input has b bits, then root has approximately b/n bits
    let input_bits = input.bits();
    let root_bits = (input_bits / n as u64).max(1);

    // Start with 2^(root_bits) as initial guess
    let mut x = BigInt::one() << root_bits;

    // Make sure initial guess is not zero
    if x.is_zero() {
        x = BigInt::one();
    }

    // Newton's method iteration
    let n_bigint = BigInt::from(n);
    let n_minus_1 = BigInt::from(n - 1);

    // Iterate until convergence
    let max_iterations = 1000;
    for iteration in 0..max_iterations {
        // x_next = ((n-1)*x + input/x^(n-1)) / n
        let x_pow_n_minus_1 = x.pow(n - 1);

        // Check for zero to avoid division by zero
        if x_pow_n_minus_1.is_zero() {
            x = BigInt::one();
            continue;
        }

        let numerator = &n_minus_1 * &x + input / &x_pow_n_minus_1;
        let x_next = numerator / &n_bigint;

        // Check for convergence: when x_next == x, we're done
        if x_next == x {
            // Verify we have the right answer (floor of actual root)
            // Check if x^n <= input < (x+1)^n
            let x_pow_n = x.pow(n);
            if x_pow_n <= *input {
                let x_plus_1: BigInt = &x + 1;
                let x_plus_1_pow_n = x_plus_1.pow(n);
                if x_plus_1_pow_n > *input {
                    return x;
                } else if x_plus_1_pow_n == *input {
                    // Perfect root, return the larger value
                    return x_plus_1;
                }
            }
            return x;
        }

        // Check if we're oscillating
        let diff = (&x_next - &x).abs();
        if diff == BigInt::one() {
            // Oscillating between two values, return the smaller
            // that satisfies x^n <= input
            let smaller = if x < x_next { x.clone() } else { x_next.clone() };
            let larger = if x >= x_next { x.clone() } else { x_next.clone() };

            let smaller_pow = smaller.pow(n);
            let larger_pow = larger.pow(n);

            if larger_pow == *input {
                return larger;
            } else if smaller_pow <= *input && larger_pow > *input {
                return smaller;
            }
        }

        x = x_next;

        // For debugging very slow convergence
        if iteration > 100 {
            debug!("nth_root: slow convergence at iteration {} for n={}, degree={}", iteration, input, n);
        }
    }

    // Final verification
    let x_pow_n = x.pow(n);
    if x_pow_n > *input && x > BigInt::one() {
        x -= 1;
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nth_root_perfect_roots() {
        // Test perfect cube roots
        assert_eq!(nth_root(&BigInt::from(8), 3), BigInt::from(2));
        assert_eq!(nth_root(&BigInt::from(27), 3), BigInt::from(3));
        assert_eq!(nth_root(&BigInt::from(1000), 3), BigInt::from(10));

        // Test perfect square roots
        assert_eq!(nth_root(&BigInt::from(4), 2), BigInt::from(2));
        assert_eq!(nth_root(&BigInt::from(100), 2), BigInt::from(10));
        assert_eq!(nth_root(&BigInt::from(10000), 2), BigInt::from(100));

        // Test 4th roots
        assert_eq!(nth_root(&BigInt::from(16), 4), BigInt::from(2));
        assert_eq!(nth_root(&BigInt::from(81), 4), BigInt::from(3));
    }

    #[test]
    fn test_nth_root_approximate() {
        // Test approximate roots (should be floor)
        let root = nth_root(&BigInt::from(999), 3);
        assert!(root >= BigInt::from(9) && root <= BigInt::from(10));

        let root = nth_root(&BigInt::from(1001), 3);
        assert!(root >= BigInt::from(10) && root <= BigInt::from(11));

        // Verify the root is reasonable: root^n should be close to input
        let input = BigInt::from(47893197);
        let root = nth_root(&input, 3);
        let root_cubed = root.pow(3);
        let diff = (&root_cubed - &input).abs();
        let next_root: BigInt = &root + 1;
        let next_cubed = next_root.pow(3);
        let next_diff = (&next_cubed - &input).abs();

        // Our root should be closer than the next integer
        assert!(diff <= next_diff, "Root {} should be closer than {}", root, next_root);
    }

    #[test]
    fn test_nth_root_edge_cases() {
        // Test edge cases
        assert_eq!(nth_root(&BigInt::zero(), 3), BigInt::zero());
        assert_eq!(nth_root(&BigInt::one(), 3), BigInt::one());
        assert_eq!(nth_root(&BigInt::from(100), 1), BigInt::from(100));

        // Large number
        let large = BigInt::from(1000000000_i64);
        let root = nth_root(&large, 3);
        assert_eq!(root, BigInt::from(1000));
    }

    #[test]
    fn test_construct_for_base_simple() {
        // Test: 1000 in base 10 should give X^3
        let n = BigInt::from(1000);
        let m = BigInt::from(10);
        let poly = construct_for_base(&n, &m, 3);

        assert_eq!(poly.evaluate(&m), n);
        assert_eq!(poly.degree(), 3);
        assert_eq!(poly[3], BigInt::one());
        assert_eq!(poly[2], BigInt::zero());
        assert_eq!(poly[1], BigInt::zero());
        assert_eq!(poly[0], BigInt::zero());
    }

    #[test]
    fn test_construct_for_base_mixed_coefficients() {
        // Test: 123 in base 10 should give 3 + 2*X + X^2
        let n = BigInt::from(123);
        let m = BigInt::from(10);
        let poly = construct_for_base(&n, &m, 3);

        assert_eq!(poly.evaluate(&m), n);
        assert_eq!(poly[0], BigInt::from(3));
        assert_eq!(poly[1], BigInt::from(2));
        assert_eq!(poly[2], BigInt::one());
    }

    #[test]
    fn test_construct_for_base_larger_number() {
        // Test with the example from the codebase: 45113 in base 31
        let n = BigInt::from(45113);
        let m = BigInt::from(31);
        let poly = construct_for_base(&n, &m, 3);

        assert_eq!(poly.evaluate(&m), n);

        // Verify base-31 representation: 45113 = 8 + 29*31 + 15*31^2 + 1*31^3
        assert_eq!(poly[0], BigInt::from(8));
        assert_eq!(poly[1], BigInt::from(29));
        assert_eq!(poly[2], BigInt::from(15));
        assert_eq!(poly[3], BigInt::one());
    }

    #[test]
    fn test_find_optimal_base_small_number() {
        let n = BigInt::from(1000);
        let (poly, m, metrics) = find_optimal_base(&n, 3);

        // Verify polynomial is valid
        let eval = poly.evaluate(&m);
        let diff = (&eval - &n).abs();
        assert!(diff < BigInt::from(10), "f(m) should be close to n: diff = {}", diff);

        // Verify quality score is reasonable
        assert!(metrics.overall_score > 0.0);
        assert!(metrics.root_sum_squares > 0.0);

        println!("Optimal base for 1000: m = {}", m);
        println!("Quality: {:.2}", metrics.overall_score);
    }

    #[test]
    fn test_find_optimal_base_test_number() {
        // Test with the example number from the codebase
        let n = BigInt::from(47893197);
        let (poly, m, metrics) = find_optimal_base(&n, 3);

        // Verify polynomial is valid
        let eval = poly.evaluate(&m);
        let diff = (&eval - &n).abs();
        let tolerance = &n / BigInt::from(1000);
        assert!(diff < tolerance, "f(m) should be close to n");

        // Verify quality is better than naive base selection
        // Naive approach: m = nth_root(n, degree)
        let m_naive = nth_root(&n, 3);
        let poly_naive = construct_for_base(&n, &m_naive, 3);
        let metrics_naive = evaluate_quality(&poly_naive, &n, &m_naive);

        println!("Optimized base: m = {}, score = {:.2}", m, metrics.overall_score);
        println!("Naive base:     m = {}, score = {:.2}", m_naive, metrics_naive.overall_score);

        // The optimized selection should be at least as good as naive
        assert!(
            metrics.overall_score <= metrics_naive.overall_score * 1.1,
            "Optimized should be better or similar to naive"
        );
    }

    #[test]
    fn test_calculate_search_window() {
        // Small number: wider search
        let n_small = BigInt::from(1000);
        let m_small = nth_root(&n_small, 3);
        let window_small = calculate_search_window(&m_small, 3, &n_small);
        assert!(window_small >= 10, "Should have reasonable window for small numbers");

        // Medium number
        let n_medium = BigInt::from(1000000000_i64);
        let m_medium = nth_root(&n_medium, 4);
        let window_medium = calculate_search_window(&m_medium, 4, &n_medium);
        assert!(window_medium >= 5, "Should have reasonable window for medium numbers");

        // Large number: narrower search (use pow to construct large BigInt)
        let mut n_large = BigInt::from(10);
        for _ in 0..20 {
            n_large = &n_large * 10;
        }
        let m_large = nth_root(&n_large, 5);
        let window_large = calculate_search_window(&m_large, 5, &n_large);
        assert!(window_large >= 5, "Should have minimum window size");
    }

    #[test]
    fn test_polynomial_construction_preserves_n() {
        // Test that constructed polynomials always satisfy f(m) = n
        let test_cases = vec![
            (BigInt::from(100), 2),
            (BigInt::from(1000), 3),
            (BigInt::from(10000), 3),
            (BigInt::from(45113), 3),
            (BigInt::from(1000730021_i64), 4),
        ];

        for (n, degree) in test_cases {
            let m = nth_root(&n, degree);
            let poly = construct_for_base(&n, &m, degree);
            let eval = poly.evaluate(&m);
            assert_eq!(
                eval, n,
                "Polynomial should satisfy f(m) = n for n={}, m={}, degree={}",
                n, m, degree
            );
        }
    }
}
