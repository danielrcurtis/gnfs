// src/polynomial/field.rs

use super::*;
use num::{BigInt, Zero, One, BigUint, ToPrimitive};
use num::integer::gcd as gcd_bigint;
use std::ops::{Rem, Sub};
use std::cmp::Ordering;
use crate::polynomial::polynomial::Polynomial;
use crate::polynomial::polynomial::Term;

pub fn gcd(left: &Polynomial, right: &Polynomial, modulus: &BigInt) -> Polynomial {
    let mut poly1 = left.clone();
    let mut poly2 = right.clone();

    if poly2.degree() > poly1.degree() {
        std::mem::swap(&mut poly1, &mut poly2);
    }

    while !poly2.terms.is_empty() && poly2.terms.values().any(|coef| coef != &BigInt::zero()) { // Validate that values() works as expected.
        let to_reduce = poly1.clone();
        poly1 = poly2.clone();
        poly2 = mod_mod(&to_reduce, &poly2, modulus);
    }

    if poly1.degree() == 0 {
        Polynomial::one()
    } else {
        poly1
    }
}

pub fn mod_mod(to_reduce: &Polynomial, mod_poly: &Polynomial, prime_modulus: &BigInt) -> Polynomial {
    modulus_bigint(&modulus(to_reduce, mod_poly), prime_modulus)
}

pub fn modulus(poly: &Polynomial, modulus: &Polynomial) -> Polynomial {
    match modulus.cmp(poly) {
        Ordering::Greater => poly.clone(),
        Ordering::Equal => Polynomial::zero(),
        _ => {
            let (_, remainder) = poly.divide(modulus);
            remainder
        }
    }
}

pub fn modulus_bigint(poly: &Polynomial, modulus: &BigInt) -> Polynomial {
    let mut result = poly.clone();
    result.terms = result.terms.iter().map(|(&exp, coef)| {
        let remainder = coef.rem(modulus);
        let new_coef = if remainder.sign() == num::bigint::Sign::Minus {
            remainder + modulus
        } else {
            remainder
        };
        (exp, new_coef)
    }).collect();
    result
}

pub fn divide(left: &Polynomial, right: &Polynomial, modulus: &BigInt) -> (Polynomial, Polynomial) {
    if right.degree() > left.degree() || right > left {
        return (Polynomial::zero(), left.clone());
    }

    let divisor = right[right.degree()].clone() % modulus;
    let mut dividend = left.clone();
    let mut quotient = Polynomial::zero();

    for i in (0..=(left.degree() - right.degree())).rev() {
        let coeff = (&dividend[right.degree() + i] / &divisor) % modulus;
        quotient[i] = coeff.clone();
        dividend[right.degree() + i] = BigInt::zero();

        for j in (i..=(right.degree() + i - 1)).rev() {
            dividend[j] = (&dividend[j] - &coeff * &right[j - i]) % modulus;
        }
    }

    dividend.remove_zeros();
    quotient.remove_zeros();
    (quotient, dividend)
}

pub fn multiply(poly: &Polynomial, multiplier: &BigInt, modulus: &BigInt) -> Polynomial {
    let mut result = poly.clone();
    result.terms = result.terms.iter().map(|(&exp, coef)| {
        if coef != &BigInt::zero() {
            (exp, (coef * multiplier) % modulus)
        } else {
            (exp, coef.clone())
        }
    }).collect();
    result
}

pub fn pow_mod(poly: &Polynomial, exponent: &BigInt, modulus: &BigInt) -> Polynomial {
    let mut result = poly.clone();
    result.terms = result.terms.iter().map(|(&exp, coef)| {
        if coef != &BigInt::zero() {
            let new_coef = coef.modpow(exponent, modulus);
            if new_coef.sign() == num::bigint::Sign::Minus {
                panic!("BigInt::modpow returned negative number");
            }
            (exp, new_coef)
        } else {
            (exp, coef.clone())
        }
    }).collect();
    result
}

pub fn exponentiate_mod(start_poly: &Polynomial, exponent: &BigInt, f: &Polynomial, p: &BigInt) -> Polynomial {
    let mut result = Polynomial::one();
    if exponent == &BigInt::zero() {
        return result;
    }

    let (_, bytes) = exponent.to_bytes_be();
    let bits = BigUint::from_bytes_be(&bytes);
    let mut base = start_poly.clone();
    
    // Iterate through each bit, starting from the most significant bit
    for bit in bits.to_radix_be(2).iter().skip(1) {
        base = mod_mod(&base.square(), f, p);
        if *bit == 1 {
            result = mod_mod(&Polynomial::multiply(&result, &base), f, p);
        }
    }

    result
}


pub fn mod_pow(poly: &Polynomial, exponent: &BigInt, modulus: &Polynomial) -> Polynomial {
    if exponent.sign() == num::bigint::Sign::Minus {
        panic!("Raising a polynomial to a negative exponent is not supported.");
    }

    match exponent {
        e if e == &BigInt::zero() => Polynomial::one(),
        e if e == &BigInt::one() => poly.clone(),
        e if e == &BigInt::from(2) => poly.square(),
        _ => {
            let mut result = poly.square();
            let upper_bound = exponent.sub(&BigInt::from(2)).to_usize().expect("Exponent is too large");
            for _ in 0..upper_bound {
                result = Polynomial::multiply(&result, poly);
                if &result < modulus {
                    result = field::modulus(&result, modulus);
                }
            }
            result
        }
    }
}

pub fn is_irreducible_over_field(f: &Polynomial, p: &BigInt) -> bool {
    let poly = Polynomial::new(vec![Term::new(BigInt::one(), 1), Term::new(-BigInt::one(), 0)]);
    let gcd = gcd(&mod_mod(&poly, f, p), f, p);
    gcd.cmp(&Polynomial::one()) == Ordering::Equal
}

pub fn is_irreducible_over_p(poly: &Polynomial, p: &BigInt) -> bool {
    let mut coefficients: Vec<BigInt> = poly.terms.values().cloned().collect();
    let leading_coeff = coefficients.pop().unwrap();
    let constant_coeff = coefficients.remove(0);

    let leading_remainder = leading_coeff % p;
    let constant_remainder = constant_coeff % p.pow(2);

    let is_monic = leading_remainder != BigInt::zero() && constant_remainder != BigInt::zero();

    coefficients.push(p.clone());
    let gcd = coefficients.iter().fold(coefficients[0].clone(), |a, b| gcd_bigint(a, b.clone()));

    is_monic && gcd == BigInt::one()
}