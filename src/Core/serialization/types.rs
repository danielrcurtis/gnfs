// src/core/serialization/types.rs

use num::{BigInt, Zero};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::str::FromStr;
use crate::core::gnfs::GNFS;
use crate::core::gnfs_integer::GnfsInteger;
use crate::core::directory_location::DirectoryLocations;
use crate::core::factor_base::FactorBase;
use crate::core::solution::Solution;
use crate::factor::factor_pair::FactorPair;
use crate::factor::factor_pair_collection::FactorPairCollection;
use crate::polynomial::polynomial::{Term, Polynomial};
use crate::relation_sieve::relation_container::RelationContainer;
use crate::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress;
use crate::relation_sieve::relation::Relation;
use crate::core::count_dictionary::CountDictionary;

#[derive(Serialize, Deserialize)]
pub struct SerializableGNFS {
    pub n: String,
    pub factorization: Option<SerializableSolution>,
    pub polynomial_degree: usize,
    pub polynomial_base: String,
    pub polynomial_collection: Vec<SerializablePolynomial>,
    pub current_polynomial: SerializablePolynomial,
    pub prime_factor_base: SerializableFactorBase,
    pub rational_factor_pair_collection: SerializableFactorPairCollection,
    pub algebraic_factor_pair_collection: SerializableFactorPairCollection,
    pub quadratic_factor_pair_collection: SerializableFactorPairCollection,
    pub save_locations: DirectoryLocations,
}

// TODO: Phase 3 - Re-implement From traits with proper generics support
// These conversions need to be reworked to handle GNFS<T> generics
/*
impl From<GNFS> for SerializableGNFS {
    fn from(gnfs: GNFS) -> Self {
        SerializableGNFS {
            n: gnfs.n.to_string(),
            factorization: gnfs.factorization.map(SerializableSolution::from),
            polynomial_degree: gnfs.polynomial_degree,
            polynomial_base: gnfs.polynomial_base.to_string(),
            polynomial_collection: gnfs.polynomial_collection.into_iter().map(SerializablePolynomial::from).collect(),
            current_polynomial: SerializablePolynomial::from(gnfs.current_polynomial),
            prime_factor_base: SerializableFactorBase::from(gnfs.prime_factor_base),
            rational_factor_pair_collection: SerializableFactorPairCollection::from(gnfs.rational_factor_pair_collection),
            algebraic_factor_pair_collection: SerializableFactorPairCollection::from(gnfs.algebraic_factor_pair_collection),
            quadratic_factor_pair_collection: SerializableFactorPairCollection::from(gnfs.quadratic_factor_pair_collection),
            save_locations: gnfs.save_locations,
        }
    }
}

impl From<SerializableGNFS> for GNFS {
    fn from(gnfs: SerializableGNFS) -> Self {
        GNFS {
            n: BigInt::parse_bytes(gnfs.n.as_bytes(), 10).unwrap(),
            factorization: gnfs.factorization.map(Solution::from),
            polynomial_degree: gnfs.polynomial_degree,
            polynomial_base: BigInt::parse_bytes(gnfs.polynomial_base.as_bytes(), 10).unwrap(),
            polynomial_collection: gnfs.polynomial_collection.into_iter().map(Polynomial::from).collect(),
            current_polynomial: Polynomial::from(gnfs.current_polynomial),
            current_relations_progress: PolyRelationsSieveProgress::default(),
            prime_factor_base: FactorBase::from(gnfs.prime_factor_base),
            rational_factor_pair_collection: FactorPairCollection::from(gnfs.rational_factor_pair_collection),
            algebraic_factor_pair_collection: FactorPairCollection::from(gnfs.algebraic_factor_pair_collection),
            quadratic_factor_pair_collection: FactorPairCollection::from(gnfs.quadratic_factor_pair_collection),
            save_locations: gnfs.save_locations,
        }
    }
}
*/

#[derive(Serialize, Deserialize)]
pub struct SerializableTerm {
    pub coefficient: String,
    pub exponent: usize,
}

impl From<Term> for SerializableTerm {
    fn from(term: Term) -> Self {
        SerializableTerm {
            coefficient: term.coefficient.to_string(),
            exponent: term.exponent,
        }
    }
}

impl From<SerializableTerm> for Term {
    fn from(term: SerializableTerm) -> Self {
        Term {
            coefficient: BigInt::parse_bytes(term.coefficient.as_bytes(), 10).unwrap(),
            exponent: term.exponent,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializablePolynomial {
    pub terms: Vec<SerializableTerm>,
}

impl From<Polynomial> for SerializablePolynomial {
    fn from(poly: Polynomial) -> Self {
        SerializablePolynomial {
            terms: poly.terms.into_iter()
                // Filter out zero coefficients before serialization
                .filter(|(_, coef)| !coef.is_zero())
                .map(|(exp, coef)| SerializableTerm {
                    coefficient: coef.to_string(),
                    exponent: exp,
                })
                .collect(),
        }
    }
}

impl From<SerializablePolynomial> for Polynomial {
    fn from(poly: SerializablePolynomial) -> Self {
        Polynomial {
            terms: poly.terms.into_iter()
                .map(|term| {
                    let coefficient = BigInt::from_str(&term.coefficient).unwrap();
                    (term.exponent, coefficient)
                })
                // Filter out zero coefficients to prevent divide-by-zero errors
                .filter(|(_, coef)| !coef.is_zero())
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializablePolyRelationsSieveProgress {
    pub a: String,
    pub b: String,
    pub smooth_relations_target_quantity: usize,
    pub value_range: String,
    pub max_b: String,
    pub smooth_relations_counter: usize,
    pub free_relations_counter: usize,
}

// Serialization - PolyRelationsSieveProgress already stores BigInt values
impl<T: GnfsInteger> From<PolyRelationsSieveProgress<T>> for SerializablePolyRelationsSieveProgress {
    fn from(progress: PolyRelationsSieveProgress<T>) -> Self {
        SerializablePolyRelationsSieveProgress {
            a: progress.a.to_string(),
            b: progress.b.to_string(),
            smooth_relations_target_quantity: progress.smooth_relations_target_quantity,
            value_range: progress.value_range.to_string(),
            max_b: progress.max_b.to_string(),
            smooth_relations_counter: progress.smooth_relations_counter,
            free_relations_counter: progress.free_relations_counter,
        }
    }
}

// Deserialization - PolyRelationsSieveProgress stores BigInt values directly
impl SerializablePolyRelationsSieveProgress {
    pub fn to_progress<T: GnfsInteger>(&self) -> PolyRelationsSieveProgress<T> {
        PolyRelationsSieveProgress::<T> {
            a: BigInt::parse_bytes(self.a.as_bytes(), 10).unwrap(),
            b: BigInt::parse_bytes(self.b.as_bytes(), 10).unwrap(),
            smooth_relations_target_quantity: self.smooth_relations_target_quantity,
            value_range: BigInt::parse_bytes(self.value_range.as_bytes(), 10).unwrap(),
            relations: RelationContainer::new(),
            max_b: BigInt::parse_bytes(self.max_b.as_bytes(), 10).unwrap(),
            smooth_relations_counter: self.smooth_relations_counter,
            free_relations_counter: self.free_relations_counter,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializableFactorPairCollection(pub Vec<SerializableFactorPair>);

impl From<FactorPairCollection> for SerializableFactorPairCollection {
    fn from(collection: FactorPairCollection) -> Self {
        SerializableFactorPairCollection(
            collection.0.into_iter().map(SerializableFactorPair::from).collect(),
        )
    }
}

impl From<SerializableFactorPairCollection> for FactorPairCollection {
    fn from(collection: SerializableFactorPairCollection) -> Self {
        FactorPairCollection(
            collection.0.into_iter().map(FactorPair::from).collect(),
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializableFactorPair {
    pub p: i128,
    pub r: i128,
}

impl From<FactorPair> for SerializableFactorPair {
    fn from(pair: FactorPair) -> Self {
        SerializableFactorPair {
            p: pair.p,
            r: pair.r,
        }
    }
}

impl From<SerializableFactorPair> for FactorPair {
    fn from(pair: SerializableFactorPair) -> Self {
        FactorPair {
            p: pair.p,
            r: pair.r,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableFactorBase {
    #[serde(rename = "RationalFactorBaseMax")]
    pub rational_factor_base_max: String,
    #[serde(rename = "AlgebraicFactorBaseMax")]
    pub algebraic_factor_base_max: String,
    #[serde(rename = "QuadraticFactorBaseMin")]
    pub quadratic_factor_base_min: String,
    #[serde(rename = "QuadraticFactorBaseMax")]
    pub quadratic_factor_base_max: String,
    #[serde(rename = "QuadraticBaseCount")]
    pub quadratic_base_count: i32,
    #[serde(skip)]
    pub rational_factor_base: Vec<String>,
    #[serde(skip)]
    pub algebraic_factor_base: Vec<String>,
    #[serde(skip)]
    pub quadratic_factor_base: Vec<String>,
}

impl From<FactorBase> for SerializableFactorBase {
    fn from(base: FactorBase) -> Self {
        SerializableFactorBase {
            rational_factor_base_max: base.rational_factor_base_max.to_string(),
            algebraic_factor_base_max: base.algebraic_factor_base_max.to_string(),
            quadratic_factor_base_min: base.quadratic_factor_base_min.to_string(),
            quadratic_factor_base_max: base.quadratic_factor_base_max.to_string(),
            quadratic_base_count: base.quadratic_base_count,
            rational_factor_base: base.rational_factor_base.iter().map(|b| b.to_string()).collect(),
            algebraic_factor_base: base.algebraic_factor_base.iter().map(|b| b.to_string()).collect(),
            quadratic_factor_base: base.quadratic_factor_base.iter().map(|b| b.to_string()).collect(),
        }
    }
}

impl From<SerializableFactorBase> for FactorBase {
    fn from(base: SerializableFactorBase) -> Self {
        FactorBase {
            rational_factor_base_max: BigInt::parse_bytes(base.rational_factor_base_max.as_bytes(), 10).unwrap(),
            algebraic_factor_base_max: BigInt::parse_bytes(base.algebraic_factor_base_max.as_bytes(), 10).unwrap(),
            quadratic_factor_base_min: BigInt::parse_bytes(base.quadratic_factor_base_min.as_bytes(), 10).unwrap(),
            quadratic_factor_base_max: BigInt::parse_bytes(base.quadratic_factor_base_max.as_bytes(), 10).unwrap(),
            quadratic_base_count: base.quadratic_base_count,
            rational_factor_base: base.rational_factor_base.iter().map(|b| BigInt::parse_bytes(b.as_bytes(), 10).unwrap()).collect(),
            algebraic_factor_base: base.algebraic_factor_base.iter().map(|b| BigInt::parse_bytes(b.as_bytes(), 10).unwrap()).collect(),
            quadratic_factor_base: base.quadratic_factor_base.iter().map(|b| BigInt::parse_bytes(b.as_bytes(), 10).unwrap()).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableSolution {
    pub p: String,
    pub q: String,
}

impl From<Solution> for SerializableSolution {
    fn from(solution: Solution) -> Self {
        SerializableSolution {
            p: solution.p.to_string(),
            q: solution.q.to_string(),
        }
    }
}

impl From<SerializableSolution> for Solution {
    fn from(solution: SerializableSolution) -> Self {
        Solution {
            p: BigInt::parse_bytes(solution.p.as_bytes(), 10).unwrap(),
            q: BigInt::parse_bytes(solution.q.as_bytes(), 10).unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializableRelationContainer {
    pub smooth_relations: Vec<SerializableRelation>,
    pub rough_relations: Vec<SerializableRelation>,
    pub free_relations: Vec<Vec<SerializableRelation>>,
}

// Serialization converts from any GnfsInteger type
impl<T: GnfsInteger> From<RelationContainer<T>> for SerializableRelationContainer {
    fn from(container: RelationContainer<T>) -> Self {
        SerializableRelationContainer {
            smooth_relations: container.smooth_relations.into_iter().map(SerializableRelation::from).collect(),
            rough_relations: container.rough_relations.into_iter().map(SerializableRelation::from).collect(),
            free_relations: container.free_relations.into_iter().map(|relations| {
                relations.into_iter().map(SerializableRelation::from).collect()
            }).collect(),
        }
    }
}

// Deserialization is generic - caller specifies target type T
impl SerializableRelationContainer {
    pub fn to_relation_container<T: GnfsInteger>(&self) -> RelationContainer<T> {
        let mut container = RelationContainer::new();
        container.smooth_relations = self.smooth_relations.iter().map(|r| r.to_relation::<T>()).collect();
        container.rough_relations = self.rough_relations.iter().map(|r| r.to_relation::<T>()).collect();
        container.free_relations = self.free_relations.iter().map(|relations| {
            relations.iter().map(|r| r.to_relation::<T>()).collect()
        }).collect();
        container
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableRelation {
    pub a: String,
    pub b: String,
    pub algebraic_norm: String,
    pub rational_norm: String,
    pub algebraic_quotient: String,
    pub rational_quotient: String,
    pub algebraic_factorization: SerializableCountDictionary,
    pub rational_factorization: SerializableCountDictionary,
    pub is_persisted: bool,
}

// Serialization converts from any GnfsInteger type by converting to BigInt strings
impl<T: GnfsInteger> From<Relation<T>> for SerializableRelation {
    fn from(relation: Relation<T>) -> Self {
        SerializableRelation {
            a: relation.a.to_bigint().to_string(),
            b: relation.b.to_bigint().to_string(),
            algebraic_norm: relation.algebraic_norm.to_bigint().to_string(),
            rational_norm: relation.rational_norm.to_bigint().to_string(),
            algebraic_quotient: relation.algebraic_quotient.to_bigint().to_string(),
            rational_quotient: relation.rational_quotient.to_bigint().to_string(),
            algebraic_factorization: SerializableCountDictionary::from(relation.algebraic_factorization),
            rational_factorization: SerializableCountDictionary::from(relation.rational_factorization),
            is_persisted: relation.is_persisted,
        }
    }
}

// CRITICAL FIX: Reference-based conversion to eliminate clone during disk serialization
// This prevents the 6GB memory spikes caused by cloning CountDictionaries full of BigInts
impl<T: GnfsInteger> From<&Relation<T>> for SerializableRelation {
    fn from(relation: &Relation<T>) -> Self {
        SerializableRelation {
            a: relation.a.to_bigint().to_string(),
            b: relation.b.to_bigint().to_string(),
            algebraic_norm: relation.algebraic_norm.to_bigint().to_string(),
            rational_norm: relation.rational_norm.to_bigint().to_string(),
            algebraic_quotient: relation.algebraic_quotient.to_bigint().to_string(),
            rational_quotient: relation.rational_quotient.to_bigint().to_string(),
            algebraic_factorization: SerializableCountDictionary::from(&relation.algebraic_factorization),
            rational_factorization: SerializableCountDictionary::from(&relation.rational_factorization),
            is_persisted: relation.is_persisted,
        }
    }
}

// Deserialization is generic - caller specifies target type T
impl SerializableRelation {
    pub fn to_relation<T: GnfsInteger>(&self) -> Relation<T> {
        use std::marker::PhantomData;
        let a_bigint = BigInt::parse_bytes(self.a.as_bytes(), 10).unwrap();
        let b_bigint = BigInt::parse_bytes(self.b.as_bytes(), 10).unwrap();
        let algebraic_norm_bigint = BigInt::parse_bytes(self.algebraic_norm.as_bytes(), 10).unwrap();
        let rational_norm_bigint = BigInt::parse_bytes(self.rational_norm.as_bytes(), 10).unwrap();
        let algebraic_quotient_bigint = BigInt::parse_bytes(self.algebraic_quotient.as_bytes(), 10).unwrap();
        let rational_quotient_bigint = BigInt::parse_bytes(self.rational_quotient.as_bytes(), 10).unwrap();

        Relation {
            a: T::from_bigint(&a_bigint).expect("Failed to convert a"),
            b: T::from_bigint(&b_bigint).expect("Failed to convert b"),
            algebraic_norm: T::from_bigint(&algebraic_norm_bigint).expect("Failed to convert algebraic_norm"),
            rational_norm: T::from_bigint(&rational_norm_bigint).expect("Failed to convert rational_norm"),
            algebraic_quotient: T::from_bigint(&algebraic_quotient_bigint).expect("Failed to convert algebraic_quotient"),
            rational_quotient: T::from_bigint(&rational_quotient_bigint).expect("Failed to convert rational_quotient"),
            algebraic_factorization: CountDictionary::from(self.algebraic_factorization.clone()),
            rational_factorization: CountDictionary::from(self.rational_factorization.clone()),
            is_persisted: self.is_persisted,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableCountDictionary(BTreeMap<String, String>);

impl From<CountDictionary> for SerializableCountDictionary {
    fn from(dict: CountDictionary) -> Self {
        let serializable_map = dict.0.into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        SerializableCountDictionary(serializable_map)
    }
}

// CRITICAL FIX: Reference-based conversion to eliminate clone during disk serialization
impl From<&CountDictionary> for SerializableCountDictionary {
    fn from(dict: &CountDictionary) -> Self {
        let serializable_map = dict.0.iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        SerializableCountDictionary(serializable_map)
    }
}

impl From<SerializableCountDictionary> for CountDictionary {
    fn from(dict: SerializableCountDictionary) -> Self {
        let original_map = dict.0.into_iter()
            .map(|(key, value)| (
                BigInt::parse_bytes(key.as_bytes(), 10).unwrap(),
                BigInt::parse_bytes(value.as_bytes(), 10).unwrap(),
            ))
            .collect();
        CountDictionary(original_map)
    }
}

impl Default for SerializableGNFS {
    fn default() -> Self {
        SerializableGNFS {
            n: String::default(),
            factorization: None,
            polynomial_degree: 0,
            polynomial_base: String::default(),
            polynomial_collection: Vec::default(),
            current_polynomial: SerializablePolynomial::default(),
            prime_factor_base: SerializableFactorBase::default(),
            rational_factor_pair_collection: SerializableFactorPairCollection::default(),
            algebraic_factor_pair_collection: SerializableFactorPairCollection::default(),
            quadratic_factor_pair_collection: SerializableFactorPairCollection::default(),
            save_locations: DirectoryLocations::default(),
        }
    }
}

impl Default for SerializablePolyRelationsSieveProgress {
    fn default() -> Self {
        SerializablePolyRelationsSieveProgress {
            a: String::default(),
            b: String::default(),
            smooth_relations_target_quantity: 0,
            value_range: String::default(),
            max_b: String::default(),
            smooth_relations_counter: 0,
            free_relations_counter: 0,
        }
    }
}

impl Default for SerializableFactorBase {
    fn default() -> Self {
        SerializableFactorBase {
            rational_factor_base_max: String::default(),
            algebraic_factor_base_max: String::default(),
            quadratic_factor_base_min: String::default(),
            quadratic_factor_base_max: String::default(),
            quadratic_base_count: 0,
            rational_factor_base: Vec::default(),
            algebraic_factor_base: Vec::default(),
            quadratic_factor_base: Vec::default(),
        }
    }
}

impl Default for SerializableFactorPairCollection {
    fn default() -> Self {
        SerializableFactorPairCollection(Vec::default())
    }
}

impl Default for SerializablePolynomial {
    fn default() -> Self {
        SerializablePolynomial {
            terms: Vec::default(),
        }
    }
}

impl Default for SerializableRelationContainer {
    fn default() -> Self {
        SerializableRelationContainer {
            smooth_relations: Vec::default(),
            rough_relations: Vec::default(),
            free_relations: Vec::default(),
        }
    }
}