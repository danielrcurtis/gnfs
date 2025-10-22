// src/core/serialization/types.rs

use num::BigInt;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::collections::BTreeMap;
use std::str::FromStr;
use crate::core::gnfs::GNFS;
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
            terms: poly.terms.into_iter().map(|(exp, coef)| SerializableTerm {
                coefficient: coef.to_string(),
                exponent: exp,
            }).collect(),
        }
    }
}

impl From<SerializablePolynomial> for Polynomial {
    fn from(poly: SerializablePolynomial) -> Self {
        Polynomial {
            terms: poly.terms.into_iter().map(|term| {
                let coefficient = BigInt::from_str(&term.coefficient).unwrap();
                (term.exponent, coefficient)
            }).collect(),
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

impl From<PolyRelationsSieveProgress> for SerializablePolyRelationsSieveProgress {
    fn from(progress: PolyRelationsSieveProgress) -> Self {
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

impl From<SerializablePolyRelationsSieveProgress> for PolyRelationsSieveProgress {
    fn from(progress: SerializablePolyRelationsSieveProgress) -> Self {
        PolyRelationsSieveProgress {
            a: BigInt::parse_bytes(progress.a.as_bytes(), 10).unwrap(),
            b: BigInt::parse_bytes(progress.b.as_bytes(), 10).unwrap(),
            smooth_relations_target_quantity: progress.smooth_relations_target_quantity,
            value_range: BigInt::parse_bytes(progress.value_range.as_bytes(), 10).unwrap(),
            relations: RelationContainer::new(),
            max_b: BigInt::parse_bytes(progress.max_b.as_bytes(), 10).unwrap(),
            smooth_relations_counter: progress.smooth_relations_counter,
            free_relations_counter: progress.free_relations_counter,
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

impl From<RelationContainer> for SerializableRelationContainer {
    fn from(container: RelationContainer) -> Self {
        SerializableRelationContainer {
            smooth_relations: container.smooth_relations.into_iter().map(SerializableRelation::from).collect(),
            rough_relations: container.rough_relations.into_iter().map(SerializableRelation::from).collect(),
            free_relations: container.free_relations.into_iter().map(|relations| {
                relations.into_iter().map(SerializableRelation::from).collect()
            }).collect(),
        }
    }
}

impl From<SerializableRelationContainer> for RelationContainer {
    fn from(container: SerializableRelationContainer) -> Self {
        RelationContainer {
            smooth_relations: container.smooth_relations.into_iter().map(Relation::from).collect(),
            rough_relations: container.rough_relations.into_iter().map(Relation::from).collect(),
            free_relations: container.free_relations.into_iter().map(|relations| {
                relations.into_iter().map(Relation::from).collect()
            }).collect(),
        }
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

impl From<Relation> for SerializableRelation {
    fn from(relation: Relation) -> Self {
        SerializableRelation {
            a: relation.a.to_string(),
            b: relation.b.to_string(),
            algebraic_norm: relation.algebraic_norm.to_string(),
            rational_norm: relation.rational_norm.to_string(),
            algebraic_quotient: relation.algebraic_quotient.to_string(),
            rational_quotient: relation.rational_quotient.to_string(),
            algebraic_factorization: SerializableCountDictionary::from(relation.algebraic_factorization),
            rational_factorization: SerializableCountDictionary::from(relation.rational_factorization),
            is_persisted: relation.is_persisted,
        }
    }
}

impl From<SerializableRelation> for Relation {
    fn from(relation: SerializableRelation) -> Self {
        Relation {
            a: BigInt::parse_bytes(relation.a.as_bytes(), 10).unwrap(),
            b: BigInt::parse_bytes(relation.b.as_bytes(), 10).unwrap(),
            algebraic_norm: BigInt::parse_bytes(relation.algebraic_norm.as_bytes(), 10).unwrap(),
            rational_norm: BigInt::parse_bytes(relation.rational_norm.as_bytes(), 10).unwrap(),
            algebraic_quotient: BigInt::parse_bytes(relation.algebraic_quotient.as_bytes(), 10).unwrap(),
            rational_quotient: BigInt::parse_bytes(relation.rational_quotient.as_bytes(), 10).unwrap(),
            algebraic_factorization: CountDictionary::from(relation.algebraic_factorization),
            rational_factorization: CountDictionary::from(relation.rational_factorization),
            is_persisted: relation.is_persisted,
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