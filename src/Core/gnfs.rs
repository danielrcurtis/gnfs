// src/core/gnfs.rs

use num::{BigInt, One, Zero};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::core::factor_base::FactorBase;
use crate::core::factor_pair::FactorPairCollection;
use crate::core::polynomial::Polynomial;
use crate::core::relations::PolyRelationsSieveProgress;
use crate::core::solution::Solution;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GNFS {
    pub n: BigInt,
    pub factorization: Option<Solution>,
    pub polynomial_degree: usize,
    pub polynomial_base: BigInt,
    pub polynomial_collection: Vec<Polynomial>,
    pub current_polynomial: Polynomial,
    pub current_relations_progress: PolyRelationsSieveProgress,
    pub prime_factor_base: FactorBase,
    pub rational_factor_pair_collection: FactorPairCollection,
    pub algebraic_factor_pair_collection: FactorPairCollection,
    pub quadratic_factor_pair_collection: FactorPairCollection,
    pub save_locations: DirectoryLocations,
}

impl GNFS {
    pub fn new(
        cancel_token: &CancellationToken,
        log_function: &mut dyn FnMut(String),
        n: &BigInt,
        polynomial_base: &BigInt,
        poly_degree: i32,
        prime_bound: &BigInt,
        relation_quantity: usize,
        relation_value_range: usize,
        created_new_data: bool,
    ) -> Self {
        let mut gnfs = GNFS {
            n: n.clone(),
            factorization: None,
            polynomial_degree: 0,
            polynomial_base: polynomial_base.clone(),
            polynomial_collection: Vec::new(),
            current_polynomial: Polynomial::default(),
            current_relations_progress: PolyRelationsSieveProgress::default(),
            prime_factor_base: FactorBase::default(),
            rational_factor_pair_collection: FactorPairCollection::default(),
            algebraic_factor_pair_collection: FactorPairCollection::default(),
            quadratic_factor_pair_collection: FactorPairCollection::default(),
            save_locations: DirectoryLocations::new(n),
        };

        if created_new_data || !gnfs.save_locations.save_directory.exists() {
            // New GNFS instance
            if !gnfs.save_locations.save_directory.exists() {
                std::fs::create_dir_all(&gnfs.save_locations.save_directory).unwrap();
                log_function(format!("Directory created: {:?}", gnfs.save_locations.save_directory));
            } else {
                if gnfs.save_locations.smooth_relations_save_file.exists() {
                    std::fs::remove_file(&gnfs.save_locations.smooth_relations_save_file).unwrap();
                }
                if gnfs.save_locations.rough_relations_save_file.exists() {
                    std::fs::remove_file(&gnfs.save_locations.rough_relations_save_file).unwrap();
                }
                if gnfs.save_locations.quadratic_factor_pair_save_file.exists() {
                    std::fs::remove_file(&gnfs.save_locations.quadratic_factor_pair_save_file).unwrap();
                }
                for free_relation_path in gnfs.save_locations.enumerate_free_relation_files() {
                    std::fs::remove_file(free_relation_path).unwrap();
                }
            }

            if poly_degree == -1 {
                gnfs.polynomial_degree = Self::calculate_degree(n);
            } else {
                gnfs.polynomial_degree = poly_degree as usize;
            }

            if cancel_token.is_cancelled() {
                return gnfs;
            }

            gnfs.construct_new_polynomial(polynomial_base, gnfs.polynomial_degree);
            log_function(format!("Polynomial constructed: {}", gnfs.current_polynomial));
            log_function(format!("Polynomial base: {}", gnfs.polynomial_base));

            if cancel_token.is_cancelled() {
                return gnfs;
            }

            gnfs.calculate_prime_factor_base_bounds(prime_bound);

            if cancel_token.is_cancelled() {
                return gnfs;
            }

            gnfs.set_prime_factor_bases();

            if cancel_token.is_cancelled() {
                return gnfs;
            }

            gnfs.new_factor_pair_collections(cancel_token);
            log_function("Factor bases populated.".to_string());

            if cancel_token.is_cancelled() {
                return gnfs;
            }

            gnfs.current_relations_progress = PolyRelationsSieveProgress::new(
                &gnfs,
                relation_quantity,
                relation_value_range,
            );
            log_function(format!("Relations container initialized. Target quantity: {}", relation_quantity));

            // TODO: Implement saving the state
            // Serialization::save_all(&gnfs);
        }

        gnfs
    }

    fn calculate_degree(n: &BigInt) -> usize {
        let base_10 = n.to_string().len();
        if base_10 < 65 {
            3
        } else if base_10 < 125 {
            4
        } else if base_10 < 225 {
            5
        } else if base_10 < 315 {
            6
        } else {
            7
        }
    }

    fn get_prime_bounds_approximation(&mut self) {
        let base_10 = self.n.to_string().len();
        let bound = if base_10 <= 10 {
            BigInt::from(100)
        } else if base_10 <= 18 {
            BigInt::from(base_10) * BigInt::from(1000)
        } else if base_10 <= 100 {
            BigInt::from(100000)
        } else if base_10 <= 150 {
            BigInt::from(250000)
        } else if base_10 <= 200 {
            BigInt::from(125000000)
        } else {
            BigInt::from(250000000)
        };

        self.set_prime_factor_bases();
    }

    pub fn calculate_prime_factor_base_bounds(&mut self, bound: &BigInt) {
        self.prime_factor_base = FactorBase::default();

        self.prime_factor_base.rational_factor_base_max = bound.clone();
        self.prime_factor_base.algebraic_factor_base_max = &self.prime_factor_base.rational_factor_base_max * 3;

        self.prime_factor_base.quadratic_base_count = Self::calculate_quadratic_base_size(self.polynomial_degree);

        self.prime_factor_base.quadratic_factor_base_min = &self.prime_factor_base.algebraic_factor_base_max + 20;
        // TODO: Implement PrimeFactory::get_approximate_value_from_index
        // self.prime_factor_base.quadratic_factor_base_max = PrimeFactory::get_approximate_value_from_index(
        //     (self.prime_factor_base.quadratic_factor_base_min + self.prime_factor_base.quadratic_base_count) as u64,
        // );

        // TODO: Implement logging
        // log_function(format!("Rational  Factor Base Bounds: Min: - Max: {}", self.prime_factor_base.rational_factor_base_max));
        // log_function(format!("Algebraic Factor Base Bounds: Min: - Max: {}", self.prime_factor_base.algebraic_factor_base_max));
        // log_function(format!("Quadratic Factor Base Bounds: Min: {} Max: {}", self.prime_factor_base.quadratic_factor_base_min, self.prime_factor_base.quadratic_factor_base_max));

        // TODO: Implement saving the state
        // Serialization::save_all(self);
        // log_function("Saved prime factor base bounds.".to_string());
    }

    // ...
}


// TODO: Implement Solution, Polynomial, PolyRelationsSieveProgress, and DirectoryLocations structs