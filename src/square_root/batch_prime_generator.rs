// src/square_root/batch_prime_generator.rs

use num::{BigInt, ToPrimitive};

use crate::integer_math::prime_factory::PrimeFactory;

/// Generates consecutive batches of primes starting from a given value.
///
/// The generator keeps track of the last candidate value that was used to
/// request a prime from `PrimeFactory`. Each call to `next_batch` advances the
/// cursor and returns the next `batch_size` primes strictly greater than the
/// last cursor value.
pub struct BatchPrimeGenerator {
    current: i128,
}

impl BatchPrimeGenerator {
    /// Create a new generator. `start_from` is treated as the last candidate
    /// examined, so the first prime returned will be the smallest prime
    /// strictly greater than `start_from`.
    pub fn new(start_from: i128) -> Self {
        BatchPrimeGenerator {
            current: start_from,
        }
    }

    /// Convenience constructor when the starting point is a `BigInt`.
    pub fn from_bigint(start_from: &BigInt) -> Option<Self> {
        start_from.to_i128().map(BatchPrimeGenerator::new)
    }

    /// Generate the next `batch_size` primes. The generator's internal cursor
    /// is updated to the last prime that was produced.
    pub fn next_batch(&mut self, batch_size: usize) -> Vec<BigInt> {
        let mut primes = Vec::with_capacity(batch_size);
        let mut cursor = self.current;

        for _ in 0..batch_size {
            let next_prime = PrimeFactory::get_next_prime_from_i128(cursor);
            let next_prime_i128 = next_prime
                .to_i128()
                .expect("PrimeFactory returned a prime that does not fit in i128");

            cursor = next_prime_i128;
            primes.push(next_prime);
        }

        self.current = cursor;
        primes
    }

    /// Return the last candidate value used to generate primes.
    pub fn last_cursor(&self) -> i128 {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integer_math::prime_factory::PrimeFactory;

    #[test]
    fn generates_strictly_increasing_primes() {
        let mut generator = BatchPrimeGenerator::new(100);
        let batch = generator.next_batch(5);

        assert_eq!(batch.len(), 5);

        for window in batch.windows(2) {
            assert!(
                window[0] < window[1],
                "Primes should be strictly increasing"
            );
        }

        for prime in batch {
            assert!(PrimeFactory::get_next_prime_from_i128(prime.to_i128().unwrap() - 1) >= prime);
        }
    }
}
