// src/realation_sieve/relation.rs

use num::{BigInt, Zero, Signed};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use crate::core::gnfs::GNFS;
use crate::core::gnfs_integer::GnfsInteger;
use crate::integer_math::factorization_factory::FactorizationFactory;
use crate::core::count_dictionary::CountDictionary;

#[derive(Debug, Clone)]
pub struct Relation<T: GnfsInteger> {
    pub a: T,
    pub b: T,
    pub algebraic_norm: T,
    pub rational_norm: T,
    pub algebraic_quotient: T,
    pub rational_quotient: T,
    pub algebraic_factorization: CountDictionary,
    pub rational_factorization: CountDictionary,
    pub is_persisted: bool,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T: GnfsInteger> Relation<T> {
    pub fn new(_gnfs: &GNFS<T>, a: &BigInt, b: &BigInt) -> Self {
        Relation {
            a: T::from_bigint(a).expect("Failed to convert a to native type"),
            b: T::from_bigint(b).expect("Failed to convert b to native type"),
            algebraic_norm: T::zero(),
            rational_norm: T::zero(),
            algebraic_quotient: T::zero(),
            rational_quotient: T::zero(),
            algebraic_factorization: CountDictionary::new(),
            rational_factorization: CountDictionary::new(),
            is_persisted: false,
            _phantom: PhantomData,
        }
    }

    pub fn is_smooth(&self) -> bool {
        self.is_rational_quotient_smooth() && self.is_algebraic_quotient_smooth()
    }

    pub fn is_rational_quotient_smooth(&self) -> bool {
        self.rational_quotient == T::one() || self.rational_quotient.is_zero()
    }

    pub fn is_algebraic_quotient_smooth(&self) -> bool {
        self.algebraic_quotient == T::one() || self.algebraic_quotient.is_zero()
    }

    pub fn apply(&self, x: &BigInt) -> BigInt {
        // Convert to BigInt for polynomial evaluation
        let a_bigint = self.a.to_bigint();
        let b_bigint = self.b.to_bigint();
        &a_bigint + &b_bigint * x
    }

    pub fn sieve(&mut self, gnfs: &GNFS<T>) {
        use log::debug;

        // OPTIMIZATION: Check rational first before computing expensive algebraic norm
        // Rational norm: a + b*m where m is the polynomial base
        let rational_norm_bigint = self.apply(&gnfs.polynomial_base);

        // Convert rational norm to native type
        if let Some(rational_norm) = T::from_bigint(&rational_norm_bigint) {
            self.rational_norm = rational_norm;
        } else {
            // Overflow - mark as non-smooth
            self.rational_quotient = T::from_i64(i64::MAX).unwrap_or(T::one());
            self.algebraic_quotient = T::from_i64(i64::MAX).unwrap_or(T::one());
            return;
        }

        // Handle negative rational norms: add -1 to factorization
        let rational_norm_for_comparison = rational_norm_bigint.clone();
        if rational_norm_for_comparison < BigInt::zero() {
            self.rational_factorization.add(&BigInt::from(-1));
        }

        // Use absolute value for factorization
        let abs_rational_norm = rational_norm_bigint.abs();

        // OPTIMIZATION: Sieve rational first (C# does this)
        // Only compute algebraic norm if rational is smooth
        let (rational_factors, rational_quotient) = FactorizationFactory::factor_with_base(
            &abs_rational_norm,
            &gnfs.prime_factor_base.rational_factor_base
        );

        self.rational_factorization.combine(&rational_factors);

        // Convert rational quotient to native type
        if let Some(quot) = T::from_bigint(&rational_quotient) {
            self.rational_quotient = quot;
        } else {
            // Quotient too large - not smooth
            self.rational_quotient = T::from_i64(i64::MAX).unwrap_or(T::one());
            self.algebraic_quotient = T::from_i64(i64::MAX).unwrap_or(T::one());
            return;
        }

        // Only continue if rational is smooth - EARLY EXIT saves algebraic norm computation
        if !self.is_rational_quotient_smooth() {
            // Not smooth on rational side, skip expensive algebraic norm calculation
            // Set algebraic quotient to a large value to indicate non-smooth
            self.algebraic_quotient = T::from_i64(i64::MAX).unwrap_or(T::one());
            self.algebraic_norm = T::zero();
            return;
        }

        // Rational is smooth, now compute algebraic norm
        // OPTIMIZATION: Use homogeneous evaluation (integer-only, no BigRational)
        // Formula: b^d * f(-a/b) = sum(c_i * (-a)^i * b^(d-i))
        let a_bigint = self.a.to_bigint();
        let b_bigint = self.b.to_bigint();

        // Use optimized homogeneous evaluation with negate_a=true for f(-a/b)
        let algebraic_norm_bigint = gnfs.current_polynomial.evaluate_homogeneous(
            &a_bigint,
            &b_bigint,
            true  // negate_a: computes f(-a/b) * b^d
        );

        // Convert algebraic norm to native type
        if let Some(algebraic_norm) = T::from_bigint(&algebraic_norm_bigint) {
            self.algebraic_norm = algebraic_norm;
        } else {
            // Overflow - mark as non-smooth
            self.algebraic_quotient = T::from_i64(i64::MAX).unwrap_or(T::one());
            return;
        }

        // Rational is smooth, now check algebraic
        if algebraic_norm_bigint < BigInt::zero() {
            self.algebraic_factorization.add(&BigInt::from(-1));
        }

        let abs_algebraic_norm = algebraic_norm_bigint.abs();
        let (algebraic_factors, algebraic_quotient) = FactorizationFactory::factor_with_base(
            &abs_algebraic_norm,
            &gnfs.prime_factor_base.algebraic_factor_base
        );

        self.algebraic_factorization.combine(&algebraic_factors);

        // Convert algebraic quotient to native type
        if let Some(quot) = T::from_bigint(&algebraic_quotient) {
            self.algebraic_quotient = quot;
        } else {
            // Quotient too large - not smooth
            self.algebraic_quotient = T::from_i64(i64::MAX).unwrap_or(T::one());
            return;
        }

        let is_smooth = self.is_smooth();
        if is_smooth || (self.a == T::one() && self.b <= T::from_i64(10).unwrap_or(T::one())) {
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

impl<T: GnfsInteger> PartialEq for Relation<T> {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b
    }
}

impl<T: GnfsInteger> Eq for Relation<T> {}

impl<T: GnfsInteger> Hash for Relation<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Convert to BigInt for hashing to maintain consistency
        self.a.to_bigint().hash(state);
        self.b.to_bigint().hash(state);
    }
}

impl<T: GnfsInteger> PartialOrd for Relation<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.a.cmp(&other.a).then(self.b.cmp(&other.b)))
    }
}

impl<T: GnfsInteger> Ord for Relation<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}