// src/polynomial/polynomial.rs

use std::cmp::Ordering;
use std::ops::{Add, Sub, Mul, Div, Index, IndexMut};
use num::{BigInt, Zero, One, Integer, Signed};
use log::error;
use std::fmt::{Display, Formatter, Result};
use crate::square_root::finite_field_arithmetic::remainder;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub coefficient: BigInt,
    pub exponent: usize,
}

impl Term {
    pub fn new(coefficient: BigInt, exponent: usize) -> Self {
        Term {
            coefficient,
            exponent,
        }
    }

    pub fn get_coefficient(&self) -> &BigInt {
        &self.coefficient
    }

    pub fn set_coefficient(&mut self, coefficient: BigInt) {
        self.coefficient = coefficient;
    }

    pub fn get_coefficient_mut(&mut self) -> &mut BigInt {
        &mut self.coefficient
    }

    pub fn get_exponent(&self) -> usize {
        self.exponent
    }

    pub fn set_exponent(&mut self, exponent: usize) {
        self.exponent = exponent;
    }

    pub fn parse(input: &str) -> Self {
        let mut coefficient = BigInt::one();
        let mut exponent = 0;
    
        let parts: Vec<&str> = input.split('*').collect();
    
        for part in parts {
            if part.starts_with('-') {
                coefficient *= -BigInt::one();
            }
    
            if part.contains('X') {
                let exp_parts: Vec<&str> = part.split('^').collect();
                if exp_parts.len() == 2 {
                    exponent = exp_parts[1].parse().unwrap_or(0);
                } else {
                    exponent = 1;
                }
    
                if let Some(coeff_str) = exp_parts[0].trim_end_matches('X').trim().strip_prefix('-') {
                    if !coeff_str.is_empty() {
                        coefficient *= coeff_str.parse::<BigInt>().unwrap_or(BigInt::one());
                    }
                } else if let Some(coeff_str) = exp_parts[0].trim_end_matches('X').trim().strip_prefix('+') {
                    if !coeff_str.is_empty() {
                        coefficient *= coeff_str.parse::<BigInt>().unwrap_or(BigInt::one());
                    }
                } else if exp_parts[0].trim_end_matches('X').trim().parse::<BigInt>().is_ok() {
                    coefficient *= exp_parts[0].trim_end_matches('X').trim().parse::<BigInt>().unwrap();
                }
            } else {
                coefficient *= part.parse::<BigInt>().unwrap_or(BigInt::one());
            }
        }
    
        Term::new(coefficient, exponent)
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();

        if self.coefficient.is_negative() {
            result.push_str("-");
        }

        if self.coefficient != BigInt::one() {
            result.push_str(&self.coefficient.to_string());
        }

        if self.exponent > 0 {
            result.push_str("X");
            if self.exponent > 1 {
                result.push_str("^");
                result.push_str(&self.exponent.to_string());
            }
        }

        result
    }

}

impl PartialOrd for Term {
    fn partial_cmp(&self, other: &Term) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Term {
    fn cmp(&self, other: &Term) -> Ordering {
        // Compare terms based on their exponents
        self.exponent.cmp(&other.exponent)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Polynomial {
    pub terms: Vec<Term>,
}

impl Polynomial {
    pub fn new(terms: Vec<Term>) -> Self {
        let mut polynomial = Polynomial { terms };
        polynomial.remove_zeros();
        polynomial
    }

    pub fn from_term(coefficient: BigInt, exponent: usize) -> Self {
        Polynomial {
            terms: vec![Term::new(coefficient, exponent)],
        }
    }

    pub fn one() -> Self {
        Polynomial::from_term(BigInt::one(), 0)
    }

    pub fn from_roots(roots: &[BigInt]) -> Self {
        let polys: Vec<Polynomial> = roots
            .iter()
            .map(|root| Polynomial::new(vec![Term::new(BigInt::one(), 1), Term::new(-root.clone(), 0)]))
            .collect();
        Polynomial::product(&polys)
    }

    pub fn product(polys: &[Polynomial]) -> Self {
        polys.iter().fold(Polynomial::new(vec![Term::new(BigInt::one(), 0)]), |acc, poly| acc * poly.clone())
    }

    pub fn mod_mod(to_reduce: &Polynomial, mod_poly: &Polynomial, prime_modulus: &BigInt) -> Polynomial {
        match mod_poly.cmp(to_reduce) {
            Ordering::Greater => to_reduce.clone(),
            Ordering::Equal => Polynomial::zero(),
            Ordering::Less => remainder(to_reduce, mod_poly, prime_modulus),
        }
    }

    pub fn field_gcd(left: &Polynomial, right: &Polynomial, modulus: &BigInt) -> Polynomial {
        let mut polynomial = left.clone();
        let mut polynomial2 = right.clone();
        if polynomial2.degree() > polynomial.degree() {
            std::mem::swap(&mut polynomial, &mut polynomial2);
        }

        while !polynomial2.is_zero() {
            let to_reduce = polynomial.clone();
            polynomial = polynomial2;
            polynomial2 = Polynomial::mod_mod(&to_reduce, &polynomial2, modulus);
        }

        if polynomial.degree() == 0 {
            return Polynomial::one();
        }

        polynomial
    }

    pub fn parse(input: &str) -> Self {
        let input = input.replace(" ", "").replace("−", "-").replace("-", "+-");
        let terms: Vec<Term> = input
            .split('+')
            .filter(|s| !s.is_empty())
            .map(Term::parse)
            .collect();
        if terms.is_empty() {
            error!("Invalid input: {}", input);
        }
        Polynomial::new(terms)
    }

    pub fn degree(&self) -> usize {
        self.terms.last().map_or(0, |term| term.exponent)
    }
    
    pub fn divide(&self, other: &Polynomial) -> (Polynomial, Polynomial) {
        if other.degree() > self.degree() || other.cmp(self) == Ordering::Greater {
            return (Polynomial::zero(), self.clone());
        }

        let right_degree = other.degree();
        let quotient_degree = self.degree() - right_degree + 1;
        let divisor = other[right_degree].clone();

        let mut polynomial = self.clone();
        let mut quotient = Polynomial::zero();

        for i in (0..quotient_degree).rev() {
            quotient[i] = polynomial[right_degree + i].clone() / divisor.clone();
            polynomial[right_degree + i] = BigInt::zero();

            for j in (i..=right_degree + i - 1).rev() {
                polynomial[j] -= &quotient[i] * &other[j - i];
            }
        }

        polynomial.remove_zeros();
        quotient.remove_zeros();

        (quotient, polynomial)
    }

    pub fn evaluate(&self, x: &BigInt) -> BigInt {
        let mut result = BigInt::zero();
        for term in &self.terms {
            result += term.coefficient.clone() * x.pow(term.exponent as u32);
        }
        result
    }

    // pub fn evaluate<T: ToPrimitive>(&self, x: T) -> BigInt {
    //     let mut result = BigInt::zero();
    //     for term in &self.terms {
    //         let x_bigint = BigInt::from(x.to_i64().unwrap());
    //         result += term.coefficient.clone() * x_bigint.pow(term.exponent as u32);
    //     }
    //     result
    // }

    pub fn derivative(&self) -> Self {
        let terms: Vec<Term> = self.terms.iter().filter_map(|term| {
            if term.exponent > 0 {
                let new_coefficient = term.coefficient.clone() * BigInt::from(term.exponent);
                let new_exponent = term.exponent - 1;
                Some(Term::new(new_coefficient, new_exponent))
            } else {
                None
            }
        }).collect();
        Polynomial::new(terms)
    }

    pub fn indefinite_integral(&self, c: &BigInt) -> Self {
        let mut terms = vec![Term::new(c.clone(), 0)];
        for term in &self.terms {
            let new_exponent = term.exponent + 1;
            let new_coefficient = term.coefficient.clone() / BigInt::from(new_exponent);
            terms.push(Term::new(new_coefficient, new_exponent));
        }
        Polynomial::new(terms)
    }

    pub fn remove_zeros(&mut self) {
        self.terms.retain(|term| !term.coefficient.is_zero());
        if self.terms.is_empty() {
            self.terms.push(Term::new(BigInt::zero(), 0));
        }
    }

    pub fn is_zero(&self) -> bool {
        self.terms.len() == 1 && self.terms[0].get_coefficient() == &BigInt::zero()
    }

    pub fn combine_like_terms(&mut self) {
        let mut terms: Vec<Term> = Vec::new();
        for term in &self.terms {
            if let Some(t) = terms.iter_mut().find(|t| t.get_exponent() == term.get_exponent()) {
                *t.get_coefficient_mut() += term.get_coefficient();
            } else {
                terms.push(term.clone());
            }
        }
        self.terms = terms;
        self.remove_zeros();
    }

    pub fn get_coefficient_mut(&mut self, exponent: usize) -> &mut BigInt {
        if let Some(term) = self.terms.iter_mut().find(|t| t.get_exponent() == exponent) {
            &mut term.coefficient
        } else {
            self.terms.push(Term::new(BigInt::zero(), exponent));
            &mut self.terms.last_mut().unwrap().coefficient
        }
    }

    pub fn zero() -> Self {
        Polynomial::new(vec![Term::new(BigInt::zero(), 0)])
    }

    pub fn square(&self) -> Self {
        Polynomial::multiply(self, self)
    }

    pub fn field_modulus(&self, modulus: &BigInt) -> Self {
        let terms: Vec<Term> = self.terms.iter().map(|term| {
            let coefficient = term.coefficient.mod_floor(modulus);
            Term::new(coefficient, term.exponent)
        }).collect();
        Polynomial::new(terms)
    }

    pub fn field_modulus_from_polynomial(&self, mod_poly: &Polynomial) -> Polynomial {
        let compare = mod_poly.cmp(self);
        if compare == Ordering::Greater {
            self.clone()
        } else if compare == Ordering::Equal {
            Polynomial::zero()
        } else {
            Polynomial::remainder(self, mod_poly)
        }
    }

    fn remainder(left: &Polynomial, right: &Polynomial) -> Polynomial {
        if right.degree() > left.degree() || right.cmp(left) == Ordering::Greater {
            Polynomial::zero()
        } else {
            let right_degree = right.degree();
            let quotient_degree = left.degree() - right_degree + 1;

            let leading_coefficient = right[right_degree].clone();
            if leading_coefficient != BigInt::one() {
                panic!("This method expects only monomials (leading coefficient is 1) for the right-hand-side polynomial.");
            }

            let mut rem = left.clone();
            let mut quot = BigInt::zero();

            for i in (0..quotient_degree).rev() {
                quot = rem[right_degree + i].clone();

                rem[right_degree + i] = BigInt::zero();

                for j in (i..=right_degree + i - 1).rev() {
                    rem[j] -= &quot * &right[j - i];
                }
            }

            Polynomial::new(rem.terms)
        }
    }

    pub fn multiply(left: &Polynomial, right: &Polynomial) -> Self {
        let mut terms = vec![Term::new(BigInt::zero(), 0); left.degree() + right.degree() + 1];
        for i in 0..=left.degree() {
            for j in 0..=right.degree() {
                let coefficient = &left[i] * &right[j];
                terms[i + j].coefficient += coefficient;
            }
        }
        Polynomial::new(terms)
    }

    // pub fn make_monic(&mut self, polynomial_base: &BigInt) {
    //     let degree = self.degree();
    //     if self[degree].abs() > BigInt::one() {
    //         let factor = (&self[degree] - BigInt::one()) * polynomial_base;
    //         self[degree] = BigInt::one();
    //         self[degree - 1] += factor;
    //     }
    // }

    pub fn make_monic(&self, polynomial_base: &BigInt) -> Polynomial {
        let degree = self.degree();
        let mut result = self.clone();

        if result[degree].abs() > BigInt::one() {
            let factor = (&result[degree] - BigInt::one()) * polynomial_base;
            result[degree] = BigInt::one();
            result[degree - 1] += factor;
        }

        result
    }

    pub fn get_derivative_polynomial(&self) -> Self {
        let terms: Vec<Term> = self.terms.iter().filter_map(|term| {
            let exponent = term.exponent;
            if exponent > 0 {
                let coefficient = &term.coefficient * BigInt::from(exponent);
                Some(Term::new(coefficient, exponent - 1))
            } else {
                None
            }
        }).collect();
        Polynomial::new(terms)
    }

    pub fn exponentiate_mod(base: &Polynomial, exponent: &BigInt, modulus: &Polynomial, prime: &BigInt) -> Polynomial {
        let mut result = Polynomial::new(vec![Term::new(BigInt::one(), 0)]);
        let mut base = base.clone();
        let mut exponent = exponent.clone();

        while exponent > BigInt::zero() {
            if exponent.is_odd() {
                result = Polynomial::multiply(&result, &base);
                result = Polynomial::mod_mod(&result, modulus, prime);
            }
            base = base.square();
            base = Polynomial::mod_mod(&base, modulus, prime);
            exponent /= 2;
        }

        result
    }
}


impl Index<usize> for Polynomial {
    type Output = BigInt;

    fn index(&self, index: usize) -> &BigInt {
        self.terms.iter().find(|term| term.exponent == index)
            .map_or(&BigInt::zero(), |term| &term.coefficient)
    }
}

impl IndexMut<usize> for Polynomial {
    fn index_mut(&mut self, index: usize) -> &mut BigInt {
        let term = self.terms.iter_mut().find(|term| term.exponent == index);
        if let Some(term) = term {
            &mut term.coefficient
        } else {
            self.terms.push(Term::new(BigInt::zero(), index));
            &mut self.terms.last_mut().unwrap().coefficient
        }
    }
}

impl Add for Polynomial {
    type Output = Polynomial;

    fn add(self, other: Polynomial) -> Polynomial {
        let mut terms = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < self.terms.len() && j < other.terms.len() {
            let term1 = &self.terms[i];
            let term2 = &other.terms[j];

            if term1.get_exponent() == term2.get_exponent() {
                let coefficient = term1.get_coefficient() + term2.get_coefficient();
                if coefficient != BigInt::zero() {
                    terms.push(Term::new(coefficient, term1.get_exponent()));
                }
                i += 1;
                j += 1;
            } else if term1.get_exponent() > term2.get_exponent() {
                terms.push(term1.clone());
                i += 1;
            } else {
                terms.push(term2.clone());
                j += 1;
            }
        }

        while i < self.terms.len() {
            terms.push(self.terms[i].clone());
            i += 1;
        }

        while j < other.terms.len() {
            terms.push(other.terms[j].clone());
            j += 1;
        }

        Polynomial::new(terms)
    }
}

impl Sub for Polynomial {
    type Output = Polynomial;

    fn sub(self, other: Polynomial) -> Polynomial {
        let negated_terms: Vec<Term> = other.terms.iter().map(|term| {
            Term::new(-term.get_coefficient(), term.get_exponent())
        }).collect();
        
        self + Polynomial::new(negated_terms)
    }
}

impl Mul for Polynomial {
    type Output = Polynomial;

    fn mul(self, other: Polynomial) -> Polynomial {
        let mut terms = Vec::new();

        for term1 in &self.terms {
            for term2 in &other.terms {
                let coefficient = term1.get_coefficient() * term2.get_coefficient();
                let exponent = term1.get_exponent() + term2.get_exponent();
                terms.push(Term::new(coefficient, exponent));
            }
        }

        let mut result = Polynomial::new(terms);
        result.combine_like_terms();
        result
    }
}

impl Div for Polynomial {
    type Output = (Polynomial, Polynomial);

    fn div(self, other: Polynomial) -> (Polynomial, Polynomial) {
        if other.is_zero() {
            error!("Division by zero polynomial");
        }

        let mut quotient = Polynomial::zero();
        let mut remainder = self;

        while !remainder.is_zero() && remainder.degree() >= other.degree() {
            let leading_term = remainder.terms.last().unwrap();
            let divisor_term = other.terms.last().unwrap();
            let term_quotient = Term::new(
                leading_term.get_coefficient() / divisor_term.get_coefficient(),
                leading_term.get_exponent() - divisor_term.get_exponent(),
            );
            let term_polynomial = Polynomial::new(vec![term_quotient]);
            quotient = quotient + term_polynomial.clone();
            remainder = remainder - term_polynomial * other.clone();
        }

        (quotient, remainder)
    }
}

impl PartialOrd for Polynomial {
    fn partial_cmp(&self, other: &Polynomial) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Polynomial {
    fn cmp(&self, other: &Polynomial) -> Ordering {
        if self.degree() != other.degree() {
            return self.degree().cmp(&other.degree());
        }

        for i in (0..=self.degree()).rev() {
            let a = self[i].clone();
            let b = other[i].clone();
            if a != b {
                return a.cmp(&b);
            }
        }

        Ordering::Equal
    }
}

impl Display for Polynomial {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut output = String::new();
        for (i, term) in self.terms.iter().enumerate() {
            if i > 0 {
                output += " + ";
            }
            output += &term.to_string();
        }
        write!(f, "{}", output)
    }
}

impl Default for Polynomial {
    fn default() -> Self {
        Polynomial::zero()
    }
}

// pub mod algorithms {
//     use super::*;
//     use num::{BigInt, Complex, Integer, One, Zero};
//     use num::ToPrimitive;
//     use std::cmp::Ordering;

//     pub fn eulers_criterion(a: &BigInt, p: &BigInt) -> BigInt {
//         let exponent = (p - 1) / 2;
//         a.modpow(&exponent, p)
//     }

//     pub fn legendre_symbol(a: &BigInt, p: &BigInt) -> i32 {
//         if p < &BigInt::from(2) {
//             panic!("Parameter 'p' must not be < 2, but you have supplied: {}", p);
//         }

//         if a.is_zero() {
//             return 0;
//         }

//         if a == &BigInt::one() {
//             return 1;
//         }

//         let mut num;
//         if a % 2 == BigInt::zero() {
//             num = legendre_symbol(&(a / 2), p);
//             if ((p * p - 1) & 8) != BigInt::zero() {
//                 num = -num;
//             }
//         } else {
//             num = legendre_symbol(&(p % a), a);
//             if (((a - 1) * (p - 1)) & 4) != BigInt::zero() {
//                 num = -num;
//             }
//         }

//         num
//     }

//     pub fn legendre_symbol_search(start: &BigInt, modulus: &BigInt, goal: &BigInt) -> BigInt {
//         if goal != &BigInt::from(-1) && goal != &BigInt::zero() && goal != &BigInt::one() {
//             panic!("Parameter 'goal' may only be -1, 0 or 1. It was {}.", goal);
//         }

//         let mut i = start.clone();
//         while legendre_symbol(&i, modulus) != goal.to_i32().unwrap() {
//             i += 1;
//         }

//         i
//     }

//     pub fn tonelli_shanks(n: &BigInt, p: &BigInt) -> BigInt {
//         let legendre = legendre_symbol(n, p);
//         if legendre != 1 {
//             panic!("Parameter n is not a quadratic residue, mod p. Legendre symbol = {}", legendre);
//         }

//         if p.mod_floor(&BigInt::from(4)) == 3 {
//             return n.modpow(&((p + 1) / 4), p);
//         }

//         let mut q = p - 1;
//         let mut s = BigInt::zero();
//         while q.mod_floor(&BigInt::from(2)) == BigInt::zero() {
//             q /= 2;
//             s += 1;
//         }

//         if s.is_zero() {
//             panic!("Unexpected error: s is zero");
//         }

//         if s == BigInt::one() {
//             panic!("This case should have already been covered by the p mod 4 check above.");
//         }

//         let z = legendre_symbol_search(&BigInt::zero(), p, &BigInt::from(-1));
//         let mut c = n.modpow(&((q + 1) / 2), p);
//         let mut r = n.modpow(&q, p);
//         let mut t = BigInt::one();
//         let mut m = s;
//         while r != BigInt::one() && t < m {
//             let exponent = BigInt::from(2).pow((m - t - 1).to_u32().unwrap());
//             let b = z.modpow(&exponent, p);
//             c = (c * b).mod_floor(p);
//             r = (r * b * b).mod_floor(p);
//             z = b * b;
//             t += 1;
//         }

//         c
//     }

//     pub fn chinese_remainder_theorem(n: &[BigInt], a: &[BigInt]) -> BigInt {
//         let product = n.iter().fold(BigInt::one(), |acc, &x| acc * x);
//         let mut sum = BigInt::zero();
//         for i in 0..n.len() {
//             let p = &product / &n[i];
//             sum += &a[i] * modular_multiplicative_inverse(&p, &n[i]) * p;
//         }
//         sum % product
//     }

//     pub fn modular_multiplicative_inverse(a: &BigInt, m: &BigInt) -> BigInt {
//         let mut r = a % m;
//         for i in 1..m {
//             if (&r * i) % m == BigInt::one() {
//                 return BigInt::from(i);
//             }
//         }
//         BigInt::one()
//     }

//     pub fn eulers_totient_phi(n: i32) -> i32 {
//         if n < 3 {
//             return 1;
//         }
//         if n == 3 {
//             return 2;
//         }

//         let mut result = n;
//         if (n & 1) == 0 {
//             result >>= 1;
//             while ((n >>= 1) & 1) == 0 {}
//         }

//         let mut i = 3;
//         while i * i <= n {
//             if n % i == 0 {
//                 result -= result / i;
//                 while (n /= i) % i == 0 {}
//             }
//             i += 2;
//         }

//         if n > 1 {
//             result -= result / n;
//         }

//         result
//     }

//     pub fn laguerres_method(poly: &Polynomial, guess: f64, max_iterations: f64, precision: f64) -> f64 {
//         if poly.degree() < 1 {
//             panic!("No root exists for a constant (degree 0) polynomial!");
//         }

//         let mut x = guess;
//         let n = poly.degree() as f64;
//         let derivative = poly.get_derivative_polynomial();
//         let second_derivative = derivative.get_derivative_polynomial();

//         for i in 0..(max_iterations as i32) {
//             if !(poly.evaluate(x).abs() >= precision) {
//                 break;
//             }

//             let g = derivative.evaluate(x) / poly.evaluate(x);
//             let h = g * g - second_derivative.evaluate(x) / poly.evaluate(x);
//             let sqrt_term = ((n - 1.0) * (n * h - g * g)).sqrt();
//             let denominator = if (g + sqrt_term).abs() >= (g - sqrt_term).abs() {
//                 g + sqrt_term
//             } else {
//                 g - sqrt_term
//             };
//             let delta = n / denominator;
//             x -= delta;

//             if (i as f64) == max_iterations {
//                 return f64::NAN;
//             }
//         }

//         if poly.evaluate(x).abs() >= precision {
//             return f64::NAN;
//         }

//         let digits = (-precision.log10()) as i32;
//         x.round_to(digits)
//     }

//     pub fn laguerres_method_complex(poly: &Polynomial, guess: Complex<f64>, max_iterations: f64, precision: f64) -> Complex<f64> {
//         if poly.degree() < 1 {
//             panic!("No root exists for a constant (degree 0) polynomial!");
//         }

//         let mut x = guess;
//         let n = poly.degree() as f64;
//         let derivative = poly.get_derivative_polynomial();
//         let second_derivative = derivative.get_derivative_polynomial();

//         for i in 0..(max_iterations as i32) {
//             if Complex::abs(poly.evaluate(x)) < precision {
//                 break;
//             }

//             let g = derivative.evaluate(x) / poly.evaluate(x);
//             let h = g * g - second_derivative.evaluate(x) / poly.evaluate(x);
//             let sqrt_term = Complex::sqrt((n - 1.0) * (n * h - g * g));
//             let denominator = if Complex::abs(g + sqrt_term) >= Complex::abs(g - sqrt_term) {
//                 g + sqrt_term
//             } else {
//                 g - sqrt_term
//             };
//             let delta = n / denominator;
//             x -= delta;

//             if (i as f64) == max_iterations {
//                 return Complex::zero();
//             }
//         }

//         let digits = (-precision.log10()) as i32;
//         Complex::new(x.re.round_to(digits), x.im.round_to(digits))
//     }
// }
