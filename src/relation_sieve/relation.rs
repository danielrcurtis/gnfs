// src/realation_sieve/relation.rs

use num::{BigInt, BigRational, Zero, Signed};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use crate::core::gnfs::GNFS;
use crate::integer_math::factorization_factory::FactorizationFactory;
use crate::core::count_dictionary::CountDictionary;

#[derive(Debug, Clone)]
pub struct Relation {
    pub a: BigInt,
    pub b: BigInt,
    pub algebraic_norm: BigInt,
    pub rational_norm: BigInt,
    pub algebraic_quotient: BigInt,
    pub rational_quotient: BigInt,
    pub algebraic_factorization: CountDictionary,
    pub rational_factorization: CountDictionary,
    pub is_persisted: bool,
}

impl Relation {
    pub fn new(_gnfs: &GNFS, a: &BigInt, b: &BigInt) -> Self {
        Relation {
            a: a.clone(),
            b: b.clone(),
            algebraic_norm: BigInt::default(),
            rational_norm: BigInt::default(),
            algebraic_quotient: BigInt::default(),
            rational_quotient: BigInt::default(),
            algebraic_factorization: CountDictionary::new(),
            rational_factorization: CountDictionary::new(),
            is_persisted: false,
        }
    }

    pub fn is_smooth(&self) -> bool {
        self.is_rational_quotient_smooth() && self.is_algebraic_quotient_smooth()
    }

    pub fn is_rational_quotient_smooth(&self) -> bool {
        self.rational_quotient == BigInt::from(1) || self.rational_quotient == BigInt::from(0)
    }

    pub fn is_algebraic_quotient_smooth(&self) -> bool {
        self.algebraic_quotient == BigInt::from(1) || self.algebraic_quotient == BigInt::from(0)
    }

    pub fn apply(&self, x: &BigInt) -> BigInt {
        &self.a + &self.b * x
    }

    pub fn sieve(&mut self, gnfs: &GNFS) {
        use log::debug;

        // Rational norm: a + b*m where m is the polynomial base
        self.rational_norm = self.apply(&gnfs.polynomial_base);

        // Algebraic norm: f(-a/b) × (-b)^degree
        // This is the correct formula from the C# reference implementation
        let neg_a = -(&self.a);
        let ab_ratio = BigRational::new(neg_a, self.b.clone());

        // Evaluate f(-a/b) using rational arithmetic
        let poly_value = gnfs.current_polynomial.evaluate_rational(&ab_ratio);

        // Calculate (-b)^degree
        let neg_b = -(&self.b);
        let degree = gnfs.current_polynomial.degree();
        let right = neg_b.pow(degree as u32);

        // Multiply: f(-a/b) × (-b)^degree
        let product = poly_value * BigRational::from_integer(right);

        // Extract integer part (should have no fractional part for valid relations)
        if !product.is_integer() {
            debug!("Warning: Algebraic norm for (a={}, b={}) is not an integer: {}", self.a, self.b, product);
        }
        self.algebraic_norm = product.numer().clone() / product.denom();

        // Handle negative norms: add -1 to factorization
        if self.rational_norm < BigInt::zero() {
            self.rational_factorization.add(&BigInt::from(-1));
        }

        // Use absolute value for factorization
        let abs_rational_norm = self.rational_norm.abs();

        // OPTIMIZATION: Sieve rational first (C# does this)
        // Only sieve algebraic if rational is smooth
        let (rational_factors, rational_quotient) = FactorizationFactory::factor_with_base(
            &abs_rational_norm,
            &gnfs.prime_factor_base.rational_factor_base
        );

        self.rational_factorization.combine(&rational_factors);
        self.rational_quotient = rational_quotient.clone();

        // Only continue if rational is smooth
        if !self.is_rational_quotient_smooth() {
            // Not smooth on rational side, no point checking algebraic
            self.algebraic_quotient = self.algebraic_norm.abs();
            return;
        }

        // Rational is smooth, now check algebraic
        if self.algebraic_norm < BigInt::zero() {
            self.algebraic_factorization.add(&BigInt::from(-1));
        }

        let abs_algebraic_norm = self.algebraic_norm.abs();
        let (algebraic_factors, algebraic_quotient) = FactorizationFactory::factor_with_base(
            &abs_algebraic_norm,
            &gnfs.prime_factor_base.algebraic_factor_base
        );

        self.algebraic_factorization.combine(&algebraic_factors);
        self.algebraic_quotient = algebraic_quotient.clone();

        let is_smooth = self.is_smooth();
        if is_smooth || (&self.a == &BigInt::from(1) && &self.b <= &BigInt::from(10)) {
            debug!("Relation (a={}, b={}): alg_norm={}, rat_norm={}, alg_quot={}, rat_quot={}, smooth={}",
                   self.a, self.b, self.algebraic_norm, self.rational_norm,
                   algebraic_quotient, rational_quotient, is_smooth);
        }

        // No need to retain - factor_with_base only returns factors from the base
        // self.algebraic_factorization
        //     .retain(|prime, _| gnfs.prime_factor_base.algebraic_factor_base.contains(prime));
        // self.rational_factorization
        //     .retain(|prime, _| gnfs.prime_factor_base.rational_factor_base.contains(prime));

        debug!("  Is smooth? {}", self.is_smooth());

        // Note: Do NOT increment smooth_relations_counter here
        // It's handled in poly_relations_sieve_progress.rs after checking is_smooth()
    }
}

impl PartialEq for Relation {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b
    }
}

impl Eq for Relation {}

impl Hash for Relation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.a.hash(state);
        self.b.hash(state);
    }
}

impl PartialOrd for Relation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.a.cmp(&other.a).then(self.b.cmp(&other.b)))
    }
}

impl Ord for Relation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}