// src/polynomial/algorithms.rs

use num::{BigInt, Zero, One, ToPrimitive, Signed, Integer};
use num::complex::Complex;
use crate::polynomial::polynomial::Polynomial;

pub fn eulers_criterion(a: &BigInt, p: &BigInt) -> BigInt {
    let exponent = (p - 1) / 2;
    a.modpow(&exponent, p)
}

pub fn legendre_symbol(a: &BigInt, p: &BigInt) -> i32 {
    if p < &BigInt::from(2) {
        panic!("Parameter 'p' must not be < 2, but you have supplied: {}", p);
    }

    if a == &BigInt::zero() {
        return 0;
    }

    if a == &BigInt::one() {
        return 1;
    }

    let mut num;
    if a % 2 == BigInt::zero() {
        num = legendre_symbol(&(a / 2), p);
        if (p.pow(2) - 1) % 8 != BigInt::zero() {
            num = -num;
        }
    } else {
        num = legendre_symbol(&(p % a), a);
        if ((a - 1) * (p - 1)) % 4 != BigInt::zero() {
            num = -num;
        }
    }

    num
}

pub fn legendre_symbol_search(start: &BigInt, modulus: &BigInt, goal: &BigInt) -> BigInt {
    if goal != &BigInt::from(-1) && goal != &BigInt::zero() && goal != &BigInt::one() {
        panic!("Parameter 'goal' may only be -1, 0 or 1. It was {}.", goal);
    }

    let mut result = start.clone();
    while legendre_symbol(&result, modulus) != goal.to_i32().unwrap() {
        result += 1;
    }

    result
}

pub fn tonelli_shanks(n: &BigInt, p: &BigInt) -> BigInt {
    let legendre = legendre_symbol(n, p);
    if legendre != 1 {
        panic!("Parameter n is not a quadratic residue, mod p. Legendre symbol = {}", legendre);
    }
    if p % 4 == BigInt::from(3) {
        return n.modpow(&((p + 1) / 4), p);
    }

    let q: BigInt = p - 1;
    let mut s = BigInt::zero();
    let mut q_clone = q.clone();
    while q_clone.clone() % 2 == BigInt::zero() {
        q_clone /= 2;
        s += 1;
    }

    if s == BigInt::zero() {
        panic!("Unexpected case: s is zero.");
    }
    if s == BigInt::one() {
        panic!("This case should have already been covered by the p mod 4 check above.");
    }

    let z = legendre_symbol_search(&BigInt::zero(), p, &BigInt::from(-1));
    let mut c = n.modpow(&((q.clone() + 1) / 2), p);
    let mut r = n.modpow(&q, p);
    let mut t = BigInt::one();
    let m = s.clone();

    while r != BigInt::one() && t < m {
        let mut i = BigInt::one();
        let mut b = r.clone();
        while b != BigInt::one() {
            b = b.modpow(&BigInt::from(2), p);
            i += 1;
        }

        let exp = BigInt::from(2).pow(m.to_u32().unwrap() - t.to_u32().unwrap() - 1);
        let base = z.modpow(&exp, p);
        let base = base.clone();
        c = (c * base.clone()).mod_floor(p);
        r = (r * base.clone() * base).mod_floor(p);
        t += 1;
    }

    c
}

pub fn chinese_remainder_theorem(n: &[BigInt], a: &[BigInt]) -> BigInt {
    let prod_n: BigInt = n.iter().product();
    let mut sum = BigInt::zero();
    for i in 0..n.len() {
        let p = &prod_n / &n[i];
        sum += &a[i] * modular_multiplicative_inverse(&p, &n[i]) * p;
    }
    sum % prod_n
}

pub fn modular_multiplicative_inverse(a: &BigInt, m: &BigInt) -> BigInt {
    let r = a % m;
    for i in 1..m.to_u32().unwrap() {
        if (r.clone() * i) % m == BigInt::one() {
            return BigInt::from(i);
        }
    }
    BigInt::one()
}

pub fn eulers_totient_phi(n: u32) -> u32 {
    if n < 3 {
        return 1;
    }
    if n == 3 {
        return 2;
    }

    let mut result = n;
    let mut n = n;
    if n & 1 == 0 {
        result >>= 1;
        while n & 1 == 0 {
            n >>= 1;
        }
    }

    let mut i = 3;
    while i * i <= n {
        if n % i == 0 {
            result -= result / i;
            while n % i == 0 {
                n /= i;
            }
        }
        i += 2;
    }

    if n > 1 {
        result -= result / n;
    }

    result
}

pub fn laguerre_method(poly: &Polynomial, guess: f64, max_iterations: u32, precision: f64) -> f64 {
    if poly.degree() < 1 {
        panic!("No root exists for a constant (degree 0) polynomial!");
    }

    let mut x = guess;
    let n = poly.degree() as f64;
    let derivative_poly = poly.derivative();
    let derivative_poly2 = derivative_poly.derivative();

    for _i in 0..max_iterations {
        if poly.evaluate(&BigInt::from(x as i64)).abs().to_f64().unwrap() < precision {
            break;
        }

        let g = derivative_poly.evaluate(&BigInt::from(x as i64)).to_f64().unwrap() / poly.evaluate(&BigInt::from(x as i64)).to_f64().unwrap();
        let h = g * g - derivative_poly2.evaluate(&BigInt::from(x as i64)).to_f64().unwrap() / poly.evaluate(&BigInt::from(x as i64)).to_f64().unwrap();

        let delta = (n - 1.0) * (n * h - g * g);
        if delta < 0.0 {
            break;
        }

        let sqrt_delta = delta.sqrt();
        let denominator = if (g + sqrt_delta).abs() > (g - sqrt_delta).abs() {
            g + sqrt_delta
        } else {
            g - sqrt_delta
        };

        let a = n / denominator;
        x -= a;
    }

    if poly.evaluate(&BigInt::from(x as i64)).abs().to_f64().unwrap() >= precision {
        f64::NAN
    } else {
        let digits = (-precision.log10()) as u32;
        x.round() as f64 / 10.0_f64.powi(digits as i32)
    }
}

pub fn laguerre_method_complex(poly: &Polynomial, guess: Complex<f64>, max_iterations: u32, precision: f64) -> Complex<f64> {
    if poly.degree() < 1 {
        panic!("No root exists for a constant (degree 0) polynomial!");
    }

    let mut x = guess;
    let n = poly.degree() as f64;
    let derivative_poly = poly.derivative();
    let derivative_poly2 = derivative_poly.derivative();

    for _i in 0..max_iterations {
        let x_bigint = BigInt::from(x.re.round() as i64);
        if poly.evaluate(&x_bigint).abs().to_f64().unwrap() < precision {
            break;
        }

        let g = derivative_poly.evaluate(&x_bigint).to_f64().unwrap() / poly.evaluate(&x_bigint).to_f64().unwrap();
        let h = g * g - derivative_poly2.evaluate(&x_bigint).to_f64().unwrap() / poly.evaluate(&x_bigint).to_f64().unwrap();

        let delta = (n - 1.0) * (n * h - g * g);
        if delta < 0.0 {
            break;
        }

        let sqrt_delta = delta.sqrt();
        let denominator = if (g + sqrt_delta).abs() > (g - sqrt_delta).abs() {
            g + sqrt_delta
        } else {
            g - sqrt_delta
        };

        let a = Complex::new(n / denominator, 0.0);
        x -= a;
    }

    let x_bigint = BigInt::from(x.re.round() as i64);
    if poly.evaluate(&x_bigint).abs().to_f64().unwrap() >= precision {
        Complex::new(0.0, 0.0)
    } else {
        let digits = (-precision.log10()) as u32;
        Complex::new(
            x.re.round() as f64 / 10.0_f64.powi(digits as i32),
            x.im.round() as f64 / 10.0_f64.powi(digits as i32),
        )
    }
}