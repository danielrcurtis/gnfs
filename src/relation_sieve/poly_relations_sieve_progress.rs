// src/relation_sieve/poly_relations_sieve_progress.rs

use std::sync::Arc;
use num::BigInt;
use serde::{Deserialize, Serialize};
use crate::integer_math::gcd::GCD;
use crate::core::sieve_range::SieveRange;
use crate::core::gnfs::GNFS;
use crate::relation_sieve::relation::Relation;
use crate::relation_sieve::relation_container::RelationContainer;
use crate::serialization::save::relations::{Smooth, Rough, Free};
use crate::integer_math::prime_factory::PrimeFactory;
use crate::core::count_dictionary::CountDictionary;
use crate::integer_math::factorization_factory::FactorizationFactory;

#[derive(Serialize, Deserialize)]
pub struct PolyRelationsSieveProgress {
    pub a: BigInt,
    pub b: BigInt,
    pub smooth_relations_target_quantity: usize,
    pub value_range: BigInt,
    pub relations: RelationContainer,
    pub max_b: BigInt,
    pub smooth_relations_counter: usize,
    pub free_relations_counter: usize,
    #[serde(skip)]
    pub gnfs: Arc<GNFS>,
}

impl PolyRelationsSieveProgress {
    pub fn new(gnfs: Arc<GNFS>, smooth_relations_target_quantity: isize, value_range: BigInt) -> Self {
        let mut progress = PolyRelationsSieveProgress {
            a: BigInt::from(0),
            b: BigInt::from(3),
            smooth_relations_target_quantity: 0,
            value_range,
            relations: RelationContainer::new(),
            max_b: BigInt::from(0),
            smooth_relations_counter: 0,
            free_relations_counter: 0,
            gnfs,
        };

        if smooth_relations_target_quantity == -1 {
            progress.smooth_relations_target_quantity = progress.smooth_relations_required_for_matrix_step();
        } else {
            progress.smooth_relations_target_quantity = std::cmp::max(
                smooth_relations_target_quantity as usize,
                progress.smooth_relations_required_for_matrix_step(),
            );
        }

        if progress.max_b == BigInt::from(0) {
            progress.max_b = progress.gnfs.prime_factor_base.algebraic_factor_base_max.clone();
        }

        progress
    }

    pub fn smooth_relations_required_for_matrix_step(&self) -> usize {
        PrimeFactory::get_index_from_value(&self.gnfs.prime_factor_base.rational_factor_base_max)
            + PrimeFactory::get_index_from_value(&self.gnfs.prime_factor_base.algebraic_factor_base_max)
            + self.gnfs.quadratic_factor_pair_collection.len()
            + 3
    }

    pub fn generate_relations(&mut self, cancel_token: &CancellationToken) {
        if !self.relations.smooth_relations.is_empty() {
            Smooth::append(&self.gnfs);
        }

        self.smooth_relations_target_quantity = std::cmp::max(
            self.smooth_relations_target_quantity,
            self.smooth_relations_required_for_matrix_step(),
        );

        if self.a >= self.value_range {
            self.value_range += BigInt::from(200);
        }

        self.value_range = if self.value_range.is_even() {
            &self.value_range + 1
        } else {
            self.value_range.clone()
        };

        self.a = if self.a.is_even() {
            &self.a + 1
        } else {
            self.a.clone()
        };

        let start_a = self.a.clone();

        while &self.b >= &self.max_b {
            self.max_b += 100;
        }

        self.gnfs.log_message(&format!(
            "GenerateRelations: TargetQuantity = {}, ValueRange = {}, A = {}, B = {}, Max B = {}",
            self.smooth_relations_target_quantity, self.value_range, self.a, self.b, self.max_b
        ));

        while self.smooth_relations_counter < self.smooth_relations_target_quantity {
            if cancel_token.is_cancellation_requested() {
                break;
            }

            if &self.b > &self.max_b {
                break;
            }

            for a in SieveRange::get_sieve_range_continuation(&self.a, &self.value_range) {
                if cancel_token.is_cancellation_requested() {
                    break;
                }

                self.a = a;
                if GCD::are_coprime(&self.a, &self.b) {
                    let mut rel = Relation::new(&self.gnfs, &self.a, &self.b);
                    rel.sieve(self);

                    let smooth = rel.is_smooth();
                    if smooth {
                        Smooth::append(&self.gnfs, &rel);
                        self.relations.smooth_relations.push(rel);
                    }
                }
            }

            if cancel_token.is_cancellation_requested() {
                break;
            }

            self.b += 1;
            self.a = start_a.clone();

            self.gnfs.log_message(&format!("B = {}", self.b));
            self.gnfs.log_message(&format!("SmoothRelations.Count: {}", self.relations.smooth_relations.len()));
        }
    }

    pub fn increase_target_quantity(&mut self, amount: usize) {
        self.smooth_relations_target_quantity += amount;
        save::gnfs(&self.gnfs);
    }

    pub fn purge_prime_rough_relations(&mut self) {
        let mut rough_relations = self.relations.rough_relations.clone();

        let to_remove_alg: Vec<_> = rough_relations
            .iter()
            .filter(|r| &r.algebraic_quotient != &BigInt::from(1) && FactorizationFactory::is_probable_prime(&r.algebraic_quotient))
            .cloned()
            .collect();

        rough_relations = rough_relations
            .into_iter()
            .filter(|r| !to_remove_alg.contains(r))
            .collect();

        self.relations.rough_relations = rough_relations.clone();

        let to_remove_rational: Vec<_> = rough_relations
            .iter()
            .filter(|r| &r.rational_quotient != &BigInt::from(1) && FactorizationFactory::is_probable_prime(&r.rational_quotient))
            .cloned()
            .collect();

        rough_relations = rough_relations
            .into_iter()
            .filter(|r| !to_remove_rational.contains(r))
            .collect();

        self.relations.rough_relations = rough_relations;
    }

    pub fn add_free_relation_solution(&mut self, free_relation_solution: Vec<Relation>) {
        self.relations.free_relations.push(free_relation_solution.clone());
        Free::single_solution(&self.gnfs, &free_relation_solution);
        self.gnfs.log_message(&format!("Added free relation solution: Relation count = {}", free_relation_solution.len()));
    }

    pub fn format_relations(&self, relations: &[Relation]) -> String {
        let mut result = String::new();

        result.push_str("Smooth relations:\n");
        result.push_str("\t_______________________________________________\n");
        result.push_str(&format!("\t|   A   |  B | ALGEBRAIC_NORM | RATIONAL_NORM | \t\tRelations count: {} Target quantity: {}\n", self.relations.smooth_relations.len(), self.smooth_relations_target_quantity));
        result.push_str("\t```````````````````````````````````````````````\n");

        for rel in relations.iter().sorted_by(|a, b| (a.a * a.b).cmp(&(b.a * b.b)).reverse()) {
            result.push_str(&format!("{}\n", rel.to_string()));
            result.push_str(&format!("Algebraic {}\n", rel.algebraic_factorization.format_string_as_factorization()));
            result.push_str(&format!("Rational  {}\n", rel.rational_factorization.format_string_as_factorization()));
            result.push_str("\n");
        }
        result.push_str("\n");

        result
    }
}

impl ToString for PolyRelationsSieveProgress {
    fn to_string(&self) -> String {
        if !self.relations.free_relations.is_empty() {
            let mut result = String::new();

            let relations = &self.relations.free_relations[0];

            result.push_str(&self.format_relations(relations));

            let algebraic = relations.iter().map(|rel| rel.algebraic_norm.clone()).product();
            let rational = relations.iter().map(|rel| rel.rational_norm.clone()).product();

            let is_algebraic_square = algebraic.is_square();
            let is_rational_square = rational.is_square();

            let mut alg_count_dict = CountDictionary::new();
            for rel in relations {
                alg_count_dict.combine(&rel.algebraic_factorization);
            }

            result.push_str("---\n");
            result.push_str(&format!("Rational  ∏(a+mb): IsSquare? {} : {}\n", is_rational_square, rational));
            result.push_str(&format!("Algebraic ∏ƒ(a/b): IsSquare? {} : {}\n", is_algebraic_square, algebraic));
            result.push_str("\n");
            result.push_str(&format!("Algebraic factorization (as prime ideals): {}\n", alg_count_dict.format_string_as_factorization()));
            result.push_str("\n");

            result.push_str("\n");
            result.push_str("\n");
            result.push_str(&relations
                .iter()
                .map(|rel| {
                    let f = self.gnfs.current_polynomial.evaluate(&rel.a);
                    if rel.b == BigInt::from(0) {
                        String::new()
                    } else {
                        format!("ƒ({}) ≡ {} ≡ {} (mod {})", rel.a, f, f % &rel.b, rel.b)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"));
            result.push_str("\n");

            result
        } else {
            self.format_relations(&self.relations.smooth_relations)
        }
    }
}