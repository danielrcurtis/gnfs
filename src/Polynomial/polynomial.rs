// src/polynomial/polynomial.rs

use std::cmp::Ordering;
use std::ops::{Add, Sub, Mul, Div, Index, IndexMut};
use num::BigInt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    coefficient: BigInt,
    exponent: usize,
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

    pub fn get_exponent(&self) -> usize {
        self.exponent
    }

    pub fn set_exponent(&mut self, exponent: usize) {
        self.exponent = exponent;
    }

    pub fn parse(input: &str) -> Self {
        // Implement parsing logic based on the C# code
        // Example: Term::parse("2*X^3") should return Term { coefficient: 2, exponent: 3 }
        // You may need to adjust this based on the specific parsing requirements
        unimplemented!("Parsing logic for Term is not implemented yet.")
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
    terms: Vec<Term>,
}

impl Polynomial {
    pub fn new(terms: Vec<Term>) -> Self {
        let mut polynomial = Polynomial { terms };
        polynomial.remove_zeros();
        polynomial
    }

    pub fn from_roots(roots: &[BigInt]) -> Self {
        // Implement logic to create a polynomial from roots
        unimplemented!("Creating polynomial from roots is not implemented yet.")
    }

    pub fn parse(input: &str) -> Self {
        // Implement parsing logic based on the C# code
        unimplemented!("Parsing logic for Polynomial is not implemented yet.")
    }

    pub fn degree(&self) -> usize {
        self.terms.last().map(|term| term.get_exponent()).unwrap_or(0)
    }

    pub fn evaluate(&self, x: &BigInt) -> BigInt {
        let mut result = BigInt::zero();
        for term in &self.terms {
            result += term.get_coefficient() * x.pow(term.get_exponent() as u32);
        }
        result
    }

    pub fn derivative(&self) -> Self {
        // Implement logic to calculate the derivative of the polynomial
        unimplemented!("Derivative calculation is not implemented yet.")
    }

    pub fn indefinite_integral(&self, c: &BigInt) -> Self {
        // Implement logic to calculate the indefinite integral of the polynomial
        unimplemented!("Indefinite integral calculation is not implemented yet.")
    }

    fn remove_zeros(&mut self) {
        self.terms.retain(|term| term.get_coefficient() != &BigInt::zero());
        if self.terms.is_empty() {
            self.terms.push(Term::new(BigInt::zero(), 0));
        }
    }

    fn is_zero(&self) -> bool {
        self.terms.len() == 1 && self.terms[0].get_coefficient() == &BigInt::zero()
    }

    fn combine_like_terms(&mut self) {
        let mut terms = Vec::new();
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

    fn get_coefficient_mut(&mut self, exponent: usize) -> &mut BigInt {
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
}


impl Index<usize> for Polynomial {
    type Output = BigInt;

    fn index(&self, index: usize) -> &BigInt {
        self.terms
            .iter()
            .find(|term| term.get_exponent() == index)
            .map(|term| term.get_coefficient())
            .unwrap_or(&BigInt::zero())
    }
}

impl IndexMut<usize> for Polynomial {
    fn index_mut(&mut self, index: usize) -> &mut BigInt {
        let term = self.terms
            .iter_mut()
            .find(|term| term.get_exponent() == index);
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
            panic!("Division by zero polynomial");
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
