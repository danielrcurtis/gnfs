// src/realation_sieve/relation.rs

use num::BigInt;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use crate::core::gnfs::GNFS;
use crate::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress;
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
    pub fn new(gnfs: &GNFS, a: &BigInt, b: &BigInt) -> Self {
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

    pub fn sieve(&mut self, gnfs: &GNFS, progress: &mut PolyRelationsSieveProgress) {
        let f_a = gnfs.current_polynomial.evaluate(&self.a);
        let f_b = gnfs.current_polynomial.evaluate(&self.b);

        self.algebraic_norm = f_a.clone();
        self.rational_norm = self.apply(&f_b);

        let (algebraic_norm, algebraic_quotient) = FactorizationFactory::factor(&self.algebraic_norm);
        let (rational_norm, rational_quotient) = FactorizationFactory::factor(&self.rational_norm);

        self.algebraic_factorization = algebraic_norm;
        self.rational_factorization = rational_norm;

        self.algebraic_quotient = algebraic_quotient;
        self.rational_quotient = rational_quotient;

        self.algebraic_factorization
            .retain(|prime, _| gnfs.prime_factor_base.algebraic_factor_base.contains(prime));
        self.rational_factorization
            .retain(|prime, _| gnfs.prime_factor_base.rational_factor_base.contains(prime));

        let is_algebraic_quotient_smooth =
            self.algebraic_quotient == BigInt::from(1) || self.algebraic_quotient == BigInt::from(0);
        let is_rational_quotient_smooth =
            self.rational_quotient == BigInt::from(1) || self.rational_quotient == BigInt::from(0);

        if is_algebraic_quotient_smooth && is_rational_quotient_smooth {
            progress.smooth_relations_counter += 1;
        }
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