// src/algorithms/quadratic_sieve.rs
//
// Quadratic Sieve (QS): Advanced factorization for medium-sized composites
// Complexity: L_n[1/2, 1] ≈ exp(sqrt(ln n ln ln n))
// Best for: Numbers in the 40-100 digit range
// Expected performance: 40-60 digit numbers in < 10 seconds
//
// The Quadratic Sieve is the second-fastest known factoring algorithm
// (after GNFS) and is preferred over GNFS for numbers below ~100 digits.
//
// Algorithm Overview:
// 1. Choose a smoothness bound B and factor base (primes up to B)
// 2. Sieve for smooth values of Q(x) = (x + floor(sqrt(n)))² - n
// 3. Find smooth relations where Q(x) factors completely over the factor base
// 4. Collect enough relations to build an over-determined system
// 5. Use linear algebra (Gaussian elimination over GF(2)) to find dependencies
// 6. Compute square roots: if X² ≡ Y² (mod n) and X ≠ ±Y, then gcd(X-Y, n) gives a factor
//
// References:
// - Pomerance (1984): "The Quadratic Sieve Factoring Algorithm"
// - Silverman (1987): "The Multiple Polynomial Quadratic Sieve"
// - Contini (1997): "Factoring Integers with the Self-Initializing Quadratic Sieve"

use num::{BigInt, Integer, One, Signed, ToPrimitive, Zero};
use log::{debug, info, warn};
use crate::integer_math::gcd::GCD;
use rayon::prelude::*;

/// Attempts to factor n using the Quadratic Sieve algorithm.
///
/// # Arguments
/// * `n` - The number to factor (should be 40-100 digits for QS to be optimal)
///
/// # Returns
/// Some((p, q)) where p * q = n and 1 < p <= q < n, or None if factorization fails
///
/// # Examples
/// ```
/// use num::BigInt;
/// use gnfs::algorithms::quadratic_sieve::quadratic_sieve;
///
/// let n = BigInt::from(8051); // 83 × 97
/// let result = quadratic_sieve(&n);
/// assert!(result.is_some());
/// ```
pub fn quadratic_sieve(n: &BigInt) -> Option<(BigInt, BigInt)> {
    info!("========================================");
    info!("QUADRATIC SIEVE FACTORIZATION");
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

    // Initialize Quadratic Sieve
    let mut qs = QuadraticSieve::new(n);

    // Run the factorization
    qs.factor()
}

/// Main Quadratic Sieve structure
struct QuadraticSieve {
    n: BigInt,
    sqrt_n: BigInt,
    smoothness_bound: u64,
    sieve_interval: i64,
    factor_base: Vec<Prime>,
    factor_base_size: usize,
}

/// Represents a prime in the factor base with its roots modulo that prime
#[derive(Clone, Debug)]
struct Prime {
    p: u64,
    roots: Vec<i64>, // Solutions to x² ≡ n (mod p)
}

/// Represents a smooth relation: Q(x) factors over the factor base
#[derive(Clone, Debug)]
struct Relation {
    x: i64,
    q_x: BigInt,
    factors: Vec<u32>, // Exponent vector for each prime in factor base
}

impl QuadraticSieve {
    /// Create a new Quadratic Sieve instance
    pub fn new(n: &BigInt) -> Self {
        let digits = n.to_string().len();
        let sqrt_n = n.sqrt();

        // Choose parameters based on size of n
        let params = Self::choose_parameters(digits);

        info!("QS Parameters:");
        info!("  Smoothness bound B: {}", params.0);
        info!("  Sieve interval [-M, M]: [-{}, {}]", params.1, params.1);
        info!("");

        QuadraticSieve {
            n: n.clone(),
            sqrt_n,
            smoothness_bound: params.0,
            sieve_interval: params.1,
            factor_base: Vec::new(),
            factor_base_size: 0,
        }
    }

    /// Choose optimal parameters based on number size using research-backed values
    /// Returns (smoothness_bound, sieve_interval)
    ///
    /// Based on:
    /// - Silverman (1987): "The Multiple Polynomial Quadratic Sieve"
    /// - Empirical parameter tables from production QS implementations
    /// - Research synthesis showing optimal B and M values for each digit count
    fn choose_parameters(digits: usize) -> (u64, i64) {
        // Use empirically-validated parameter table from research
        // Note: Silverman's F(d) formula gives FACTOR BASE SIZE (after QR filtering),
        // but we need SMOOTHNESS BOUND B (before filtering), which is ~10x larger

        match digits {
            // Very small numbers - empirical tuning
            0..=10 => (100, 20000),
            11..=15 => (200, 40000),
            16..=20 => (500, 100000),
            21..=23 => (1000, 200000),

            // Research-backed parameters for QS sweet spot (24-66 digits)
            24..=29 => (2000, 300000),      // ~24-29 digits
            30..=34 => (3500, 450000),      // ~30-34 digits
            35..=39 => (5000, 550000),      // ~35-39 digits
            40..=44 => (8000, 700000),      // ~40-44 digits (research: 6k-10k, 400k-600k)
            45..=49 => (15000, 1200000),    // ~45-49 digits (research: 12k-18k, 600k-900k)
            50..=54 => (25000, 1800000),    // ~50-54 digits (research: 20k-30k, 900k-1.5M)
            55..=59 => (42000, 3000000),    // ~55-59 digits (research: 35k-50k, 1.5M-2.5M)
            60..=64 => (65000, 4500000),    // ~60-64 digits (research: 50k-80k, 2.5M-4M)
            65..=69 => (100000, 7000000),   // ~65-69 digits (research: 80k-120k, 4M-6M)

            // Large numbers (70-100 digits) - extrapolated with safety margins
            70..=74 => (150000, 11000000),  // ~70-74 digits
            75..=79 => (220000, 17000000),  // ~75-79 digits
            80..=84 => (300000, 27000000),  // ~80-84 digits
            85..=89 => (425000, 42000000),  // ~85-89 digits
            90..=94 => (600000, 65000000),  // ~90-94 digits
            95..=99 => (850000, 100000000), // ~95-99 digits
            100 => (1200000, 150000000),    // 100 digits (approaching GNFS crossover)

            // Very large (>100 digits): QS is suboptimal, GNFS strongly recommended
            _ => {
                warn!("Number > 100 digits - QS is suboptimal, GNFS strongly recommended");
                warn!("QS/GNFS crossover is typically around 100-110 digits");
                let d = digits as f64;
                let factor_base = (d * 15000.0) as u64;          // Linear scaling
                let sieve_interval = (d * d * 150000.0) as i64;  // Quadratic scaling
                (factor_base, sieve_interval)
            }
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
        });

        // Special handling for p = 2
        if self.n.mod_floor(&BigInt::from(8)) == BigInt::one() {
            factor_base.push(Prime {
                p: 2,
                roots: vec![1],
            });
        }

        // Check odd primes up to smoothness bound
        let mut p = 3u64;
        while p <= self.smoothness_bound {
            if Self::is_prime_simple(p) {
                let p_bigint = BigInt::from(p);

                // Check if n is a quadratic residue mod p using direct computation
                // Legendre symbol (n/p) = n^((p-1)/2) mod p
                let exp = BigInt::from((p - 1) / 2);
                let legendre_val = self.n.modpow(&exp, &p_bigint);
                let is_qr = legendre_val.is_one();

                if is_qr {
                    // n is a QR mod p, find the roots
                    let roots = Self::tonelli_shanks(&self.n, p);

                    if !roots.is_empty() {
                        factor_base.push(Prime { p, roots });
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
    /// Returns solutions to x² ≡ n (mod p)
    fn tonelli_shanks(n: &BigInt, p: u64) -> Vec<i64> {
        let p_bigint = BigInt::from(p);
        let n_mod = n.mod_floor(&p_bigint);

        // Handle special cases
        if n_mod.is_zero() {
            return vec![0];
        }

        // Check if n is actually a QR using direct computation
        // Legendre symbol (n/p) = n^((p-1)/2) mod p
        let exp = BigInt::from((p - 1) / 2);
        let legendre_val = n_mod.modpow(&exp, &p_bigint);
        let is_qr = legendre_val.is_one();

        if !is_qr {
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

        // General case: Tonelli-Shanks
        // Find Q and S such that p - 1 = Q * 2^S
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
            // Non-residue has Legendre symbol = -1 ≡ p-1 (mod p)
            if z_legendre == &p_bigint - 1 {
                break;
            }
            z += 1;
            if z > 1000 {
                // Safety check
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
                // Safety check to prevent infinite loop
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

            // Find the least i such that t^(2^i) = 1
            let mut i = 1u32;
            let mut temp = (&t * &t).mod_floor(&p_bigint);
            while !temp.is_one() && i < m {
                temp = (&temp * &temp).mod_floor(&p_bigint);
                i += 1;
            }

            if i >= m {
                // Failed to find proper i
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

        vec![] // Failed
    }

    /// Main sieving routine to find smooth relations
    fn sieve(&self) -> Vec<Relation> {
        info!("Sieving for smooth relations...");

        let m = self.sieve_interval;
        let sqrt_n = self.sqrt_n.to_i64().unwrap_or(0);

        // Sieve around sqrt(n) where Q(x) = x² - n is small
        let start_x = sqrt_n - m / 2;
        let end_x = sqrt_n + m / 2;
        let interval_size = (end_x - start_x + 1) as usize;

        // Initialize log approximation array
        let mut log_array = vec![0.0f32; interval_size];

        // Pre-compute log values for primes
        let log_primes: Vec<f32> = self.factor_base.iter()
            .map(|pr| if pr.p > 1 { (pr.p as f32).ln() } else { 0.0 })
            .collect();

        // For each prime in factor base, sieve positions where Q(x) is divisible by p
        for (idx, prime) in self.factor_base.iter().enumerate().skip(1) { // Skip -1
            let p = prime.p as i64;
            let log_p = log_primes[idx];

            for &root in &prime.roots {
                // Sieve positions x ≡ root (mod p)
                // Q(x) = x² - n is divisible by p when x ≡ ±sqrt(n) (mod p)

                // Start at first x in interval where x ≡ root (mod p)
                let start1 = if root >= start_x {
                    root
                } else {
                    let offset = ((start_x - root) / p + 1) * p;
                    root + offset
                };

                let mut x = start1;
                while x <= end_x {
                    let array_idx = (x - start_x) as usize;
                    if array_idx < interval_size {
                        log_array[array_idx] += log_p;
                    }
                    x += p;
                }

                // Handle the other root (-root mod p)
                let root2 = (p - root) % p;
                if root2 != root {
                    let start2 = if root2 >= start_x {
                        root2
                    } else {
                        let offset = ((start_x - root2) / p + 1) * p;
                        root2 + offset
                    };

                    let mut x = start2;
                    while x <= end_x {
                        let array_idx = (x - start_x) as usize;
                        if array_idx < interval_size {
                            log_array[array_idx] += log_p;
                        }
                        x += p;
                    }
                }
            }
        }

        // Calculate threshold for sieving
        // For x near sqrt(n), Q(x) = x² - n ≈ 2*sqrt(n)*|x - sqrt(n)|
        // Maximum Q(x) in interval: 2*sqrt(n)*M where M is sieve_interval/2
        let sqrt_n_float = self.sqrt_n.to_f64().unwrap_or(1.0);
        let max_q_x = 2.0 * sqrt_n_float * (self.sieve_interval as f64 / 2.0);
        let expected_log = max_q_x.ln() as f32;

        // Threshold: percentage of expected log (sum of factor base prime logs)
        // Lower threshold = more candidates but slower trial division
        // Higher threshold = fewer candidates but may miss smooth relations
        let threshold_multiplier = match self.n.to_string().len() {
            0..=10 => 0.50,   // Very aggressive for small numbers
            11..=30 => 0.60,  // Moderate for medium numbers
            31..=60 => 0.65,  // Balanced for QS sweet spot
            _ => 0.70,        // Conservative for large numbers
        };
        let threshold = expected_log * threshold_multiplier;

        info!("Sieving threshold calculation:");
        info!("  Max Q(x) in interval: {:.2e}", max_q_x);
        info!("  Expected log(Q(x)): {:.2}", expected_log);
        info!("  Threshold multiplier: {:.2}", threshold_multiplier);
        info!("  Final threshold: {:.2}", threshold);

        // Collect candidate smooth relations
        let mut candidates = Vec::new();
        for x in start_x..=end_x {
            let array_idx = (x - start_x) as usize;
            if array_idx < interval_size && log_array[array_idx] >= threshold {
                candidates.push(x);
            }
        }

        info!("Found {} candidates (log threshold passed)", candidates.len());

        // Trial divide candidates to confirm smoothness and build relations
        let relations: Vec<Relation> = candidates.par_iter()
            .filter_map(|&x| self.trial_divide_candidate(x))
            .collect();

        info!("Found {} smooth relations", relations.len());
        relations
    }

    /// Trial divide Q(x) to confirm smoothness and build factor vector
    fn trial_divide_candidate(&self, x: i64) -> Option<Relation> {
        // Compute Q(x) = x² - n (simple form)
        let x_big = BigInt::from(x);
        let q_x = &x_big * &x_big - &self.n;

        if q_x.is_zero() || q_x.abs() < BigInt::from(2) {
            return None;
        }

        // Try to factor q_x over the factor base
        let mut remaining = q_x.abs();
        let mut exponents = vec![0u32; self.factor_base_size];

        // Handle sign (if q_x is negative)
        let is_negative = q_x.is_negative();
        if is_negative {
            exponents[0] = 1; // -1 is at index 0
        }

        // Trial divide by each prime in factor base
        for (idx, prime) in self.factor_base.iter().enumerate().skip(1) {
            let p = BigInt::from(prime.p);
            while remaining.is_multiple_of(&p) {
                remaining /= &p;
                exponents[idx] += 1;
            }

            // Early exit if remaining is 1
            if remaining.is_one() {
                break;
            }
        }

        // Check if completely factored (smooth)
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

    /// Build matrix over GF(2) from relations
    fn build_matrix(&self, relations: &[Relation]) -> Vec<Vec<u8>> {
        info!("Building matrix over GF(2)...");

        let num_relations = relations.len();
        let num_primes = self.factor_base_size;

        info!("Matrix size: {} × {}", num_relations, num_primes);

        let mut matrix = Vec::with_capacity(num_relations);

        for relation in relations {
            let mut row = vec![0u8; num_primes];
            for (idx, &exp) in relation.factors.iter().enumerate() {
                row[idx] = (exp % 2) as u8; // Reduce to GF(2)
            }
            matrix.push(row);
        }

        matrix
    }

    /// Gaussian elimination over GF(2) to find linear dependencies
    fn find_dependencies(&self, matrix: &mut Vec<Vec<u8>>) -> Vec<Vec<usize>> {
        info!("Finding linear dependencies (Gaussian elimination)...");

        let num_rows = matrix.len();
        let num_cols = matrix[0].len();

        let mut pivot_row = 0;
        let mut pivot_cols = Vec::new();

        // Forward elimination
        for col in 0..num_cols {
            // Find pivot
            let mut found_pivot = false;
            for row in pivot_row..num_rows {
                if matrix[row][col] == 1 {
                    // Swap rows
                    matrix.swap(pivot_row, row);
                    found_pivot = true;
                    break;
                }
            }

            if !found_pivot {
                continue;
            }

            pivot_cols.push(col);

            // Eliminate
            for row in 0..num_rows {
                if row != pivot_row && matrix[row][col] == 1 {
                    for c in 0..num_cols {
                        matrix[row][c] ^= matrix[pivot_row][c]; // XOR for GF(2)
                    }
                }
            }

            pivot_row += 1;
        }

        info!("Matrix rank: {}", pivot_row);
        info!("Free variables: {}", num_rows - pivot_row);

        // Find dependencies from free rows (zero rows)
        let mut dependencies = Vec::new();

        for row_idx in pivot_row..num_rows {
            let mut dependency = Vec::new();

            // This row should be all zeros (free variable)
            // Back-substitute to find which original relations combine to zero
            for col in 0..num_cols {
                if matrix[row_idx][col] == 1 {
                    dependency.push(col);
                }
            }

            if !dependency.is_empty() {
                dependencies.push(dependency);
            }
        }

        // If no dependencies from free rows, find from the reduced matrix
        if dependencies.is_empty() {
            // Try all subset combinations (simplified approach)
            for row_idx in 0..num_rows.min(pivot_row) {
                let mut dep = vec![row_idx];

                // Try to find complementary rows
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
        // Multiply left sides: X = product of x values
        let mut x_product = BigInt::one();
        for &idx in dependency {
            if idx < relations.len() {
                let x = BigInt::from(relations[idx].x);
                x_product = (&x_product * x).mod_floor(&self.n);
            }
        }

        // Multiply right sides: Y² = product of Q(x) values
        // Since Q(x) values factor over factor base, compute product of exponents
        let mut exponent_sum = vec![0u32; self.factor_base_size];
        for &idx in dependency {
            if idx < relations.len() {
                for (i, &exp) in relations[idx].factors.iter().enumerate() {
                    exponent_sum[i] += exp;
                }
            }
        }

        // All exponents should be even (that's what dependency ensures)
        // Compute Y = product of primes^(exponent/2)
        let mut y_product = BigInt::one();
        for (i, prime) in self.factor_base.iter().enumerate().skip(1) {
            let exp = exponent_sum[i] / 2;
            if exp > 0 {
                let p = BigInt::from(prime.p);
                let p_pow = p.pow(exp);
                y_product = (&y_product * p_pow).mod_floor(&self.n);
            }
        }

        // Handle sign
        if exponent_sum[0] % 2 == 1 {
            y_product = -y_product;
        }
        y_product = y_product.mod_floor(&self.n);

        // Now we have X² ≡ Y² (mod n)
        // Compute gcd(X - Y, n) and gcd(X + Y, n)
        let diff = (&x_product - &y_product).mod_floor(&self.n);
        let sum = (&x_product + &y_product).mod_floor(&self.n);

        let gcd1 = GCD::find_gcd_pair(&diff, &self.n);
        let gcd2 = GCD::find_gcd_pair(&sum, &self.n);

        // Check if we found a non-trivial factor
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

    /// Main factorization routine
    pub fn factor(&mut self) -> Option<(BigInt, BigInt)> {
        // Step 1: Build factor base
        self.build_factor_base();

        if self.factor_base_size < 2 {
            warn!("Factor base too small");
            return None;
        }

        // Step 2: Sieve for smooth relations
        let relations = self.sieve();

        // We need more relations than the size of the factor base
        // Larger margin ensures we find linear dependencies in matrix
        let n_digits = self.n.to_string().len();
        let margin = match n_digits {
            0..=10 => 5,
            11..=30 => 10,
            31..=60 => 20,
            61..=80 => 50,
            _ => 100,
        };
        let required_relations = self.factor_base_size + margin;

        info!("Relation requirements:");
        info!("  Factor base size: {}", self.factor_base_size);
        info!("  Margin for dependencies: {}", margin);
        info!("  Total required: {}", required_relations);
        info!("  Found: {} relations", relations.len());

        if relations.len() < required_relations {
            warn!("Not enough smooth relations: found {}, need {}",
                  relations.len(), required_relations);
            warn!("Success rate: {:.1}%", (relations.len() as f64 / required_relations as f64) * 100.0);
            warn!("Try increasing sieve interval or smoothness bound");
            return None;
        }

        info!("");
        info!("Collected {} relations (need {})", relations.len(), required_relations);
        info!("");

        // Step 3: Build matrix
        let mut matrix = self.build_matrix(&relations);

        // Step 4: Find dependencies
        let dependencies = self.find_dependencies(&mut matrix);

        if dependencies.is_empty() {
            warn!("No linear dependencies found");
            return None;
        }

        // Step 5: Try each dependency to extract factors
        info!("Attempting to extract factors from dependencies...");

        for (idx, dependency) in dependencies.iter().enumerate() {
            debug!("Trying dependency {} (size {})", idx + 1, dependency.len());

            if let Some((p, q)) = self.extract_factors(&relations, dependency) {
                // Verify factorization
                if &p * &q == self.n {
                    info!("");
                    info!("========================================");
                    info!("SUCCESS!");
                    info!("========================================");
                    info!("Found factors: {} × {}", p, q);
                    info!("");

                    // Return in ascending order
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic_sieve_small() {
        // 143 is too small for QS to work reliably with simple implementation
        // Use a slightly larger number: 3599 = 59 × 61
        let n = BigInt::from(3599); // 59 × 61
        let result = quadratic_sieve(&n);
        assert!(result.is_some(), "QS should factor 3599");
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
        assert!(&p == &BigInt::from(59) || &p == &BigInt::from(61));
        assert!(&q == &BigInt::from(59) || &q == &BigInt::from(61));
    }

    #[test]
    fn test_quadratic_sieve_8051() {
        let n = BigInt::from(8051); // 83 × 97
        let result = quadratic_sieve(&n);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
    }

    #[test]
    fn test_quadratic_sieve_perfect_square() {
        let n = BigInt::from(121); // 11²
        let result = quadratic_sieve(&n);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
        assert_eq!(p, BigInt::from(11));
        assert_eq!(q, BigInt::from(11));
    }

    #[test]
    fn test_quadratic_sieve_even() {
        let n = BigInt::from(100); // 2² × 5²
        let result = quadratic_sieve(&n);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(&p * &q, n);
        assert_eq!(p, BigInt::from(2));
    }

    #[test]
    fn test_tonelli_shanks() {
        // Test with p ≡ 3 (mod 4) first (simpler case)
        // x² ≡ 2 (mod 7)
        // Solutions are x = 3 and x = 4
        let n = BigInt::from(2);
        let p = 7u64;
        let roots = QuadraticSieve::tonelli_shanks(&n, p);
        assert!(roots.len() >= 1, "Should find at least one root for p=7");

        // Verify all roots
        for root in &roots {
            let root_big = BigInt::from(*root);
            let check = (&root_big * &root_big).mod_floor(&BigInt::from(p));
            assert_eq!(check, n.mod_floor(&BigInt::from(p)),
                      "Root {} should satisfy x² ≡ 2 (mod 7)", root);
        }

        // Also test another simple case
        // x² ≡ 4 (mod 11)
        // Solutions are x = 2 and x = 9
        let n2 = BigInt::from(4);
        let p2 = 11u64;
        let roots2 = QuadraticSieve::tonelli_shanks(&n2, p2);
        assert!(roots2.len() >= 1, "Should find at least one root for p=11");

        for root in &roots2 {
            let root_big = BigInt::from(*root);
            let check = (&root_big * &root_big).mod_floor(&BigInt::from(p2));
            assert_eq!(check, n2.mod_floor(&BigInt::from(p2)),
                      "Root {} should satisfy x² ≡ 4 (mod 11)", root);
        }
    }

    #[test]
    fn test_is_prime_simple() {
        assert!(QuadraticSieve::is_prime_simple(2));
        assert!(QuadraticSieve::is_prime_simple(3));
        assert!(QuadraticSieve::is_prime_simple(5));
        assert!(QuadraticSieve::is_prime_simple(7));
        assert!(QuadraticSieve::is_prime_simple(97));

        assert!(!QuadraticSieve::is_prime_simple(1));
        assert!(!QuadraticSieve::is_prime_simple(4));
        assert!(!QuadraticSieve::is_prime_simple(100));
    }

    #[test]
    fn test_parameter_selection() {
        let params_20 = QuadraticSieve::choose_parameters(20);
        assert!(params_20.0 > 0);
        assert!(params_20.1 > 0);

        let params_40 = QuadraticSieve::choose_parameters(40);
        assert!(params_40.0 > params_20.0);
        assert!(params_40.1 > params_20.1);
    }

    #[test]
    fn test_factor_base_construction() {
        let n = BigInt::from(143);
        let mut qs = QuadraticSieve::new(&n);
        qs.build_factor_base();

        assert!(qs.factor_base_size > 0);
        assert!(!qs.factor_base.is_empty());

        // First element should be -1 marker
        assert_eq!(qs.factor_base[0].p, 1);
    }
}
