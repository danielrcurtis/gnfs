// src/relation_sieve/poly_relations_sieve_progress.rs

use log::{debug, info};
use num::{BigInt, Integer};
use rayon::prelude::*;
use crate::integer_math::gcd::GCD;
use crate::core::sieve_range::SieveRange;
use crate::core::gnfs::GNFS;
use crate::relation_sieve::relation::Relation;
use crate::relation_sieve::relation_container::RelationContainer;
use crate::integer_math::prime_factory::PrimeFactory;
use crate::core::count_dictionary::CountDictionary;
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
}

impl PolyRelationsSieveProgress {
    pub fn new(gnfs: &GNFS, smooth_relations_target_quantity: isize, value_range: BigInt) -> Self {
        let required_for_matrix = Self::smooth_relations_required_for_matrix_step(gnfs);

        let target_quantity = if smooth_relations_target_quantity == -1 {
            required_for_matrix
        } else {
            std::cmp::max(smooth_relations_target_quantity as usize, required_for_matrix)
        };

        PolyRelationsSieveProgress {
            a: BigInt::from(0),
            b: BigInt::from(3),
            smooth_relations_target_quantity: target_quantity,
            value_range,
            relations: RelationContainer::new(),
            max_b: gnfs.prime_factor_base.algebraic_factor_base_max.clone(),
            smooth_relations_counter: 0,
            free_relations_counter: 0,
        }
    }
    
    pub fn smooth_relations_required_for_matrix_step(gnfs: &GNFS) -> usize {
        let mut prime_factory = PrimeFactory::new();
        PrimeFactory::get_index_from_value(&mut prime_factory, &gnfs.prime_factor_base.rational_factor_base_max) as usize
            + PrimeFactory::get_index_from_value(&mut prime_factory, &gnfs.prime_factor_base.algebraic_factor_base_max) as usize
            + gnfs.quadratic_factor_pair_collection.0.len()
            + 3
    }

    pub fn generate_relations(&mut self, gnfs: &GNFS, cancel_token: &CancellationToken) {
        self.smooth_relations_target_quantity = std::cmp::max(
            self.smooth_relations_target_quantity,
            Self::smooth_relations_required_for_matrix_step(gnfs),
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
            self.max_b += 100;  // Fixed: C# uses 100, not 1000
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
                debug!("Breaking because B ({}) > MaxB ({})", self.b, self.max_b);
                break;
            }

            debug!("About to call get_sieve_range_continuation with self.a = {}, self.value_range = {}", self.a, self.value_range);

            // Batch multiple B values together for better parallelization
            // This gives Rayon enough work to effectively use multiple cores
            let batch_size = 50; // Increased from 10 to 50 for more parallel work

            // Collect (A, B) pairs for a batch of B values
            let mut ab_pairs = Vec::new();
            let batch_start_b = self.b.clone();

            for b_offset in 0..batch_size {
                let current_b = &batch_start_b + BigInt::from(b_offset);
                if &current_b > &self.max_b {
                    break;
                }

                let a_values: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start_a, &self.value_range)
                    .collect();

                for a in a_values {
                    ab_pairs.push((a, current_b.clone()));
                }
            }

            let total_pairs = ab_pairs.len();
            let rayon_threads = rayon::current_num_threads();

            info!("=== PARALLEL BATCH START ===");
            info!("  Batch size (B values): {}", batch_size);
            info!("  Total (A,B) pairs: {}", total_pairs);
            info!("  Rayon threads: {}", rayon_threads);
            info!("  Work per thread (avg): {:.1}", total_pairs as f64 / rayon_threads as f64);
            info!("  B range: {} to {}", batch_start_b, &batch_start_b + batch_size - 1);

            use std::time::Instant;
            let parallel_start = Instant::now();

            // Use Rayon's filter_map + collect to avoid mutex contention
            let found: Vec<Relation> = ab_pairs.par_iter()
                .filter(|_| !cancel_token.is_cancellation_requested())
                .filter(|(a, b)| GCD::are_coprime(&[a.clone(), b.clone()]))
                .filter_map(|(a, b)| {
                    // Each thread creates and tests its own relation
                    let mut rel = Relation::new(gnfs, a, b);
                    rel.sieve(gnfs);

                    if rel.is_smooth() {
                        Some(rel)
                    } else {
                        None
                    }
                })
                .collect();  // Rayon's parallel collect - no mutex needed!

            let parallel_elapsed = parallel_start.elapsed();
            let num_found = found.len();

            // Update progress tracking
            self.relations.smooth_relations.extend(found);
            self.smooth_relations_counter += num_found;

            // Update B to the next batch
            self.b = &self.b + batch_size;
            self.a = start_a.clone();

            info!("=== PARALLEL BATCH COMPLETE ===");
            info!("  Time elapsed: {:.2}s", parallel_elapsed.as_secs_f64());
            info!("  Pairs processed: {}", total_pairs);
            info!("  Throughput: {:.0} pairs/sec", total_pairs as f64 / parallel_elapsed.as_secs_f64());
            info!("  Smooth relations found: {}", num_found);
            info!("  Total smooth relations: {}", self.relations.smooth_relations.len());
            info!("  Progress: {} / {} ({:.1}%)",
                  self.smooth_relations_counter,
                  self.smooth_relations_target_quantity,
                  100.0 * self.smooth_relations_counter as f64 / self.smooth_relations_target_quantity as f64);
            debug!("Now at B = {}", self.b);

        }
    }
    
    pub fn increase_target_quantity(&mut self, amount: usize) {
        self.smooth_relations_target_quantity += amount;
        // TODO: Re-enable serialization once GNFS serialization is re-implemented
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
        // TODO: Re-enable serialization once ownership is properly handled
        // free::single_solution(gnfs, &mut free_relation_solution);
        info!("{}", &format!("Added free relation solution: Relation count = {}", free_relation_solution.len()));
        self.free_relations_counter += 1;
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
            // TODO: Fix this to take GNFS as parameter
            // result.push_str(&relations
            //     .iter()
            //     .map(|rel| {
            //         let f = gnfs.current_polynomial.evaluate(&rel.a);
            //         if rel.b == BigInt::from(0) {
            //             String::new()
            //         } else {
            //             format!("ƒ({}) ≡ {} ≡ {} (mod {})", rel.a, f.clone(), f % &rel.b, rel.b)
            //         }
            //     })
            //     .collect::<Vec<_>>()
            //     .join("\n"));
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
        }
    }
}

impl std::fmt::Display for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Customize the formatting of Relation struct
        write!(f, "Relation {{ a: {}, b: {}, algebraic_norm: {}, rational_norm: {} }}", self.a, self.b, self.algebraic_norm, self.rational_norm)
    }
}
