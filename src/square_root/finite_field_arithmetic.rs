// src/square_root/

use num::BigInt;
use num::Integer;
use num::{One, Zero};
use std::cmp::Ordering;
use crate::integer_math::legendre::Legendre;
use crate::polynomial::polynomial::Polynomial;

pub fn square_root(start_polynomial: &Polynomial, f: &Polynomial, p: &BigInt, degree: i32, m: &BigInt) -> Polynomial {
    let q = p.pow(degree as u32);
    let mut s: BigInt = &q - 1;

    let mut r = 0;
    while s.is_even() {
        s /= 2;
        r += 1;
    }

    let half_s = (&s + 1) / 2;
    let half_s = if r == 1 && q.mod_floor(&BigInt::from(4)) == BigInt::from(3) {
        (&q + 1) / 4
    } else {
        half_s
    };

    let quadratic_non_residue = Legendre::symbol_search(&(m + 1), &q, &BigInt::from(-1));
    let theta = quadratic_non_residue;
    let minus_one = theta.modpow(&((&q - 1) / 2), p);

    let mut omega_poly = Polynomial::exponentiate_mod(start_polynomial, &half_s, f, p);

    let mut lambda = minus_one;

    let mut i = 0;
    loop {
        i += 1;

        let zeta = theta.modpow(&(&i * &s), p);  // Declare zeta here since it's used right after

        lambda = (&lambda * &zeta.pow((2u32.pow((r - i) as u32)) as u32)).mod_floor(p);

        omega_poly = Polynomial::multiply(&omega_poly, &Polynomial::from_term(zeta.pow(2u32.pow((r - i - 1) as u32) as u32), 0));

        if lambda == BigInt::one() || i > r {
            break;
        }
    }

    omega_poly
}

pub fn modular_multiplicative_inverse(a: &BigInt, p: &BigInt) -> Option<BigInt> {
    if p == &BigInt::one() {
        return Some(BigInt::zero());
    }

    if a.gcd(p) != BigInt::one() {
        return None;
    }

    let mut dividend = a.clone();
    let mut divisor = p.clone();
    let mut result = BigInt::zero();
    let mut last_result = BigInt::one();
    let mut temp_result;

    while divisor > BigInt::zero() {
        let (quotient, remainder) = dividend.div_rem(&divisor);
        dividend = divisor;
        divisor = remainder;
        temp_result = result.clone();
        result = last_result - &quotient * &temp_result;
        last_result = temp_result;
    }

    if last_result < BigInt::zero() {
        last_result += p;
    }

    Some(last_result)
}

pub fn chinese_remainder(primes: &[BigInt], values: &[BigInt]) -> Option<BigInt> {
    let prime_product: BigInt = primes.iter().product();
    let mut z = BigInt::zero();

    for (i, pi) in primes.iter().enumerate() {
        let pj = &prime_product / pi;
        let aj = modular_multiplicative_inverse(&pj, &pi)?; // Use ? to handle the Option
        let ax_pj = &values[i] * &aj * pj;
        z += ax_pj;
    }

    let r = &z / &prime_product;
    let r_p = &r * &prime_product;
    Some(&z - r_p) // Return the result wrapped in Some
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
