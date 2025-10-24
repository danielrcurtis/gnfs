// src/relation_sieve/poly_relations_sieve_progress.rs

use log::{debug, info};
use num::{BigInt, Integer};
use rayon::prelude::*;
use crate::integer_math::gcd::GCD;
use crate::core::sieve_range::SieveRange;
use crate::core::gnfs::GNFS;
use crate::core::gnfs_integer::GnfsInteger;
use crate::relation_sieve::relation::Relation;
use crate::relation_sieve::relation_container::RelationContainer;
use crate::integer_math::prime_factory::PrimeFactory;
use crate::core::count_dictionary::CountDictionary;
use crate::integer_math::factorization_factory::FactorizationFactory;
use crate::core::cancellation_token::CancellationToken;
use crate::square_root::square_finder::is_square;
use crate::config::BufferConfig;

#[derive(Debug, Clone)]
pub struct PolyRelationsSieveProgress<T: GnfsInteger> {
    pub a: BigInt,
    pub b: BigInt,
    pub smooth_relations_target_quantity: usize,
    pub value_range: BigInt,
    pub relations: RelationContainer<T>,
    pub max_b: BigInt,
    pub smooth_relations_counter: usize,
    pub free_relations_counter: usize,
}

impl<T: GnfsInteger> PolyRelationsSieveProgress<T> {
    pub fn new(gnfs: &GNFS<T>, smooth_relations_target_quantity: isize, value_range: BigInt) -> Self {
        Self::with_config(gnfs, smooth_relations_target_quantity, value_range, BufferConfig::default())
    }

    pub fn with_config(gnfs: &GNFS<T>, smooth_relations_target_quantity: isize, value_range: BigInt, buffer_config: BufferConfig) -> Self {
        let required_for_matrix = Self::smooth_relations_required_for_matrix_step(gnfs);

        let target_quantity = if smooth_relations_target_quantity == -1 {
            required_for_matrix
        } else {
            std::cmp::max(smooth_relations_target_quantity as usize, required_for_matrix)
        };

        // Get digit count for size estimation
        let digit_count = gnfs.n.to_string().len();

        PolyRelationsSieveProgress {
            a: BigInt::from(0),
            b: BigInt::from(3),
            smooth_relations_target_quantity: target_quantity,
            value_range,
            relations: RelationContainer::with_config(buffer_config, digit_count),
            max_b: gnfs.prime_factor_base.algebraic_factor_base_max.clone(),
            smooth_relations_counter: 0,
            free_relations_counter: 0,
        }
    }
    
    pub fn smooth_relations_required_for_matrix_step(gnfs: &GNFS<T>) -> usize {
        let mut prime_factory = PrimeFactory::new();
        PrimeFactory::get_index_from_value(&mut prime_factory, &gnfs.prime_factor_base.rational_factor_base_max) as usize
            + PrimeFactory::get_index_from_value(&mut prime_factory, &gnfs.prime_factor_base.algebraic_factor_base_max) as usize
            + gnfs.quadratic_factor_pair_collection.0.len()
            + 3
    }

    pub fn generate_relations(&mut self, gnfs: &GNFS<T>, cancel_token: &CancellationToken) {
        self.smooth_relations_target_quantity = std::cmp::max(
            self.smooth_relations_target_quantity,
            Self::smooth_relations_required_for_matrix_step(gnfs),
        );
    
        // CRITICAL FIX: Cap value_range to prevent unbounded memory growth
        // Previously this would grow: 50 → 250 → 450 → 650 → ... causing 80GB spikes
        const MAX_VALUE_RANGE_GLOBAL: i64 = 150;
        if self.a >= self.value_range {
            self.value_range += BigInt::from(200);
            // Enforce maximum to prevent exponential memory growth
            if self.value_range > BigInt::from(MAX_VALUE_RANGE_GLOBAL) {
                self.value_range = BigInt::from(MAX_VALUE_RANGE_GLOBAL);
            }
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

            // Increase max_b if needed instead of breaking
            if &self.b > &self.max_b {
                self.max_b = &self.b + 100;
                debug!("Increased MaxB to {} (current B = {})", self.max_b, self.b);
            }

            debug!("About to call get_sieve_range_continuation with self.a = {}, self.value_range = {}", self.a, self.value_range);

            // AGGRESSIVE MEMORY LIMITS FOR LARGE NUMBERS:
            // The issue: Even with disk streaming, temporary BigInt allocations during
            // sieving use massive memory. For 11-digit numbers, we must drastically limit
            // the number of (a,b) pairs processed per iteration.
            //
            // Strategy:
            // - Cap value_range much lower (150 instead of 400)
            // - Always process 1 B at a time to minimize temp allocations
            // - Let disk streaming handle smooth relations

            // VERY AGGRESSIVE value_range cap for memory safety
            const MAX_VALUE_RANGE: i64 = 150;
            let effective_value_range = if self.value_range > BigInt::from(MAX_VALUE_RANGE) {
                BigInt::from(MAX_VALUE_RANGE)
            } else {
                self.value_range.clone()
            };

            // Process multiple B values per batch to amortize parallelism overhead
            // With value_range ~150 and batch_size=16: ~2400 (A,B) pairs per parallel batch
            // This gives each of 8 threads ~300 pairs, reducing synchronization overhead
            let batch_size = gnfs.buffer_config.batch_size;

            let batch_start_b = self.b.clone();

            use std::time::Instant;
            let parallel_start = Instant::now();

            let mut total_pairs = 0;
            // MEMORY FIX: Preallocate with realistic capacity to prevent rayon over-allocation
            // Expected: ~0.1% of pairs are smooth, batch_size=16, value_range=150
            // Conservative estimate: 16-80 smooth relations per batch
            let expected_smooth_relations = 80;
            let mut all_found = Vec::with_capacity(expected_smooth_relations);

            // Process B values one at a time, with aggressive memory cleanup
            for b_offset in 0..batch_size {
                let current_b = &batch_start_b + BigInt::from(b_offset);
                // Skip this B value if it exceeds max_b (max_b increases in outer loop)
                if &current_b > &self.max_b {
                    continue;  // Changed from break to continue
                }

                // Generate A values for this B - use capped value_range
                let a_iter = SieveRange::get_sieve_range_continuation(&start_a, &effective_value_range);

                // Collect only coprime A values (filters early)
                let a_values: Vec<BigInt> = a_iter
                    .filter(|a| !cancel_token.is_cancellation_requested())
                    .filter(|a| GCD::are_coprime_pair(a, &current_b))
                    .collect();

                let pairs_for_this_b = a_values.len();
                total_pairs += pairs_for_this_b;

                // MEMORY LEAK INVESTIGATION: Track a_values size
                debug!("MEMORY: a_values: len={}, capacity={}, estimated_mem={}KB",
                       a_values.len(),
                       a_values.capacity(),
                       (a_values.capacity() * std::mem::size_of::<BigInt>()) / 1024);

                // Process in parallel, but immediately drop non-smooth Relations
                // KEY OPTIMIZATION: Relation<T> now uses native types internally
                let found_for_this_b: Vec<Relation<T>> = a_values
                    .par_iter()
                    .filter_map(|a| {
                        let mut rel = Relation::new(gnfs, a, &current_b);
                        rel.sieve(gnfs);

                        if rel.is_smooth() {
                            Some(rel)
                        } else {
                            None  // Non-smooth Relations dropped here
                        }
                    })
                    .collect();

                all_found.extend(found_for_this_b);
                // Explicit drop to free memory immediately
                drop(a_values);
            }

            let parallel_elapsed = parallel_start.elapsed();
            let num_found = all_found.len();

            info!("=== PARALLEL BATCH COMPLETE ===");
            info!("  B values processed: {}", batch_size);
            info!("  Total (A,B) pairs: {}", total_pairs);
            info!("  Time elapsed: {:.2}s", parallel_elapsed.as_secs_f64());
            info!("  Throughput: {:.0} pairs/sec", total_pairs as f64 / parallel_elapsed.as_secs_f64());
            info!("  Smooth relations found: {}", num_found);
            info!("  MEMORY: all_found len={}, capacity={}, est_size={}MB",
                  all_found.len(),
                  all_found.capacity(),
                  (all_found.capacity() * std::mem::size_of::<Relation<T>>()) / (1024 * 1024));

            // MEMORY LEAK INVESTIGATION: Track vector capacity
            debug!("MEMORY: all_found before transfer: len={}, capacity={}, waste={}",
                   all_found.len(),
                   all_found.capacity(),
                   all_found.capacity().saturating_sub(all_found.len()));

            // CRITICAL FIX: Shrink all_found before transferring to reclaim excess capacity
            all_found.shrink_to_fit();

            // Update progress tracking - use streaming to avoid memory accumulation
            if let Err(e) = self.relations.add_smooth_relations(all_found) {
                log::error!("Failed to add smooth relations: {}", e);
            }
            self.smooth_relations_counter += num_found;

            // Update B to the next batch
            self.b = &self.b + batch_size;
            // Advance A to the end of the range (like C# reference implementation)
            self.a = &start_a + &effective_value_range;

            info!("  Total smooth relations: {}", self.relations.smooth_relations_count());
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
            .filter(|r| r.algebraic_quotient != T::one() && FactorizationFactory::is_probable_prime(&r.algebraic_quotient.to_bigint()))
            .cloned()
            .collect();

        rough_relations = rough_relations
            .into_iter()
            .filter(|r| !to_remove_alg.contains(r))
            .collect();

        self.relations.rough_relations = rough_relations.clone();

        let to_remove_rational: Vec<_> = rough_relations
            .iter()
            .filter(|r| r.rational_quotient != T::one() && FactorizationFactory::is_probable_prime(&r.rational_quotient.to_bigint()))
            .cloned()
            .collect();

        rough_relations = rough_relations
            .into_iter()
            .filter(|r| !to_remove_rational.contains(r))
            .collect();

        self.relations.rough_relations = rough_relations;
    }

    pub fn add_free_relation_solution(&mut self, free_relation_solution: Vec<Relation<T>>) {
        self.relations.free_relations.push(free_relation_solution.clone());
        // TODO: Re-enable serialization once ownership is properly handled
        // free::single_solution(gnfs, &mut free_relation_solution);
        info!("{}", &format!("Added free relation solution: Relation count = {}", free_relation_solution.len()));
        self.free_relations_counter += 1;
    }
    

    pub fn format_relations(&self, relations: &[Relation<T>]) -> String {
        let mut result = String::new();

        result.push_str("Smooth relations:\n");
        result.push_str("\t_______________________________________________\n");
        result.push_str(&format!("\t|   A   |  B | ALGEBRAIC_NORM | RATIONAL_NORM | \t\tRelations count: {} Target quantity: {}\n", self.relations.smooth_relations_count(), self.smooth_relations_target_quantity));
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

impl<T: GnfsInteger> ToString for PolyRelationsSieveProgress<T> {
    fn to_string(&self) -> String {
        if !self.relations.free_relations.is_empty() {
            let mut result = String::new();

            let relations = &self.relations.free_relations[0];

            result.push_str(&self.format_relations(relations));

            let algebraic: BigInt = relations.iter().map(|rel| rel.algebraic_norm.to_bigint()).product();
            let rational: BigInt = relations.iter().map(|rel| rel.rational_norm.to_bigint()).product();

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

impl<T: GnfsInteger> Default for PolyRelationsSieveProgress<T> {
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

impl<T: GnfsInteger> std::fmt::Display for Relation<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Customize the formatting of Relation struct
        write!(f, "Relation {{ a: {}, b: {}, algebraic_norm: {}, rational_norm: {} }}", self.a, self.b, self.algebraic_norm, self.rational_norm)
    }
}
