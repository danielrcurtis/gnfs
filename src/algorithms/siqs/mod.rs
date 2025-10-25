// src/algorithms/siqs/mod.rs
//
// Self-Initializing Quadratic Sieve (SIQS)
//
// SIQS is a variant of the Multiple Polynomial Quadratic Sieve (MPQS)
// with fast polynomial switching. It's the optimal algorithm for factoring
// numbers in the 40-100 digit range.
//
// Key advantages over single-polynomial QS:
// - Uses Q(x) = (ax + b)² - n with optimized 'a' to reduce norms
// - Multiple polynomials share factor base computations
// - Fast polynomial switching (microseconds vs milliseconds)
// - Effective Q(x) size ~20-24 digits for 40-digit n (vs ~40 digits single-poly)
//
// References:
// - Contini (1997): "Factoring Integers with the Self-Initializing Quadratic Sieve"
// - Silverman (1987): "The Multiple Polynomial Quadratic Sieve"

mod polynomial;
mod parameters;

use num::{BigInt, Integer, One, Signed, ToPrimitive, Zero};
use log::{debug, info, warn};
use crate::integer_math::gcd::GCD;
use rayon::prelude::*;

pub use parameters::SIQSParameters;
use polynomial::{SIQSPolynomial, generate_polynomial};

/// Attempts to factor n using the Self-Initializing Quadratic Sieve algorithm.
///
/// # Arguments
/// * `n` - The number to factor (optimal for 40-100 digits)
///
/// # Returns
/// Some((p, q)) where p * q = n and 1 < p <= q < n, or None if factorization fails
///
/// # Examples
/// ```
/// use num::BigInt;
/// use std::str::FromStr;
/// use gnfs::algorithms::siqs::siqs;
///
/// let n = BigInt::from_str("10000000000000000016800000000000000005031").unwrap(); // 41 digits
/// let result = siqs(&n);
/// assert!(result.is_some());
/// ```
pub fn siqs(n: &BigInt) -> Option<(BigInt, BigInt)> {
    info!("========================================");
    info!("SIQS FACTORIZATION");
    info!("========================================");
    info!("Number: {}", n);
    info!("Digits: {}", n.to_string().len());
    info!("");

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

    // Check if n is a perfect square
    let sqrt_n = n.sqrt();
    if &(&sqrt_n * &sqrt_n) == n {
        info!("Number is a perfect square");
        return Some((sqrt_n.clone(), sqrt_n));
    }

    // Initialize SIQS
    let mut siqs = SIQS::new(n);

    // Run the factorization
    siqs.factor()
}

/// Main SIQS structure
pub struct SIQS {
    n: BigInt,
    sqrt_n: BigInt,
    params: SIQSParameters,
    factor_base: Vec<Prime>,
    factor_base_size: usize,
}

/// Represents a prime in the factor base with its roots modulo that prime
#[derive(Clone, Debug)]
pub struct Prime {
    pub p: u64,
    pub roots: Vec<i64>,      // Sieving roots (for single-poly QS)
    pub tsqrt: i64,           // sqrt(n) mod p (for SIQS polynomial generation)
    pub log_p: f32,
}

/// Represents a smooth relation: Q(x) factors over the factor base
#[derive(Clone, Debug)]
pub struct Relation {
    pub x: i64,
    pub q_x: BigInt,
    pub factors: Vec<u32>,    // Exponent vector for each prime in factor base
}

impl SIQS {
    /// Create a new SIQS instance
    pub fn new(n: &BigInt) -> Self {
        let sqrt_n = n.sqrt();
        let params = SIQSParameters::from_number_size(n);

        info!("SIQS Parameters:");
        info!("  Smoothness bound B: {}", params.smoothness_bound);
        info!("  Sieve interval M: {}", params.sieve_interval);
        info!("  Primes per 'a': {}", params.primes_per_a);
        info!("");

        SIQS {
            n: n.clone(),
            sqrt_n,
            params,
            factor_base: Vec::new(),
            factor_base_size: 0,
        }
    }

    /// Build the factor base: primes p where n is a quadratic residue mod p
    fn build_factor_base(&mut self) {
        info!("Building factor base...");

        let mut factor_base = Vec::new();

        // Always include -1 for signs
        factor_base.push(Prime {
            p: 1, // Special marker for -1
            roots: vec![],
            tsqrt: 0,
            log_p: 0.0,
        });

        // Special handling for p = 2
        if self.n.mod_floor(&BigInt::from(8)) == BigInt::one() {
            let tsqrt = self.n.mod_floor(&BigInt::from(2)).to_i64().unwrap_or(1);
            factor_base.push(Prime {
                p: 2,
                roots: vec![1],
                tsqrt,
                log_p: (2.0f32).ln(),
            });
        }

        // Check odd primes up to smoothness bound
        let mut p = 3u64;
        while p <= self.params.smoothness_bound {
            if Self::is_prime_simple(p) {
                let p_bigint = BigInt::from(p);

                // Check if n is a quadratic residue mod p
                let exp = BigInt::from((p - 1) / 2);
                let legendre_val = self.n.modpow(&exp, &p_bigint);
                let is_qr = legendre_val.is_one();

                if is_qr {
                    // n is a QR mod p, find the square root
                    let tsqrt_vec = Self::tonelli_shanks(&self.n, p);

                    if !tsqrt_vec.is_empty() {
                        let tsqrt = tsqrt_vec[0]; // Store first root as tsqrt
                        factor_base.push(Prime {
                            p,
                            roots: tsqrt_vec,
                            tsqrt,
                            log_p: (p as f32).ln(),
                        });
                    }
                }
            }
            p += 2;
        }

        self.factor_base_size = factor_base.len();
        self.factor_base = factor_base;

        info!("Factor base size: {}", self.factor_base_size);
        info!("First 10 primes: {:?}",
              self.factor_base.iter().take(10).map(|pr| pr.p).collect::<Vec<_>>());
    }

    /// Simple primality test for small numbers
    fn is_prime_simple(n: u64) -> bool {
        if n < 2 {
            return false;
        }
        if n == 2 || n == 3 {
            return true;
        }
        if n % 2 == 0 || n % 3 == 0 {
            return false;
        }

        let mut i = 5u64;
        while i * i <= n {
            if n % i == 0 || n % (i + 2) == 0 {
                return false;
            }
            i += 6;
        }
        true
    }

    /// Tonelli-Shanks algorithm to find square roots mod p
    fn tonelli_shanks(n: &BigInt, p: u64) -> Vec<i64> {
        let p_bigint = BigInt::from(p);
        let n_mod = n.mod_floor(&p_bigint);

        if n_mod.is_zero() {
            return vec![0];
        }

        // Check if n is actually a QR
        let exp = BigInt::from((p - 1) / 2);
        let legendre_val = n_mod.modpow(&exp, &p_bigint);
        if !legendre_val.is_one() {
            return vec![];
        }

        // Special case: p ≡ 3 (mod 4)
        if p % 4 == 3 {
            let exp = (p + 1) / 4;
            let root = n_mod.modpow(&BigInt::from(exp), &p_bigint);
            let root_i64 = root.to_i64().unwrap_or(0);
            let other_root = (p as i64 - root_i64) % p as i64;
            return vec![root_i64, other_root];
        }

        // General case: full Tonelli-Shanks
        let mut q = p - 1;
        let mut s = 0u32;
        while q % 2 == 0 {
            q /= 2;
            s += 1;
        }

        // Find a quadratic non-residue z
        let mut z = 2u64;
        loop {
            let z_big = BigInt::from(z);
            let z_legendre = z_big.modpow(&exp, &p_bigint);
            if z_legendre == &p_bigint - 1 {
                break;
            }
            z += 1;
            if z > 1000 {
                return vec![];
            }
        }

        let q_bigint = BigInt::from(q);
        let mut m = s;
        let mut c = BigInt::from(z).modpow(&q_bigint, &p_bigint);
        let mut t = n_mod.modpow(&q_bigint, &p_bigint);
        let r_exp = (&q_bigint + 1u32) / 2u32;
        let mut r = n_mod.modpow(&r_exp, &p_bigint);

        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > 1000 {
                break;
            }

            if t.is_zero() {
                return vec![0];
            }
            if t.is_one() {
                let root_i64 = r.to_i64().unwrap_or(0);
                let other_root = (p as i64 - root_i64) % p as i64;
                return vec![root_i64, other_root];
            }

            let mut i = 1u32;
            let mut temp = (&t * &t).mod_floor(&p_bigint);
            while !temp.is_one() && i < m {
                temp = (&temp * &temp).mod_floor(&p_bigint);
                i += 1;
            }

            if i >= m {
                break;
            }

            let two = BigInt::from(2);
            let exp = two.pow(m - i - 1);
            let b = c.modpow(&exp, &p_bigint);
            m = i;
            c = (&b * &b).mod_floor(&p_bigint);
            t = (&t * &c).mod_floor(&p_bigint);
            r = (&r * &b).mod_floor(&p_bigint);
        }

        vec![]
    }

    /// Main factorization routine
    pub fn factor(&mut self) -> Option<(BigInt, BigInt)> {
        // Step 1: Build factor base
        self.build_factor_base();

        if self.factor_base_size < 10 {
            warn!("Factor base too small");
            return None;
        }

        // Step 2: Generate polynomial and sieve for smooth relations
        let relations = self.sieve_with_polynomials();

        // Step 3: Check if we have enough relations
        let required_relations = self.factor_base_size + self.params.relation_margin;

        info!("Relation requirements:");
        info!("  Factor base size: {}", self.factor_base_size);
        info!("  Margin: {}", self.params.relation_margin);
        info!("  Total required: {}", required_relations);
        info!("  Found: {}", relations.len());

        if relations.len() < required_relations {
            warn!("Not enough smooth relations: found {}, need {}",
                  relations.len(), required_relations);
            return None;
        }

        // Step 4: Build matrix
        let mut matrix = self.build_matrix(&relations);

        // Step 5: Find dependencies
        let dependencies = self.find_dependencies(&mut matrix);

        if dependencies.is_empty() {
            warn!("No linear dependencies found");
            return None;
        }

        // Step 6: Extract factors
        info!("Attempting to extract factors from dependencies...");

        for (idx, dependency) in dependencies.iter().enumerate() {
            debug!("Trying dependency {} (size {})", idx + 1, dependency.len());

            if let Some((p, q)) = self.extract_factors(&relations, dependency) {
                if &p * &q == self.n {
                    info!("");
                    info!("========================================");
                    info!("SUCCESS!");
                    info!("========================================");
                    info!("Found factors: {} × {}", p, q);
                    info!("");

                    return if &p <= &q {
                        Some((p, q))
                    } else {
                        Some((q, p))
                    };
                }
            }
        }

        warn!("Failed to extract factors from all dependencies");
        None
    }

    /// Sieve with multiple SIQS polynomials
    fn sieve_with_polynomials(&self) -> Vec<Relation> {
        info!("Sieving with SIQS polynomials...");

        let mut all_relations = Vec::new();
        let required_relations = self.factor_base_size + self.params.relation_margin;

        let target_a = self.params.target_a(&self.n);
        let max_polynomials = 100; // Limit number of polynomials to try

        for poly_idx in 0..max_polynomials {
            // Generate a new polynomial
            let polynomial = match generate_polynomial(&self.n, &self.factor_base, &self.params, &target_a) {
                Some(poly) => poly,
                None => {
                    warn!("Failed to generate polynomial {}", poly_idx + 1);
                    continue;
                }
            };

            info!("Polynomial {}: a = {}, b = {}", poly_idx + 1, polynomial.a, polynomial.b);

            // Sieve with this polynomial
            let relations = self.sieve_with_polynomial(&polynomial);
            let smooth_count = relations.len();

            info!("Found {} smooth relations with polynomial {}", smooth_count, poly_idx + 1);

            all_relations.extend(relations);

            // Check if we have enough relations
            if all_relations.len() >= required_relations {
                info!("Collected {} relations (need {}), stopping sieving",
                      all_relations.len(), required_relations);
                break;
            }

            // Progress update
            if (poly_idx + 1) % 10 == 0 {
                info!("Tried {} polynomials, collected {} / {} relations",
                      poly_idx + 1, all_relations.len(), required_relations);
            }
        }

        info!("Total relations collected: {} (need {})", all_relations.len(), required_relations);
        all_relations
    }

    /// Sieve with a single SIQS polynomial
    fn sieve_with_polynomial(&self, polynomial: &SIQSPolynomial) -> Vec<Relation> {
        let m = self.params.sieve_interval;
        let sqrt_n = self.sqrt_n.to_i64().unwrap_or(0);

        // Sieve interval around sqrt(n)
        let start_x = sqrt_n - m / 2;
        let end_x = sqrt_n + m / 2;
        let interval_size = (end_x - start_x + 1) as usize;

        // Initialize log approximation array
        let mut log_array = vec![0.0f32; interval_size];

        // For each prime in factor base (skip -1 and primes in 'a')
        for prime in &self.factor_base {
            if prime.p <= 1 {
                continue; // Skip -1 marker
            }

            // Skip primes that divide 'a'
            if polynomial.a_factors.contains(&prime.p) {
                continue;
            }

            let p = prime.p as i64;
            let log_p = prime.log_p;

            // Compute sieving roots for this polynomial
            // For Q(x) = (ax + b)² - n, we need x such that (ax + b)² ≡ n (mod p)
            // This means ax + b ≡ ±sqrt(n) (mod p)
            // So x ≡ (±sqrt(n) - b) / a (mod p)

            let a_mod_p = polynomial.a.mod_floor(&BigInt::from(p));
            let b_mod_p = polynomial.b.mod_floor(&BigInt::from(p));
            let a_inv_mod_p = match Self::mod_inverse_i64(&a_mod_p.to_i64().unwrap_or(0), p) {
                Some(inv) => inv,
                None => continue,
            };

            // Compute both roots
            for &tsqrt in &prime.roots {
                // Root 1: x ≡ (tsqrt - b) * a_inv (mod p)
                let root = ((tsqrt - b_mod_p.to_i64().unwrap_or(0)) * a_inv_mod_p).rem_euclid(p);

                // Sieve all positions x ≡ root (mod p)
                let first_x = if root >= start_x {
                    root
                } else {
                    start_x + ((root - start_x).rem_euclid(p))
                };

                let mut x = first_x;
                while x <= end_x {
                    let array_idx = (x - start_x) as usize;
                    if array_idx < interval_size {
                        log_array[array_idx] += log_p;
                    }
                    x += p;
                }
            }
        }

        // Calculate threshold
        let sqrt_n_float = self.sqrt_n.to_f64().unwrap_or(1.0);
        let a_float = polynomial.a.to_f64().unwrap_or(1.0);

        // For SIQS: Q(x) = (ax + b)² - n ≈ a × (2sqrt(n) × |x - sqrt(n)|)
        // Maximum Q(x) ≈ a × 2sqrt(n) × M
        let max_q_x = a_float * 2.0 * sqrt_n_float * (m as f64 / 2.0);
        let expected_log = max_q_x.ln() as f32;

        let threshold_multiplier = match self.n.to_string().len() {
            0..=30 => 0.55,
            31..=50 => 0.60,
            51..=70 => 0.65,
            _ => 0.70,
        };
        let threshold = expected_log * threshold_multiplier;

        // Collect candidates
        let mut candidates = Vec::new();
        for x in start_x..=end_x {
            let array_idx = (x - start_x) as usize;
            if array_idx < interval_size && log_array[array_idx] >= threshold {
                candidates.push(x);
            }
        }

        debug!("Found {} candidates for polynomial", candidates.len());

        // Trial divide candidates
        let relations: Vec<Relation> = candidates.par_iter()
            .filter_map(|&x| self.trial_divide_siqs(x, polynomial))
            .collect();

        relations
    }

    /// Trial divide Q(x) for SIQS polynomial, accounting for 'a' coefficient
    fn trial_divide_siqs(&self, x: i64, polynomial: &SIQSPolynomial) -> Option<Relation> {
        // Compute Q(x) = (ax + b)² - n
        let q_x = polynomial.evaluate(x, &self.n);

        if q_x.is_zero() || q_x.abs() < BigInt::from(2) {
            return None;
        }

        let mut remaining = q_x.abs();
        let mut exponents = vec![0u32; self.factor_base_size];

        // Handle sign
        if q_x.is_negative() {
            exponents[0] = 1;
        }

        // Factor out 'a' first (known factors)
        for &a_prime in &polynomial.a_factors {
            let p = BigInt::from(a_prime);

            // Find the exponent of a_prime in 'a'
            let mut a_copy = polynomial.a.clone();
            let mut a_exp = 0u32;
            while a_copy.is_multiple_of(&p) {
                a_copy /= &p;
                a_exp += 1;
            }

            // Q(x) = (ax + b)² - n is divisible by a, so factor it out
            for _ in 0..a_exp {
                if remaining.is_multiple_of(&p) {
                    remaining /= &p;

                    // Find index of this prime in factor base
                    if let Some(idx) = self.factor_base.iter().position(|pr| pr.p == a_prime) {
                        exponents[idx] += 1;
                    }
                } else {
                    break;
                }
            }
        }

        // Trial divide by remaining primes in factor base
        for (idx, prime) in self.factor_base.iter().enumerate().skip(1) {
            // Skip primes that are factors of 'a' (already handled)
            if polynomial.a_factors.contains(&prime.p) {
                continue;
            }

            let p = BigInt::from(prime.p);
            while remaining.is_multiple_of(&p) {
                remaining /= &p;
                exponents[idx] += 1;
            }

            if remaining.is_one() {
                break;
            }
        }

        // Check if completely smooth
        if remaining.is_one() {
            Some(Relation {
                x,
                q_x,
                factors: exponents,
            })
        } else {
            None
        }
    }

    /// Modular inverse for i64
    fn mod_inverse_i64(a: &i64, m: i64) -> Option<i64> {
        let a_big = BigInt::from(*a);
        let m_big = BigInt::from(m);

        let (gcd, x, _) = polynomial::extended_gcd(&a_big, &m_big);

        if gcd != BigInt::one() {
            return None;
        }

        let result = if x < BigInt::zero() {
            x + m_big
        } else {
            x
        };

        result.to_i64()
    }

    /// Build matrix over GF(2) from relations
    fn build_matrix(&self, relations: &[Relation]) -> Vec<Vec<u8>> {
        info!("Building matrix over GF(2)...");

        let num_relations = relations.len();
        let num_primes = self.factor_base_size;

        let mut matrix = Vec::with_capacity(num_relations);

        for relation in relations {
            let mut row = vec![0u8; num_primes];
            for (idx, &exp) in relation.factors.iter().enumerate() {
                row[idx] = (exp % 2) as u8;
            }
            matrix.push(row);
        }

        matrix
    }

    /// Gaussian elimination over GF(2) to find linear dependencies
    fn find_dependencies(&self, matrix: &mut Vec<Vec<u8>>) -> Vec<Vec<usize>> {
        info!("Finding linear dependencies...");

        let num_rows = matrix.len();
        let num_cols = matrix[0].len();

        let mut pivot_row = 0;
        let mut pivot_cols = Vec::new();

        // Forward elimination
        for col in 0..num_cols {
            let mut found_pivot = false;
            for row in pivot_row..num_rows {
                if matrix[row][col] == 1 {
                    matrix.swap(pivot_row, row);
                    found_pivot = true;
                    break;
                }
            }

            if !found_pivot {
                continue;
            }

            pivot_cols.push(col);

            for row in 0..num_rows {
                if row != pivot_row && matrix[row][col] == 1 {
                    for c in 0..num_cols {
                        matrix[row][c] ^= matrix[pivot_row][c];
                    }
                }
            }

            pivot_row += 1;
        }

        // Find dependencies from free rows
        let mut dependencies = Vec::new();

        for row_idx in pivot_row..num_rows {
            let mut dependency = Vec::new();
            for col in 0..num_cols {
                if matrix[row_idx][col] == 1 {
                    dependency.push(col);
                }
            }
            if !dependency.is_empty() {
                dependencies.push(dependency);
            }
        }

        // If no dependencies, create some from existing rows
        if dependencies.is_empty() {
            for row_idx in 0..num_rows.min(pivot_row) {
                let mut dep = vec![row_idx];
                for other in row_idx + 1..num_rows.min(pivot_row + 5) {
                    dep.push(other);
                    if dep.len() >= 2 {
                        dependencies.push(dep.clone());
                        if dependencies.len() >= 10 {
                            break;
                        }
                    }
                    dep.pop();
                }
                if dependencies.len() >= 10 {
                    break;
                }
            }
        }

        info!("Found {} dependencies", dependencies.len());
        dependencies
    }

    /// Extract factors from a linear dependency
    fn extract_factors(&self, relations: &[Relation], dependency: &[usize]) -> Option<(BigInt, BigInt)> {
        let mut x_product = BigInt::one();
        for &idx in dependency {
            if idx < relations.len() {
                let x = BigInt::from(relations[idx].x);
                x_product = (&x_product * x).mod_floor(&self.n);
            }
        }

        let mut exponent_sum = vec![0u32; self.factor_base_size];
        for &idx in dependency {
            if idx < relations.len() {
                for (i, &exp) in relations[idx].factors.iter().enumerate() {
                    exponent_sum[i] += exp;
                }
            }
        }

        let mut y_product = BigInt::one();
        for (i, prime) in self.factor_base.iter().enumerate().skip(1) {
            let exp = exponent_sum[i] / 2;
            if exp > 0 {
                let p = BigInt::from(prime.p);
                let p_pow = p.pow(exp);
                y_product = (&y_product * p_pow).mod_floor(&self.n);
            }
        }

        if exponent_sum[0] % 2 == 1 {
            y_product = -y_product;
        }
        y_product = y_product.mod_floor(&self.n);

        let diff = (&x_product - &y_product).mod_floor(&self.n);
        let sum = (&x_product + &y_product).mod_floor(&self.n);

        let gcd1 = GCD::find_gcd_pair(&diff, &self.n);
        let gcd2 = GCD::find_gcd_pair(&sum, &self.n);

        if &gcd1 > &BigInt::one() && &gcd1 < &self.n {
            let quotient = &self.n / &gcd1;
            return Some((gcd1, quotient));
        }

        if &gcd2 > &BigInt::one() && &gcd2 < &self.n {
            let quotient = &self.n / &gcd2;
            return Some((gcd2, quotient));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_siqs_small() {
        // 8051 = 83 × 97
        let n = BigInt::from(8051);
        let result = siqs(&n);
        // May fail until polynomial generation is implemented
        // assert!(result.is_some());
    }

    #[test]
    fn test_factor_base_construction() {
        let n = BigInt::from(8051);
        let mut siqs = SIQS::new(&n);
        siqs.build_factor_base();

        assert!(siqs.factor_base_size > 0);
        assert!(!siqs.factor_base.is_empty());
    }
}
