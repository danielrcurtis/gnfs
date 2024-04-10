// src/square_root/


use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::cmp::Ordering;

pub struct Polynomial {
    // Define the Polynomial struct and its fields
}

impl Polynomial {
    // Implement the necessary methods for Polynomial
}

pub mod finite_field_arithmetic {
    use super::*;

    pub fn square_root(start_polynomial: &Polynomial, f: &Polynomial, p: &BigInt, degree: i32, m: &BigInt) -> Polynomial {
        let q = p.pow(degree as u32);
        let mut s = &q - 1;

        let mut r = 0;
        while s.is_even() {
            s /= 2;
            r += 1;
        }

        let half_s = (&s + 1) / 2;
        let half_s = if r == 1 && q.mod_floor(&BigInt::from(4)) == 3 {
            (&q + 1) / 4
        } else {
            half_s
        };

        let quadratic_non_residue = legendre::symbol_search(&(m + 1), &q, &BigInt::from(-1));
        let theta = quadratic_non_residue;
        let minus_one = theta.modpow(&((&q - 1) / 2), p);

        let mut omega_poly = Polynomial::exponentiate_mod(start_polynomial, &half_s, f, p);

        let mut lambda = minus_one;
        let mut zeta = BigInt::zero();

        let mut i = 0;
        loop {
            i += 1;

            zeta = theta.modpow(&(&i * &s), p);

            lambda = (&lambda * &zeta.pow((2u32.pow((r - i) as u32)) as u32)).mod_floor(p);

            omega_poly = Polynomial::multiply(&omega_poly, &zeta.pow((2u32.pow(((r - i) - 1) as u32)) as u32), p);

            if lambda == BigInt::one() || i > r {
                break;
            }
        }

        omega_poly
    }

    pub fn modular_multiplicative_inverse(a: &BigInt, p: &BigInt) -> BigInt {
        if p == &BigInt::one() {
            return BigInt::zero();
        }

        let mut divisor;
        let mut dividend = a.clone();
        let mut diff = BigInt::zero();
        let mut result = BigInt::one();
        let mut quotient;
        let mut last_divisor;
        let mut remainder = p.clone();

        while dividend > BigInt::one() {
            divisor = remainder.clone();
            quotient = dividend.div_rem(&divisor, &mut remainder).0;
            dividend = divisor;
            last_divisor = diff;

            diff = &result - &(&quotient * &diff);
            result = last_divisor;
        }

        if result < BigInt::zero() {
            result += p;
        }
        result
    }

    pub fn chinese_remainder(primes: &[BigInt], values: &[BigInt]) -> BigInt {
        let prime_product = primes.iter().product();

        let mut z = BigInt::zero();
        for (i, pi) in primes.iter().enumerate() {
            let pj = &prime_product / pi;
            let aj = modular_multiplicative_inverse(pj, pi);
            let ax_pj = &values[i] * &aj * pj;

            z += ax_pj;
        }

        let r = &z / &prime_product;
        let r_p = &r * &prime_product;
        &z - r_p
    }

    pub fn mod_mod(to_reduce: &Polynomial, mod_poly: &Polynomial, prime_modulus: &BigInt) -> Polynomial {
        match mod_poly.cmp(to_reduce) {
            Ordering::Greater => to_reduce.clone(),
            Ordering::Equal => Polynomial::zero(),
            Ordering::Less => remainder(to_reduce, mod_poly, prime_modulus),
        }
    }

    pub fn remainder(left: &Polynomial, right: &Polynomial, mod_: &BigInt) -> Polynomial {
        if right.degree() > left.degree() || right.cmp(left) == Ordering::Greater {
            return Polynomial::zero();
        }

        let right_degree = right.degree();
        let quotient_degree = left.degree() - right_degree + 1;

        let leading_coefficient = right[right_degree].mod_floor(mod_);
        if leading_coefficient != BigInt::one() {
            panic!("This method was expecting only monomials (leading coefficient is 1) for the right-hand-side polynomial.");
        }

        let mut rem = left.clone();
        let mut quot;

        for i in (0..quotient_degree).rev() {
            quot = rem[right_degree + i].mod_floor(mod_);

            rem[right_degree + i] = BigInt::zero();

            for j in (i..(right_degree + i)).rev() {
                rem[j] = (rem[j].clone() - &quot * &right[j - i]).mod_floor(mod_);
            }
        }

        rem
    }
}