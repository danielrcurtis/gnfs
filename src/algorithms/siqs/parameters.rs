// src/algorithms/siqs/parameters.rs
//
// Parameter selection for SIQS based on number size

use num::BigInt;

/// SIQS parameter configuration
#[derive(Clone, Debug)]
pub struct SIQSParameters {
    pub smoothness_bound: u64,     // B: maximum prime in factor base
    pub sieve_interval: i64,       // M: half-width of sieve interval [-M, M]
    pub primes_per_a: usize,       // j: number of primes in 'a' coefficient
    pub relation_margin: usize,    // Extra relations beyond factor base size
}

impl SIQSParameters {
    /// Choose optimal parameters based on number size
    ///
    /// Based on research from:
    /// - Contini (1997): "Factoring Integers with the Self-Initializing Quadratic Sieve"
    /// - Silverman (1987): "The Multiple Polynomial Quadratic Sieve"
    /// - Empirical tables from production QS implementations
    pub fn from_number_size(n: &BigInt) -> Self {
        let digits = n.to_string().len();

        // Match digits to optimal parameters
        // Format: (B, M, j, margin)
        let (smoothness_bound, sieve_interval, primes_per_a, relation_margin) = match digits {
            // Small numbers (< 40 digits) - not optimal for SIQS but supported
            0..=10 => (100, 20000, 3, 10),
            11..=20 => (500, 100000, 3, 15),
            21..=30 => (2000, 300000, 3, 20),
            31..=39 => (5000, 550000, 4, 25),

            // SIQS sweet spot (40-100 digits)
            40..=44 => (8000, 700000, 4, 30),
            45..=49 => (15000, 1200000, 4, 40),
            50..=54 => (25000, 1800000, 4, 50),
            55..=59 => (42000, 3000000, 5, 60),
            60..=64 => (65000, 4500000, 5, 75),
            65..=69 => (100000, 7000000, 5, 100),
            70..=74 => (150000, 11000000, 5, 125),
            75..=79 => (220000, 17000000, 5, 150),
            80..=84 => (300000, 27000000, 6, 200),
            85..=89 => (425000, 42000000, 6, 250),
            90..=94 => (600000, 65000000, 6, 300),
            95..=99 => (850000, 100000000, 6, 400),
            100 => (1200000, 150000000, 6, 500),

            // Very large (> 100 digits): GNFS recommended
            _ => {
                let d = digits as f64;
                let smoothness_bound = (d * 15000.0) as u64;
                let sieve_interval = (d * d * 150000.0) as i64;
                let primes_per_a = 6;
                let relation_margin = (d * 5.0) as usize;
                (smoothness_bound, sieve_interval, primes_per_a, relation_margin)
            }
        };

        SIQSParameters {
            smoothness_bound,
            sieve_interval,
            primes_per_a,
            relation_margin,
        }
    }

    /// Get the target 'a' coefficient value
    ///
    /// For SIQS, a ≈ sqrt(2n) / M
    /// This ensures Q(x) values are approximately M * sqrt(n), which is optimal for smoothness
    pub fn target_a(&self, n: &BigInt) -> BigInt {
        let two_n: BigInt = n * 2;
        let sqrt_2n = two_n.sqrt();
        let m_big = BigInt::from(self.sieve_interval);

        // a ≈ sqrt(2n) / M
        if m_big > BigInt::from(0) {
            &sqrt_2n / &m_big
        } else {
            sqrt_2n
        }
    }

    /// Get the prime range for selecting primes in 'a'
    ///
    /// Primes should be from the middle of the factor base to ensure good distribution
    /// Typical range: [B/10, B/3] for 40-digit numbers
    pub fn a_prime_range(&self) -> (u64, u64) {
        let lower = self.smoothness_bound / 10;
        let upper = self.smoothness_bound / 3;
        (lower.max(100), upper.max(200))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_selection() {
        // Test 40-digit number
        let n40 = BigInt::from(10u64).pow(40);
        let params40 = SIQSParameters::from_number_size(&n40);
        assert_eq!(params40.smoothness_bound, 8000);
        assert_eq!(params40.sieve_interval, 700000);
        assert_eq!(params40.primes_per_a, 4);

        // Test 50-digit number
        let n50 = BigInt::from(10u64).pow(50);
        let params50 = SIQSParameters::from_number_size(&n50);
        assert_eq!(params50.smoothness_bound, 25000);
        assert_eq!(params50.sieve_interval, 1800000);
    }

    #[test]
    fn test_target_a() {
        let n = BigInt::from(10u64).pow(40);
        let params = SIQSParameters::from_number_size(&n);
        let target_a = params.target_a(&n);

        // For 40-digit n with M=700000:
        // sqrt(2n) ≈ 4.47e20
        // target_a ≈ 4.47e20 / 700000 ≈ 6.4e14
        assert!(target_a > BigInt::from(1e14 as u64));
        assert!(target_a < BigInt::from(1e16 as u64));
    }

    #[test]
    fn test_a_prime_range() {
        let params = SIQSParameters {
            smoothness_bound: 8000,
            sieve_interval: 700000,
            primes_per_a: 4,
            relation_margin: 30,
        };

        let (lower, upper) = params.a_prime_range();
        assert_eq!(lower, 800);  // B/10
        assert_eq!(upper, 2666);  // B/3
    }
}
