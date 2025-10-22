// src/polynomial/polynomial_quality.rs

use num::{BigInt, ToPrimitive, Zero, One, Signed};
use crate::polynomial::polynomial::Polynomial;

/// Quality metrics for evaluating GNFS polynomials
///
/// Better polynomials have:
/// - Smaller coefficients (lower root sum of squares)
/// - Balanced norms between rational and algebraic factor bases
/// - Good root properties
#[derive(Debug, Clone)]
pub struct PolynomialQualityMetrics {
    /// Root sum of squares: sqrt(sum(ci^2)) - measures overall coefficient size
    pub root_sum_squares: f64,

    /// Maximum absolute coefficient value
    pub max_coefficient: BigInt,

    /// Sum of absolute coefficients
    pub sum_absolute_coefficients: BigInt,

    /// Skewness score - ratio between rational and algebraic norms
    /// Closer to 1.0 is better (balanced)
    pub skewness_score: f64,

    /// Overall quality score (lower is better)
    /// Combines RSS, max coefficient, and skewness
    pub overall_score: f64,
}

impl PolynomialQualityMetrics {
    /// Display the quality metrics in a human-readable format
    pub fn to_string(&self) -> String {
        format!(
            "Quality Metrics:\n  RSS: {:.2}\n  Max Coeff: {}\n  Sum |Coeffs|: {}\n  Skewness: {:.4}\n  Overall Score: {:.2}",
            self.root_sum_squares,
            self.max_coefficient,
            self.sum_absolute_coefficients,
            self.skewness_score,
            self.overall_score
        )
    }
}

/// Evaluate polynomial quality metrics for GNFS
///
/// This function computes several quality metrics that predict how well
/// a polynomial will perform in the GNFS algorithm. Better polynomials lead
/// to more smooth relations being found during sieving, which dramatically
/// reduces overall factorization time.
///
/// # Quality Metrics
///
/// 1. **Root Sum of Squares (RSS)**: sqrt(sum(ci^2))
///    - Measures overall coefficient magnitude
///    - Lower is better
///    - Dominates the overall quality score
///
/// 2. **Max Coefficient**: max(|ci|)
///    - Largest absolute coefficient value
///    - Affects norm sizes during sieving
///    - Lower is better
///
/// 3. **Skewness Score**: Balance between rational and algebraic norms
///    - Ratio close to 1.0 indicates good balance
///    - Affects sieving efficiency
///    - Range: [0, infinity], target ≈ 1.0
///
/// # Algorithm
///
/// The overall quality score is computed as:
///   score = RSS + (skewness_penalty * 0.1)
/// where skewness_penalty = |log(skewness) - 0|
///
/// This heavily weights coefficient size while penalizing extreme imbalance.
///
/// # Arguments
///
/// * `poly` - The polynomial to evaluate
/// * `n` - The number being factored
/// * `m` - The polynomial base (m such that f(m) ≈ n)
///
/// # Returns
///
/// PolynomialQualityMetrics struct containing all quality measurements
///
/// # Performance
///
/// O(d) where d is the polynomial degree (typically 3-5)
/// Each coefficient operation is O(log²(coeff_size))
///
/// # Example
///
/// ```rust
/// use num::BigInt;
/// use gnfs::polynomial::polynomial::Polynomial;
/// use gnfs::polynomial::polynomial_quality::evaluate_quality;
///
/// let n = BigInt::from(47893197);
/// let m = BigInt::from(363);
/// let poly = Polynomial::parse("8 + 29*X + 15*X^2 + X^3");
///
/// let metrics = evaluate_quality(&poly, &n, &m);
/// println!("Polynomial quality score: {:.2}", metrics.overall_score);
/// // Lower scores indicate better polynomials
/// ```
pub fn evaluate_quality(poly: &Polynomial, _n: &BigInt, m: &BigInt) -> PolynomialQualityMetrics {
    let degree = poly.degree();

    // Calculate root sum of squares: sqrt(sum(ci^2))
    let mut sum_squares = BigInt::zero();
    let mut max_coeff = BigInt::zero();
    let mut sum_abs_coeffs = BigInt::zero();

    for i in 0..=degree {
        let coeff = &poly[i];
        let abs_coeff = coeff.abs();

        // Update max coefficient
        if abs_coeff > max_coeff {
            max_coeff = abs_coeff.clone();
        }

        // Sum of absolute coefficients
        sum_abs_coeffs += &abs_coeff;

        // Sum of squares
        sum_squares += coeff * coeff;
    }

    // Convert to f64 for root calculation
    // For very large coefficients, this is an approximation
    let rss = approximate_sqrt(&sum_squares);

    // Calculate skewness score
    // Skewness measures the ratio between algebraic and rational norm sizes
    // For now, use a simple heuristic based on coefficient distribution
    let skewness = calculate_skewness(poly, m);

    // Overall quality score: heavily weight RSS, lightly penalize extreme skewness
    // Lower score is better
    let skewness_penalty = (skewness.ln().abs()) * 0.1;
    let overall_score = rss + skewness_penalty;

    PolynomialQualityMetrics {
        root_sum_squares: rss,
        max_coefficient: max_coeff,
        sum_absolute_coefficients: sum_abs_coeffs,
        skewness_score: skewness,
        overall_score,
    }
}

/// Calculate approximate square root of a BigInt as f64
///
/// For very large numbers, this returns an approximation suitable for
/// comparison purposes. The relative ordering is preserved even when
/// exact values are lost to floating point precision.
///
/// # Arguments
///
/// * `n` - The number to take the square root of
///
/// # Returns
///
/// Approximate square root as f64
///
/// # Performance
///
/// O(log(n)) for string conversion and parsing
///
/// # Example
///
/// ```rust
/// use num::BigInt;
/// let n = BigInt::from(10000);
/// let sqrt = approximate_sqrt(&n);
/// assert!((sqrt - 100.0).abs() < 0.1);
/// ```
fn approximate_sqrt(n: &BigInt) -> f64 {
    // For numbers that fit in f64, compute directly
    if let Some(n_f64) = n.to_f64() {
        return n_f64.sqrt();
    }

    // For very large numbers, use logarithmic approximation
    // sqrt(n) ≈ exp(0.5 * ln(n))
    // We approximate ln(n) from the decimal representation
    let s = n.to_string();
    let digits = s.len();

    // ln(n) ≈ digits * ln(10) + ln(first_few_digits)
    let first_digits = s[..digits.min(15)].parse::<f64>().unwrap_or(1.0);
    let ln_n = (digits as f64) * 2.302585 + first_digits.ln();

    // Return exp(0.5 * ln(n))
    (0.5 * ln_n).exp()
}

/// Calculate skewness score for a polynomial
///
/// Skewness measures the balance between rational and algebraic norm sizes.
/// A well-balanced polynomial (skewness ≈ 1.0) leads to more efficient sieving
/// because both factor bases contribute equally to finding smooth relations.
///
/// # Algorithm
///
/// The skewness is computed as the ratio of characteristic sizes:
///   skewness = (algebraic_size / rational_size)^(1/d)
///
/// Where:
/// - algebraic_size: geometric mean of |ci|
/// - rational_size: m^d / n (characteristic rational norm size)
///
/// # Arguments
///
/// * `poly` - The polynomial to evaluate
/// * `m` - The polynomial base
///
/// # Returns
///
/// Skewness score as f64 (target ≈ 1.0)
///
/// # Performance
///
/// O(d) where d is polynomial degree
///
/// # Example
///
/// ```rust
/// use num::BigInt;
/// use gnfs::polynomial::polynomial::Polynomial;
///
/// let poly = Polynomial::parse("2 + 3*X + X^2");
/// let m = BigInt::from(10);
/// let skew = calculate_skewness(&poly, &m);
/// // Skew ≈ 1.0 indicates good balance
/// ```
fn calculate_skewness(poly: &Polynomial, m: &BigInt) -> f64 {
    let degree = poly.degree();

    // Geometric mean of absolute coefficients (approximation of algebraic norm size)
    let mut product = 1.0_f64;
    let mut count = 0;

    for i in 0..=degree {
        let coeff = &poly[i];
        if !coeff.is_zero() {
            if let Some(abs_coeff_f64) = coeff.abs().to_f64() {
                product *= abs_coeff_f64;
                count += 1;
            } else {
                // For very large coefficients, use logarithmic approximation
                let s = coeff.abs().to_string();
                let digits = s.len();
                let first_digits = s[..digits.min(10)].parse::<f64>().unwrap_or(1.0);
                let log_coeff = (digits as f64) * 2.302585 + first_digits.ln();
                product *= log_coeff.exp();
                count += 1;
            }
        }
    }

    if count == 0 {
        return 1.0;
    }

    let geometric_mean = product.powf(1.0 / count as f64);

    // Rational norm size characteristic: related to m
    let m_f64 = if let Some(m_val) = m.to_f64() {
        m_val
    } else {
        // Approximate for large m
        let s = m.to_string();
        let digits = s.len();
        let first_digits = s[..digits.min(10)].parse::<f64>().unwrap_or(1.0);
        ((digits as f64) * 2.302585 + first_digits.ln()).exp()
    };

    // Skewness is the ratio of these characteristic sizes
    // This is a simplified heuristic; full Murphy-E calculation is more complex
    let skewness = geometric_mean / m_f64.powf(0.5);

    // Ensure skewness is in reasonable range [0.1, 10.0]
    skewness.max(0.1).min(10.0)
}

/// Compare two polynomials and return the better one
///
/// Compares polynomials based on their overall quality score.
/// Lower scores indicate better polynomials that will perform
/// better during GNFS sieving.
///
/// # Arguments
///
/// * `poly1`, `m1` - First polynomial and its base
/// * `poly2`, `m2` - Second polynomial and its base
/// * `n` - The number being factored
///
/// # Returns
///
/// Tuple of (better_polynomial, better_base, quality_metrics)
///
/// # Example
///
/// ```rust
/// use num::BigInt;
/// use gnfs::polynomial::polynomial::Polynomial;
/// use gnfs::polynomial::polynomial_quality::select_best;
///
/// let n = BigInt::from(1000);
/// let poly1 = Polynomial::parse("0 + 0*X + 0*X^2 + X^3");
/// let m1 = BigInt::from(10);
/// let poly2 = Polynomial::parse("8 + 2*X + X^2 + X^3");
/// let m2 = BigInt::from(9);
///
/// let (best_poly, best_m, metrics) = select_best(&poly1, &m1, &poly2, &m2, &n);
/// println!("Selected polynomial with score: {:.2}", metrics.overall_score);
/// ```
pub fn select_best<'a>(
    poly1: &'a Polynomial,
    m1: &'a BigInt,
    poly2: &'a Polynomial,
    m2: &'a BigInt,
    n: &BigInt,
) -> (&'a Polynomial, &'a BigInt, PolynomialQualityMetrics) {
    let metrics1 = evaluate_quality(poly1, n, m1);
    let metrics2 = evaluate_quality(poly2, n, m2);

    if metrics1.overall_score <= metrics2.overall_score {
        (poly1, m1, metrics1)
    } else {
        (poly2, m2, metrics2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polynomial::polynomial::{Polynomial, Term};

    #[test]
    fn test_approximate_sqrt_small_numbers() {
        assert!((approximate_sqrt(&BigInt::from(100)) - 10.0).abs() < 0.01);
        assert!((approximate_sqrt(&BigInt::from(10000)) - 100.0).abs() < 0.01);
        assert!((approximate_sqrt(&BigInt::from(1000000)) - 1000.0).abs() < 0.1);
    }

    #[test]
    fn test_approximate_sqrt_zero() {
        assert_eq!(approximate_sqrt(&BigInt::zero()), 0.0);
    }

    #[test]
    fn test_quality_prefers_smaller_coefficients() {
        let n = BigInt::from(1000);
        let m = BigInt::from(10);

        // Polynomial with small coefficients: X^3
        let small_coef = Polynomial::new(vec![
            Term::new(BigInt::zero(), 0),
            Term::new(BigInt::zero(), 1),
            Term::new(BigInt::zero(), 2),
            Term::new(BigInt::one(), 3),
        ]);

        // Polynomial with large coefficients: 100 + 200*X + 300*X^2 + X^3
        let large_coef = Polynomial::new(vec![
            Term::new(BigInt::from(100), 0),
            Term::new(BigInt::from(200), 1),
            Term::new(BigInt::from(300), 2),
            Term::new(BigInt::one(), 3),
        ]);

        let metrics_small = evaluate_quality(&small_coef, &n, &m);
        let metrics_large = evaluate_quality(&large_coef, &n, &m);

        assert!(
            metrics_small.overall_score < metrics_large.overall_score,
            "Smaller coefficients should score better: {} vs {}",
            metrics_small.overall_score,
            metrics_large.overall_score
        );

        assert!(
            metrics_small.root_sum_squares < metrics_large.root_sum_squares,
            "RSS should be smaller for small coefficients"
        );
    }

    #[test]
    fn test_quality_metrics_structure() {
        let n = BigInt::from(1000);
        let m = BigInt::from(10);
        let poly = Polynomial::new(vec![
            Term::new(BigInt::from(8), 0),
            Term::new(BigInt::from(29), 1),
            Term::new(BigInt::from(15), 2),
            Term::new(BigInt::one(), 3),
        ]);

        let metrics = evaluate_quality(&poly, &n, &m);

        // Verify metrics are reasonable
        assert!(metrics.root_sum_squares > 0.0);
        assert!(metrics.max_coefficient >= BigInt::from(29));
        assert!(metrics.sum_absolute_coefficients > BigInt::zero());
        assert!(metrics.skewness_score > 0.0);
        assert!(metrics.overall_score > 0.0);
    }

    #[test]
    fn test_select_best_chooses_lower_score() {
        let n = BigInt::from(1000);

        let poly1 = Polynomial::new(vec![
            Term::new(BigInt::from(100), 0),
            Term::new(BigInt::from(50), 1),
            Term::new(BigInt::one(), 2),
        ]);
        let m1 = BigInt::from(30);

        let poly2 = Polynomial::new(vec![
            Term::new(BigInt::from(10), 0),
            Term::new(BigInt::from(5), 1),
            Term::new(BigInt::one(), 2),
        ]);
        let m2 = BigInt::from(31);

        let (best_poly, best_m, _metrics) = select_best(&poly1, &m1, &poly2, &m2, &n);

        // poly2 has smaller coefficients, so it should be selected
        assert_eq!(best_poly, &poly2);
        assert_eq!(best_m, &m2);
    }

    #[test]
    fn test_skewness_calculation() {
        let poly = Polynomial::new(vec![
            Term::new(BigInt::from(2), 0),
            Term::new(BigInt::from(3), 1),
            Term::new(BigInt::one(), 2),
        ]);
        let m = BigInt::from(10);

        let skew = calculate_skewness(&poly, &m);

        // Skewness should be positive and reasonably bounded
        assert!(skew > 0.0);
        assert!(skew <= 10.0);
        assert!(skew >= 0.1);
    }
}
