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
            factorization.add(&quotient);
        }

        (factorization, quotient)
    }
    
}