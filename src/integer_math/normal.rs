// src/factors/normal.rs

use num::{BigInt, BigRational, Zero};
use crate::polynomial::polynomial::Polynomial;

pub struct Normal;

impl Normal {
    /// a + bm
    pub fn rational(a: &BigInt, b: &BigInt, polynomial_base: &BigInt) -> BigInt {
        a + b * polynomial_base
    }

    /// a - bm
    pub fn rational_subtract(a: &BigInt, b: &BigInt, polynomial_base: &BigInt) -> BigInt {
        a - b * polynomial_base
    }

    /// ƒ(b) ≡ 0 (mod a)
    ///
    /// Calculated as:
    /// ƒ(-a/b) * -b^deg
    pub fn algebraic(a: &BigInt, b: &BigInt, poly: &Polynomial) -> BigInt {
        let a_d = BigRational::from(a.clone());
        let b_d = BigRational::from(b.clone());
        let ab = -a_d / b_d;
        let left = Self::polynomial_evaluate_big_rational(poly, &ab);
        let right = BigRational::from((-b).pow(poly.degree() as u32));
        let product = right * left;
        let fractional_part = product.fract();
        if fractional_part != BigRational::zero() {
            // GNFS::log_function(&format!("{} failed to result in an integer. This shouldn't happen.", "Algebraic"));
        }
        product.to_integer()
    }

    fn polynomial_evaluate_big_rational(polynomial: &Polynomial, indeterminate_value: &BigRational) -> BigRational {
        let mut num = polynomial.degree();
        let mut result = BigRational::from(polynomial[num].clone());
        while num > 0 {
            num -= 1;
            result *= indeterminate_value;
            result += BigRational::from(polynomial[num].clone());
        }
        result
    }
}