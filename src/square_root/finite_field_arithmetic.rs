// src/square_root/

use num::BigInt;
use num::Integer;
use num::{One, Zero};
use std::cmp::Ordering;
use std::time::Instant;
use log::info;
use crate::integer_math::legendre::Legendre;
use crate::polynomial::polynomial::Polynomial;

pub fn square_root(start_polynomial: &Polynomial, f: &Polynomial, p: &BigInt, degree: i32, m: &BigInt) -> Polynomial {
    let function_start = Instant::now();
    info!("square_root() ENTRY: p={}, degree={}, m={}", p, degree, m);
    info!("  start_polynomial degree: {}, f degree: {}", start_polynomial.degree(), f.degree());

    // Line 11: q = p.pow(degree)
    let q_start = Instant::now();
    let q = p.pow(degree as u32);
    let q_elapsed = q_start.elapsed();
    info!("  q = p.pow(degree) took: {:.3?}", q_elapsed);
    info!("  q value: {}", q);

    let mut s: BigInt = &q - 1;

    let mut r = 0;
    while s.is_even() {
        s /= 2;
        r += 1;
    }
    info!("  r={}, s={}", r, s);

    let half_s = (&s + 1) / 2;
    let half_s = if r == 1 && q.mod_floor(&BigInt::from(4)) == BigInt::from(3) {
        (&q + 1) / 4
    } else {
        half_s
    };
    info!("  half_s={}", half_s);

    // Line 27: Legendre::symbol_search()
    let legendre_start = Instant::now();
    let quadratic_non_residue = Legendre::symbol_search(&(m + 1), &q, &BigInt::from(-1));
    let legendre_elapsed = legendre_start.elapsed();
    info!("  Legendre::symbol_search() took: {:.3?}", legendre_elapsed);

    let theta = quadratic_non_residue;

    let minus_one_start = Instant::now();
    let minus_one = theta.modpow(&((&q - 1) / 2), p);
    let minus_one_elapsed = minus_one_start.elapsed();
    info!("  theta.modpow() (minus_one) took: {:.3?}", minus_one_elapsed);

    // Line 31: Polynomial::exponentiate_mod() - OPTIMIZED WITH WINDOWED METHOD + KARATSUBA
    let exp_mod_start = Instant::now();
    use crate::polynomial::optimized_exp::windowed_exponentiate_mod;
    let mut omega_poly = windowed_exponentiate_mod(start_polynomial, &half_s, f, p, 4);
    let exp_mod_elapsed = exp_mod_start.elapsed();
    info!("  OPTIMIZED windowed_exponentiate_mod() took: {:.3?} *** TIER 1 OPTIMIZATIONS (windowed + Karatsuba + eager reduction) ***", exp_mod_elapsed);

    let mut lambda = minus_one;

    // Loop timing (lines 36-48)
    let loop_start = Instant::now();
    let mut i = 0;
    let mut total_zeta_time = std::time::Duration::ZERO;
    let mut total_lambda_time = std::time::Duration::ZERO;
    let mut total_multiply_time = std::time::Duration::ZERO;

    loop {
        i += 1;
        let iteration_start = Instant::now();

        // Line 39: theta.modpow()
        let zeta_start = Instant::now();
        let zeta = theta.modpow(&(&i * &s), p);
        let zeta_elapsed = zeta_start.elapsed();
        total_zeta_time += zeta_elapsed;

        // Line 41: lambda update
        let lambda_start = Instant::now();
        lambda = (&lambda * &zeta.pow((2u32.pow((r - i) as u32)) as u32)).mod_floor(p);
        let lambda_elapsed = lambda_start.elapsed();
        total_lambda_time += lambda_elapsed;

        // Line 43: Polynomial::multiply()
        let multiply_start = Instant::now();
        omega_poly = Polynomial::multiply(&omega_poly, &Polynomial::from_term(zeta.pow(2u32.pow((r - i - 1) as u32) as u32), 0));
        let multiply_elapsed = multiply_start.elapsed();
        total_multiply_time += multiply_elapsed;

        let iteration_elapsed = iteration_start.elapsed();
        info!("    Loop iteration {}: total={:.3?}, zeta={:.3?}, lambda={:.3?}, multiply={:.3?}",
              i, iteration_elapsed, zeta_elapsed, lambda_elapsed, multiply_elapsed);

        if lambda == BigInt::one() || i > r {
            break;
        }
    }

    let loop_elapsed = loop_start.elapsed();
    info!("  Loop completed: {} iterations, total_time={:.3?}", i, loop_elapsed);
    info!("    Loop breakdown: zeta_total={:.3?}, lambda_total={:.3?}, multiply_total={:.3?}",
          total_zeta_time, total_lambda_time, total_multiply_time);

    let function_elapsed = function_start.elapsed();
    info!("square_root() EXIT: total time={:.3?}", function_elapsed);

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

/// Optimized computation of (X^p - X) mod f mod prime
/// This is faster than creating the full X^p polynomial and then reducing it
/// because we can use binary exponentiation which only requires O(log p) polynomial multiplications
pub fn x_power_p_minus_x_mod_f(p: &BigInt, f: &Polynomial, prime: &BigInt) -> Polynomial {
    use crate::polynomial::polynomial::Term;

    // Compute X^p mod f using binary exponentiation
    let x = Polynomial::from_term(BigInt::one(), 1); // X
    let x_pow_p_mod_f = Polynomial::exponentiate_mod(&x, p, f, prime);

    // Subtract X
    let result = x_pow_p_mod_f - x;

    // Apply prime modulus to all coefficients
    result.field_modulus(prime)
}

pub fn remainder(left: &Polynomial, right: &Polynomial, mod_: &BigInt) -> Polynomial {
    // Ensure right polynomial has no leading zeros and coefficients are reduced mod p
    let mut right_cleaned = right.clone();
    right_cleaned = right_cleaned.field_modulus(mod_);
    right_cleaned.remove_zeros();

    // If right polynomial is zero after cleanup, return left
    if right_cleaned.is_zero() {
        return left.clone();
    }

    if right_cleaned.degree() > left.degree() {
        return left.clone();
    }

    let right_degree = right_cleaned.degree();
    let quotient_degree = left.degree() - right_degree + 1;

    // Get the leading coefficient of right polynomial (already reduced mod p)
    let leading_coef = right_cleaned[right_degree].clone();

    // Safety check: if leading coefficient is zero, something went wrong
    if leading_coef.is_zero() {
        // This shouldn't happen after field_modulus + remove_zeros, but handle it gracefully
        return left.clone();
    }

    // If not monic, we need to compute the modular inverse to normalize
    let leading_coef_inv = if leading_coef == BigInt::one() {
        BigInt::one()
    } else {
        match modular_multiplicative_inverse(&leading_coef, mod_) {
            Some(inv) => inv,
            None => {
                // Leading coef is not coprime with modulus - shouldn't happen with prime modulus
                // but return left to avoid panic
                return left.clone();
            }
        }
    };

    let mut rem = left.clone();
    let mut quot;

    for i in (0..quotient_degree).rev() {
        // Divide by leading coefficient (multiply by its inverse)
        quot = (rem[right_degree + i].clone() * &leading_coef_inv).mod_floor(mod_);

        rem[right_degree + i] = BigInt::zero();

        for j in (i..(right_degree + i)).rev() {
            rem[j] = (rem[j].clone() - &quot * &right_cleaned[j - i]).mod_floor(mod_);
        }
    }

    rem.remove_zeros();
    rem
}
