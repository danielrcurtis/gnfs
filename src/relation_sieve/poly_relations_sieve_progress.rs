// src/relation_sieve/poly_relations_sieve_progress.rs


use std::sync::{Arc, Weak};
use log::{debug, info};
use num::{BigInt, Integer};
use crate::integer_math::gcd::GCD;
use crate::core::sieve_range::SieveRange;
use crate::core::gnfs::GNFS;
use crate::relation_sieve::relation::Relation;
use crate::relation_sieve::relation_container::RelationContainer;
use crate::core::serialization::save::relations::{smooth, free};
use crate::integer_math::prime_factory::PrimeFactory;
use crate::core::count_dictionary::CountDictionary;
use crate::core::serialization::save;
use crate::integer_math::factorization_factory::FactorizationFactory;
use crate::core::cancellation_token::CancellationToken;
use crate::square_root::square_finder::is_square;

#[derive(Debug, Clone)]
pub struct PolyRelationsSieveProgress {
    pub a: BigInt,
    pub b: BigInt,
    pub smooth_relations_target_quantity: usize,
    pub value_range: BigInt,
    pub relations: RelationContainer,
    pub max_b: BigInt,
    pub smooth_relations_counter: usize,
    pub free_relations_counter: usize,
    pub gnfs: Weak<GNFS>,
}

impl PolyRelationsSieveProgress {
    pub fn new(gnfs: Weak<GNFS>, smooth_relations_target_quantity: isize, value_range: BigInt) -> Self {
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
            if let Some(gnfs) = progress.gnfs.upgrade() {
                progress.max_b = gnfs.prime_factor_base.algebraic_factor_base_max.clone();
            }
        }
    
        progress
    }
    
    pub fn smooth_relations_required_for_matrix_step(&self) -> usize {
        if let Some(gnfs) = self.gnfs.upgrade() {
            let mut prime_factory = PrimeFactory::new();
            PrimeFactory::get_index_from_value(&mut prime_factory, &gnfs.prime_factor_base.rational_factor_base_max) as usize
                + PrimeFactory::get_index_from_value(&mut prime_factory, &gnfs.prime_factor_base.algebraic_factor_base_max) as usize
                + gnfs.quadratic_factor_pair_collection.0.len()
                + 3
        } else {
            0
        }
    }

    pub fn generate_relations(&mut self, cancel_token: &CancellationToken) {
        if let Some(gnfs) = self.gnfs.upgrade() {
            let mut gnfs = (*gnfs).clone();
            smooth::append(&mut gnfs);
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
            self.max_b += 1000;
        }
    
        
        debug!("{}", format!(
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
    
                if GCD::are_coprime(&[self.a.clone(), self.b.clone()]) {
                    if let Some(gnfs) = self.gnfs.upgrade() {
                        let mut rel = Relation::new(&gnfs, &self.a, &self.b);
                        rel.sieve(&gnfs, self);
                        let smooth = rel.is_smooth();
                        if smooth {
                            self.relations.smooth_relations.push(rel);
                            self.smooth_relations_counter += 1;
                        }
                    }
                }
            }
    
            if cancel_token.is_cancellation_requested() {
                break;
            }
    
            self.b += 1;
            self.a = start_a.clone();
    
            
            debug!("{}", &format!("B = {}", self.b));
            debug!("{}", &format!("SmoothRelations.Count: {}", self.relations.smooth_relations.len()));
            
        }
    
        if let Some(gnfs) = self.gnfs.upgrade() {
            let mut gnfs = (*gnfs).clone();
            smooth::append(&mut gnfs);
        }
    }
    
    pub fn increase_target_quantity(&mut self, amount: usize) {
        self.smooth_relations_target_quantity += amount;
        if let Some(gnfs) = self.gnfs.upgrade() {
            save::gnfs(&gnfs);
        }
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

    pub fn add_free_relation_solution(&mut self, mut free_relation_solution: Vec<Relation>) {
        self.relations.free_relations.push(free_relation_solution.clone());
        if let Some(arc_gnfs) = self.gnfs.upgrade() {
            if let Some(gnfs) = Arc::get_mut(&mut arc_gnfs.clone()) {
                free::single_solution(gnfs, &mut free_relation_solution);
                info!("{}", &format!("Added free relation solution: Relation count = {}", free_relation_solution.len()));
            }
        }
    }
    

    pub fn format_relations(&self, relations: &[Relation]) -> String {
        let mut result = String::new();
    
        result.push_str("Smooth relations:\n");
        result.push_str("\t_______________________________________________\n");
        result.push_str(&format!("\t|   A   |  B | ALGEBRAIC_NORM | RATIONAL_NORM | \t\tRelations count: {} Target quantity: {}\n", self.relations.smooth_relations.len(), self.smooth_relations_target_quantity));
        result.push_str("\t```````````````````````````````````````````````\n");
    
        let mut sorted_relations: Vec<_> = relations.iter().collect();
        sorted_relations.sort_by(|a, b| (b.a.clone() * b.b.clone()).cmp(&(a.a.clone() * a.b.clone())));
    
        for rel in sorted_relations {
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

            let algebraic: BigInt = relations.iter().map(|rel| rel.algebraic_norm.clone()).product();
            let rational: BigInt = relations.iter().map(|rel| rel.rational_norm.clone()).product();

            let is_algebraic_square = is_square(&algebraic); // look at abstract algebraic factorization  <----
            let is_rational_square = is_square(&rational);

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
                    let f = self.gnfs.upgrade().unwrap().current_polynomial.evaluate(&rel.a);
                    if rel.b == BigInt::from(0) {
                        String::new()
                    } else {
                        format!("ƒ({}) ≡ {} ≡ {} (mod {})", rel.a, f.clone(), f % &rel.b, rel.b)
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

impl Default for PolyRelationsSieveProgress {
    fn default() -> Self {
        PolyRelationsSieveProgress {
            a: BigInt::from(0),
            b: BigInt::from(3),
            smooth_relations_target_quantity: 0,
            value_range: BigInt::from(0),
            relations: RelationContainer::new(),
            max_b: BigInt::from(0),
            smooth_relations_counter: 0,
            free_relations_counter: 0,
            gnfs: Weak::new(),
        }
    }
}

impl std::fmt::Display for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Customize the formatting of Relation struct
        write!(f, "Relation {{ a: {}, b: {}, algebraic_norm: {}, rational_norm: {} }}", self.a, self.b, self.algebraic_norm, self.rational_norm)
    }
}
