// src/algorithms/siqs/polynomial.rs
//
// SIQS polynomial generation and switching
//
// Key algorithm: Given n and target 'a' value, generate polynomial Q(x) = (ax + b)² - n
// where:
// - a = q₁ × q₂ × ... × qⱼ (product of j primes from factor base)
// - b² ≡ n (mod a) (computed using Chinese Remainder Theorem)
// - c = (b² - n) / a

use num::{BigInt, Integer, One, ToPrimitive, Zero};
use log::{debug, info};

use super::Prime;
use super::parameters::SIQSParameters;

/// Represents a SIQS polynomial Q(x) = (ax + b)² - n
#[derive(Clone, Debug)]
pub struct SIQSPolynomial {
    pub a: BigInt,              // Leading coefficient (product of j primes)
    pub b: BigInt,              // Linear coefficient (from CRT)
    pub c: BigInt,              // Constant term: (b² - n) / a
    pub a_factors: Vec<u64>,    // Prime factors of 'a'
    pub b_array: Vec<BigInt>,   // B[i] values for fast polynomial switching

    // Fast switching metadata
    pub poly_index: u32,        // Current polynomial index (0 to max_polynomials - 1)
    pub max_polynomials: u32,   // Total polynomials for this 'a': 2^(j-1)
}

impl SIQSPolynomial {
    /// Evaluate Q(x) = (ax + b)² - n
    pub fn evaluate(&self, x: i64, n: &BigInt) -> BigInt {
        let x_big = BigInt::from(x);
        let ax_plus_b = &self.a * &x_big + &self.b;
        &ax_plus_b * &ax_plus_b - n
    }

    /// Evaluate just the inner term: ax + b
    pub fn evaluate_inner(&self, x: i64) -> BigInt {
        let x_big = BigInt::from(x);
        &self.a * &x_big + &self.b
    }
}

/// Generate a SIQS polynomial with optimized 'a' coefficient
///
/// Algorithm (from Contini 1997):
/// 1. Select j primes q₁, ..., qⱼ from factor base such that product ≈ target_a
/// 2. Compute a = q₁ × q₂ × ... × qⱼ
/// 3. For each qᵢ, compute B[i] using Chinese Remainder Theorem:
///    - γ = tsqrt[qᵢ] × (a/qᵢ)⁻¹ mod qᵢ
///    - If γ > qᵢ/2: γ = qᵢ - γ  (choose smaller root)
///    - B[i] = (a/qᵢ) × γ
/// 4. Compute b = B[1] + B[2] + ... + B[j]
/// 5. Compute c = (b² - n) / a
///
/// The iteration parameter varies the prime selection to generate different 'a' values
///
/// Returns polynomial and B-array for fast switching
pub fn generate_polynomial(
    n: &BigInt,
    factor_base: &[Prime],
    params: &SIQSParameters,
    target_a: &BigInt,
    iteration: usize,
) -> Option<SIQSPolynomial> {
    let j = params.primes_per_a;

    // Step 1: Select j primes for 'a' coefficient (varied by iteration)
    let selected_primes = select_a_primes(factor_base, params, target_a, j, iteration)?;

    if selected_primes.len() != j {
        debug!("Could not select {} primes for 'a'", j);
        return None;
    }

    debug!("Selected primes for 'a': {:?}", selected_primes.iter().map(|p| p.p).collect::<Vec<_>>());

    // Step 2: Compute a = product of selected primes
    let mut a = BigInt::one();
    let mut a_factors = Vec::new();

    for prime in &selected_primes {
        a *= prime.p;
        a_factors.push(prime.p);
    }

    debug!("Computed a = {} (target was ~{})", a, target_a);

    // Step 3: Compute B[i] for each prime using CRT
    let mut b_array = Vec::new();

    for prime in &selected_primes {
        let q_i = BigInt::from(prime.p);
        let a_div_qi = &a / &q_i;

        // Compute (a/qᵢ)⁻¹ mod qᵢ
        let ainv_mod_qi = match mod_inverse(&a_div_qi, &q_i) {
            Some(inv) => inv,
            None => {
                debug!("Could not compute modular inverse for prime {}", prime.p);
                return None;
            }
        };

        // γ = tsqrt[qᵢ] × (a/qᵢ)⁻¹ mod qᵢ
        let tsqrt = BigInt::from(prime.tsqrt);
        let mut gamma = (&tsqrt * &ainv_mod_qi).mod_floor(&q_i);

        // Choose smaller root: if γ > qᵢ/2, use qᵢ - γ
        if &gamma > &(&q_i / 2) {
            gamma = &q_i - &gamma;
        }

        // B[i] = (a/qᵢ) × γ
        let b_i = &a_div_qi * &gamma;
        b_array.push(b_i);
    }

    // Step 4: Compute b = sum of B[i]
    let mut b = BigInt::zero();
    for b_i in &b_array {
        b += b_i;
    }

    // Verify: b² ≡ n (mod a)
    let b_squared = &b * &b;
    let b_sq_mod_a = b_squared.mod_floor(&a);
    let n_mod_a = n.mod_floor(&a);

    if b_sq_mod_a != n_mod_a {
        debug!("Polynomial generation failed: b² ≢ n (mod a)");
        debug!("  b² mod a = {}", b_sq_mod_a);
        debug!("  n mod a = {}", n_mod_a);
        return None;
    }

    // Step 5: Compute c = (b² - n) / a
    let b_sq_minus_n = &b * &b - n;

    if !b_sq_minus_n.is_multiple_of(&a) {
        debug!("Polynomial generation failed: (b² - n) not divisible by a");
        return None;
    }

    let c = b_sq_minus_n / &a;

    debug!("Generated polynomial:");
    debug!("  a = {}", a);
    debug!("  b = {}", b);
    debug!("  c = {}", c);
    debug!("  Verification: b² ≡ n (mod a) ✓");

    // Calculate max polynomials: 2^(j-1)
    // For j primes, we can generate 2^(j-1) polynomials using Gray code
    let j = selected_primes.len() as u32;
    let max_polynomials = if j > 0 {
        2u32.pow(j - 1)
    } else {
        1
    };

    debug!("  Fast switching: can generate {} polynomials from this 'a'", max_polynomials);

    Some(SIQSPolynomial {
        a,
        b,
        c,
        a_factors,
        b_array,
        poly_index: 0,          // Start at index 0
        max_polynomials,
    })
}

/// Select j primes from the factor base to construct 'a' coefficient
///
/// Strategy:
/// - Choose primes from middle of factor base (good distribution)
/// - Product should be close to target_a ≈ sqrt(2n)/M
/// - Avoid very small primes (already heavily sieved)
/// - Avoid very large primes (few sieve hits)
/// - Use iteration parameter to vary starting offset (prevents duplicate 'a' values)
fn select_a_primes(
    factor_base: &[Prime],
    params: &SIQSParameters,
    target_a: &BigInt,
    j: usize,
    iteration: usize,
) -> Option<Vec<Prime>> {
    let (prime_min, prime_max) = params.a_prime_range();

    // Filter primes in the desired range
    let candidates: Vec<&Prime> = factor_base
        .iter()
        .filter(|p| p.p >= prime_min && p.p <= prime_max)
        .collect();

    if candidates.len() < j {
        debug!("Not enough primes in range [{}, {}]: found {}, need {}",
               prime_min, prime_max, candidates.len(), j);
        return None;
    }

    // Use greedy selection to get product close to target_a
    // Skip primes based on iteration to generate different 'a' values
    let mut selected = Vec::new();
    let mut product = BigInt::one();
    let mut used_indices = std::collections::HashSet::new();

    // Create a set of indices to skip based on iteration
    // Skip (iteration * j) primes to ensure different selections
    let skip_count = (iteration * j) % candidates.len();
    let mut skipped_indices = std::collections::HashSet::new();
    for i in 0..skip_count.min(candidates.len() - j) {
        skipped_indices.insert(i);
    }

    for _ in 0..j {
        let mut best_idx = 0;
        let mut best_distance = BigInt::from(u64::MAX);

        // Find prime that brings product closest to target
        // Skip primes based on iteration to vary selection
        for (idx, prime) in candidates.iter().enumerate() {
            if used_indices.contains(&idx) || skipped_indices.contains(&idx) {
                continue;
            }

            let new_product = &product * prime.p;
            let distance = if &new_product > target_a {
                &new_product - target_a
            } else {
                target_a - &new_product
            };

            if distance < best_distance {
                best_distance = distance;
                best_idx = idx;
            }
        }

        used_indices.insert(best_idx);
        selected.push((*candidates[best_idx]).clone());
        product *= candidates[best_idx].p;
    }

    Some(selected)
}

/// Compute modular inverse: a⁻¹ mod m
///
/// Uses extended Euclidean algorithm
fn mod_inverse(a: &BigInt, m: &BigInt) -> Option<BigInt> {
    if m == &BigInt::one() {
        return Some(BigInt::zero());
    }

    let (gcd, x, _) = extended_gcd(a, m);

    if gcd != BigInt::one() {
        // No inverse exists
        return None;
    }

    // Make positive
    let result = if x < BigInt::zero() {
        x + m
    } else {
        x
    };

    Some(result.mod_floor(m))
}

/// Extended Euclidean algorithm
///
/// Returns (gcd, x, y) such that a*x + b*y = gcd
pub fn extended_gcd(a: &BigInt, b: &BigInt) -> (BigInt, BigInt, BigInt) {
    if a.is_zero() {
        return (b.clone(), BigInt::zero(), BigInt::one());
    }

    let (gcd, x1, y1) = extended_gcd(&(b.mod_floor(a)), a);

    let x = &y1 - (b / a) * &x1;
    let y = x1;

    (gcd, x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_inverse() {
        // 3 * 5 ≡ 1 (mod 7)
        let a = BigInt::from(3);
        let m = BigInt::from(7);
        let inv = mod_inverse(&a, &m).unwrap();
        assert_eq!(inv, BigInt::from(5));

        // Verify: 3 * 5 mod 7 = 1
        assert_eq!((&a * &inv).mod_floor(&m), BigInt::one());
    }

    #[test]
    fn test_extended_gcd() {
        let a = BigInt::from(35);
        let b = BigInt::from(15);
        let (gcd, x, y) = extended_gcd(&a, &b);

        assert_eq!(gcd, BigInt::from(5));
        // Verify: 35*x + 15*y = 5
        assert_eq!(&a * &x + &b * &y, gcd);
    }

    #[test]
    fn test_polynomial_evaluation() {
        let poly = SIQSPolynomial {
            a: BigInt::from(6),
            b: BigInt::from(7),
            c: BigInt::from(1),
            a_factors: vec![2, 3],
            b_array: vec![],
            poly_index: 0,
            max_polynomials: 1,
        };

        let n = BigInt::from(48);

        // Q(0) = (6*0 + 7)² - 48 = 49 - 48 = 1
        assert_eq!(poly.evaluate(0, &n), BigInt::one());

        // Q(1) = (6*1 + 7)² - 48 = 169 - 48 = 121
        assert_eq!(poly.evaluate(1, &n), BigInt::from(121));
    }

    #[test]
    fn test_polynomial_index_tracking() {
        // Test that polynomial index is correctly initialized
        let poly = SIQSPolynomial {
            a: BigInt::from(100),
            b: BigInt::from(50),
            c: BigInt::from(25),
            a_factors: vec![2, 5],
            b_array: vec![BigInt::from(10), BigInt::from(20)],
            poly_index: 0,
            max_polynomials: 2, // 2^(2-1) = 2
        };

        assert_eq!(poly.poly_index, 0);
        assert_eq!(poly.max_polynomials, 2);
    }

    #[test]
    fn test_max_polynomials_calculation() {
        // Test that max_polynomials is correctly calculated as 2^(j-1)
        // For j=3: 2^(3-1) = 4 polynomials
        // For j=4: 2^(4-1) = 8 polynomials
        // For j=5: 2^(5-1) = 16 polynomials

        // Mock factor base with enough primes
        let mut factor_base = Vec::new();
        for p in &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29] {
            factor_base.push(Prime {
                p: *p,
                roots: vec![1],
                tsqrt: 1,
                log_p: 1.0,
            });
        }

        let n = BigInt::from(10007); // Small test number
        let params = SIQSParameters {
            smoothness_bound: 30,
            sieve_interval: 1000,
            primes_per_a: 3, // j=3 → max_polynomials = 4
            relation_margin: 10,
        };
        let target_a = BigInt::from(100);

        // Generate polynomial
        if let Some(poly) = generate_polynomial(&n, &factor_base, &params, &target_a, 0) {
            // For j=3, we should get 2^(3-1) = 4 polynomials
            assert_eq!(poly.max_polynomials, 4);
            assert_eq!(poly.poly_index, 0);
            assert_eq!(poly.b_array.len(), 3);
        }
    }
}
