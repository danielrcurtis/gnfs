// src/polynomial/polynomial.rs

use std::cmp::Ordering;
use std::ops::{Add, Sub, Mul, Div, Index, IndexMut};
use std::collections::HashMap;
use num::{BigInt, Zero, One, Integer, Signed};
use log::error;
use std::fmt::{Display, Formatter, Result};
use crate::square_root::finite_field_arithmetic::remainder;
use lazy_static::lazy_static;

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
    pub terms: HashMap<usize, BigInt>,
}

impl Polynomial {
    pub fn new(terms: Vec<Term>) -> Self {
        let mut map = HashMap::new();
        for term in terms {
            map.insert(term.exponent, term.coefficient);
        }
        Polynomial { terms: map }
    }

    pub fn from_term(coefficient: BigInt, exponent: usize) -> Self {
        let mut terms = HashMap::new();
        terms.insert(exponent, coefficient);
        Polynomial { terms }
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
            std::mem::swap(&mut polynomial, &mut polynomial2);
            polynomial2 = Polynomial::mod_mod(&to_reduce, &polynomial2.clone(), modulus);
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
        self.terms.keys().max().copied().unwrap_or(0)
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
        for (&exponent, coefficient) in &self.terms {
            result += coefficient.clone() * x.pow(exponent as u32);
        }
        result
    }

    pub fn derivative(&self) -> Self {
        let mut terms = HashMap::new();
        for (&exponent, coefficient) in &self.terms {
            if exponent > 0 {
                let new_coefficient = coefficient.clone() * BigInt::from(exponent);
                terms.insert(exponent - 1, new_coefficient);
            }
        }
        Polynomial { terms }
    }

    pub fn indefinite_integral(&self, c: &BigInt) -> Self {
        let mut terms = HashMap::new();
        terms.insert(0, c.clone());
        for (&exponent, coefficient) in &self.terms {
            let new_exponent = exponent + 1;
            let new_coefficient = coefficient.clone() / BigInt::from(new_exponent);
            terms.insert(new_exponent, new_coefficient);
        }
        Polynomial { terms }
    }

    pub fn remove_zeros(&mut self) {
        self.terms.retain(|_, coefficient| !coefficient.is_zero());
        if self.terms.is_empty() {
            self.terms.insert(0, BigInt::zero());
        }
    }

    pub fn is_zero(&self) -> bool {
        self.terms.len() == 1 && self.terms.contains_key(&0) && self.terms[&0].is_zero()
    }

    // pub fn combine_like_terms(&mut self) {
    //     let mut terms: Vec<Term> = Vec::new();
    //     for term in &self.terms {
    //         if let Some(t) = terms.iter_mut().find(|t| t.get_exponent() == term.get_exponent()) {
    //             *t.get_coefficient_mut() += term.get_coefficient();
    //         } else {
    //             terms.push(term.clone());
    //         }
    //     }
    //     self.terms = terms;
    //     self.remove_zeros();
    // }

    // pub fn get_coefficient_mut(&mut self, exponent: usize) -> &mut BigInt {
    // // Check if the term exists and get its index
    // if let Some(index) = self.terms.iter().position(|t| t.get_exponent() == exponent) {
    //     // Return a mutable reference to the coefficient of the existing term
    //     &mut self.terms[index].coefficient
    // } else {
    //     // Create a new term and add it to the list
    //     let new_term = Term::new(BigInt::zero(), exponent);
    //     self.terms.push(new_term);
    //     // Return a mutable reference to the coefficient of the new term
    //     &mut self.terms.last_mut().unwrap().coefficient
    // }
    // }


    pub fn zero() -> Self {
        Polynomial::new(vec![Term::new(BigInt::zero(), 0)])
    }

    pub fn square(&self) -> Self {
        Polynomial::multiply(self, self)
    }

    pub fn field_modulus(&self, modulus: &BigInt) -> Self {
        let terms: HashMap<_, _> = self.terms
            .iter()
            .map(|(&exponent, coefficient)| (exponent, coefficient.mod_floor(modulus)))
            .collect();
        Polynomial { terms }
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

            let terms: Vec<Term> = rem.terms.into_iter().map(|(exponent, coefficient)| Term::new(coefficient, exponent)).collect();
            Polynomial::new(terms)
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
        let mut terms = HashMap::new();
        for (&exponent, coefficient) in &self.terms {
            if exponent > 0 {
                terms.insert(exponent - 1, coefficient * BigInt::from(exponent));
            }
        }
        Polynomial { terms }
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

lazy_static! {
    static ref ZERO: BigInt = BigInt::zero();
}

impl Index<usize> for Polynomial {
    type Output = BigInt;

    fn index(&self, index: usize) -> &BigInt {
        self.terms.get(&index).unwrap_or_else(|| &ZERO)
    }
}

impl IndexMut<usize> for Polynomial {
    fn index_mut(&mut self, index: usize) -> &mut BigInt {
        self.terms.entry(index).or_insert(BigInt::zero())
    }
}

impl Add for Polynomial {
    type Output = Polynomial;

    fn add(self, other: Polynomial) -> Polynomial {
        let mut terms = self.terms;
        for (exponent, coefficient) in other.terms {
            *terms.entry(exponent).or_insert(BigInt::zero()) += coefficient;
        }
        Polynomial { terms }
    }
}

impl Sub for Polynomial {
    type Output = Polynomial;

    fn sub(self, other: Polynomial) -> Polynomial {
        let mut terms = self.terms;
        for (exponent, coefficient) in other.terms {
            *terms.entry(exponent).or_insert(BigInt::zero()) -= coefficient;
        }
        terms.retain(|_, coefficient| !coefficient.is_zero());
        Polynomial { terms }
    }
}

impl Mul for Polynomial {
    type Output = Polynomial;

    fn mul(self, other: Polynomial) -> Polynomial {
        let mut terms = HashMap::new();
        for (&exp1, coef1) in &self.terms {
            for (&exp2, coef2) in &other.terms {
                let exponent = exp1 + exp2;
                let coefficient = coef1 * coef2;
                *terms.entry(exponent).or_insert(BigInt::zero()) += coefficient;
            }
        }
        Polynomial { terms }
    }
}

impl Div for Polynomial {
    type Output = (Polynomial, Polynomial);

    fn div(self, other: Polynomial) -> (Polynomial, Polynomial) {
        if other.is_zero() {
            error!("Division by zero polynomial");
        }

        let mut quotient = HashMap::new();
        let mut remainder = self.terms;

        while !remainder.is_empty() && remainder.keys().max().unwrap() >= other.terms.keys().max().unwrap() {
            let remainder_degree = *remainder.keys().max().unwrap();
            let divisor_degree = *other.terms.keys().max().unwrap();

            let leading_term_remainder = remainder.remove(&remainder_degree).unwrap();
            let leading_term_divisor = other.terms.get(&divisor_degree).unwrap();

            let term_quotient_exponent = remainder_degree - divisor_degree;
            let term_quotient_coefficient = leading_term_remainder / leading_term_divisor;

            let term_quotient = Term::new(term_quotient_coefficient.clone(), term_quotient_exponent);
            quotient.insert(term_quotient.exponent, term_quotient_coefficient);

            for (&exp, coef) in &other.terms {
                let exponent = term_quotient.exponent + exp;
                let coefficient = term_quotient.coefficient.clone() * coef;
                *remainder.entry(exponent).or_insert(BigInt::zero()) -= coefficient;
            }

            remainder.retain(|_, coef| !coef.is_zero());
        }

        (Polynomial { terms: quotient }, Polynomial { terms: remainder })
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
        let mut first = true;
        for (exponent, coefficient) in self.terms.iter() {
            if !first {
                output += " + ";
            }
            first = false;
            output += &format!("{}X^{}", coefficient, exponent);
        }
        write!(f, "{}", output)
    }
}

impl Default for Polynomial {
    fn default() -> Self {
        Polynomial::zero()
    }
}
