// src/integer_math/factorization_factory.rs

use num::{BigInt, One, Zero};
use crate::core::count_dictionary::CountDictionary;

pub struct FactorizationFactory;

impl FactorizationFactory {
    const PRIME_CHECK_BASES: [i64; 15] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];

    pub fn is_probable_prime(input: &BigInt) -> bool {
        if input == &BigInt::from(2) || input == &BigInt::from(3) {
            return true;
        }
        if input < &BigInt::from(2) || input % 2 == BigInt::from(0) {
            return false;
        }

        let mut d = input - 1;
        let mut s = 0;
        while &d % 2 == BigInt::zero() {
            d /= 2;
            s += 1;
        }

        for &a in &Self::PRIME_CHECK_BASES {
            let mut x = BigInt::from(a).modpow(&d, input);
            if x == BigInt::one() || x == input - 1 {
                continue;
            }
            let mut r = 1;
            while r < s {
                x = x.modpow(&BigInt::from(2), input);
                if x == BigInt::one() {
                    return false;
                }
                if x == input - 1 {
                    break;
                }
                r += 1;
            }
            if x != input - 1 {
                return false;
            }
        }
        true
    }

    pub fn factor(input: &BigInt) -> (CountDictionary, BigInt) {
        let mut factorization = CountDictionary::new();
        let mut quotient = input.clone();

        let two = BigInt::from(2);
        while &quotient % &two == BigInt::zero() {
            factorization.add(&two);
            quotient /= &two;
        }

        let mut divisor = BigInt::from(3);
        while divisor.clone() * divisor.clone() <= quotient {
            if &quotient % &divisor == BigInt::zero() {
                factorization.add(&divisor);
                quotient /= &divisor;
            } else {
                divisor += 2;
            }
        }

        if quotient > BigInt::one() {
            // quotient is the remaining unfactored part
            // Don't add it to the factorization
            // Return it as-is so the caller knows there's an unfactored part
        } else {
            // Fully factored, quotient should be 1
            quotient = BigInt::one();
        }

        (factorization, quotient)
    }

    /// Factor a number using only primes from a given factor base.
    /// This is much faster than trial division when the factor base is small.
    ///
    /// Returns:
    /// - CountDictionary: The factorization over the factor base
    /// - BigInt: The unfactored quotient (1 if completely factored, >1 otherwise)
    pub fn factor_with_base(input: &BigInt, factor_base: &[BigInt]) -> (CountDictionary, BigInt) {
        let mut factorization = CountDictionary::new();
        let mut quotient = input.clone();

        // Try to divide by each prime in the factor base
        for prime in factor_base {
            while &quotient % prime == BigInt::zero() {
                factorization.add(prime);
                quotient /= prime;
            }
        }

        (factorization, quotient)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use num::BigInt;

    #[test]
    fn test_factor_with_base_smooth() {
        // Test with a number that factors completely over the base
        // 60 = 2^2 * 3 * 5
        let input = BigInt::from(60);
        let factor_base = vec![BigInt::from(2), BigInt::from(3), BigInt::from(5), BigInt::from(7)];

        let (factorization, quotient) = FactorizationFactory::factor_with_base(&input, &factor_base);

        // Should be completely factored
        assert_eq!(quotient, BigInt::one());

        // Check that we got the right factors
        let dict = factorization.to_dict();
        assert_eq!(dict.get(&BigInt::from(2)), Some(&BigInt::from(2))); // 2^2
        assert_eq!(dict.get(&BigInt::from(3)), Some(&BigInt::from(1))); // 3^1
        assert_eq!(dict.get(&BigInt::from(5)), Some(&BigInt::from(1))); // 5^1
        assert_eq!(dict.get(&BigInt::from(7)), None);                   // 7 not used
    }

    #[test]
    fn test_factor_with_base_not_smooth() {
        // Test with a number that doesn't factor completely
        // 210 = 2 * 3 * 5 * 7, but 7 is not in our base
        let input = BigInt::from(210);
        let factor_base = vec![BigInt::from(2), BigInt::from(3), BigInt::from(5)];

        let (factorization, quotient) = FactorizationFactory::factor_with_base(&input, &factor_base);

        // Quotient should be 7 (the unfactored part)
        assert_eq!(quotient, BigInt::from(7));

        // Check factors
        let dict = factorization.to_dict();
        assert_eq!(dict.get(&BigInt::from(2)), Some(&BigInt::from(1))); // 2^1
        assert_eq!(dict.get(&BigInt::from(3)), Some(&BigInt::from(1))); // 3^1
        assert_eq!(dict.get(&BigInt::from(5)), Some(&BigInt::from(1))); // 5^1
    }

    #[test]
    fn test_factor_with_base_prime() {
        // Test with a prime number
        let input = BigInt::from(13);
        let factor_base = vec![BigInt::from(2), BigInt::from(3), BigInt::from(5), BigInt::from(7)];

        let (factorization, quotient) = FactorizationFactory::factor_with_base(&input, &factor_base);

        // 13 is prime and not in the base, so quotient should be 13
        assert_eq!(quotient, BigInt::from(13));

        // No factors
        assert_eq!(factorization.len(), 0);
    }
}