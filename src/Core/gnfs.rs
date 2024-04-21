// src/core/gnfs.rs

use log::info;
use num::{BigInt, ToPrimitive, Zero};
use std::path::{Path,PathBuf};
use std::sync::{atomic::AtomicBool, Arc};
use std::iter::Iterator;
use crate::core::factor_base::FactorBase;
use crate::factor::factor_pair_collection::{FactorPairCollection, Factory};
use crate::polynomial::polynomial::Polynomial;
use crate::polynomial::polynomial::Term;
use crate::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress;
use crate::relation_sieve::relation::Relation;
use crate::core::solution::Solution;
use crate::core::directory_location::DirectoryLocations;
use crate::core::cancellation_token::CancellationToken;
use crate::integer_math::prime_factory::PrimeFactory;

#[derive(Debug, Clone)]
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
            save_locations: DirectoryLocations::new(&DirectoryLocations::get_unique_name_from_n(&n)),
        };

        if created_new_data || !Path::new(&gnfs.save_locations.save_directory).exists() {
            // New GNFS instance
            if !Path::new(&gnfs.save_locations.save_directory).exists() {
                std::fs::create_dir_all(&gnfs.save_locations.save_directory).unwrap();
                info!("Directory created: {:?}", gnfs.save_locations.save_directory);
            } else {
                if Path::new(&gnfs.save_locations.smooth_relations_filepath).exists() {
                    std::fs::remove_file(&gnfs.save_locations.smooth_relations_filepath).unwrap();
                }
                if Path::new(&gnfs.save_locations.rough_relations_filepath).exists() {
                    std::fs::remove_file(&gnfs.save_locations.rough_relations_filepath).unwrap();
                }
                if Path::new(&gnfs.save_locations.rational_factor_pair_filepath).exists() {
                    std::fs::remove_file(&gnfs.save_locations.rational_factor_pair_filepath).unwrap();
                }

                if Path::new(&gnfs.save_locations.algebraic_factor_pair_filepath).exists() {
                    std::fs::remove_file(&gnfs.save_locations.algebraic_factor_pair_filepath).unwrap();
                }
                if Path::new(&gnfs.save_locations.quadratic_factor_pair_filepath).exists() {
                    std::fs::remove_file(&gnfs.save_locations.quadratic_factor_pair_filepath).unwrap();
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

            if cancel_token.is_cancellation_requested() {
                return gnfs;
            }

            gnfs.construct_new_polynomial(polynomial_base, gnfs.polynomial_degree);
            info!("Polynomial constructed: {}", gnfs.current_polynomial);
            info!("Polynomial base: {}", gnfs.polynomial_base);

            if cancel_token.is_cancellation_requested() {
                return gnfs;
            }

            gnfs.calculate_prime_factor_base_bounds(prime_bound);

            if cancel_token.is_cancellation_requested() {
                return gnfs;
            }

            gnfs.set_prime_factor_bases();

            if cancel_token.is_cancellation_requested() {
                return gnfs;
            }

            gnfs.new_factor_pair_collections(cancel_token);
            info!("Factor bases populated.");

            if cancel_token.is_cancellation_requested() {
                return gnfs;
            }

            gnfs.current_relations_progress = PolyRelationsSieveProgress::new(
                Arc::new(gnfs.clone()),
                relation_quantity.try_into().unwrap(),
                relation_value_range.into(),
            );
            info!("Relations container initialized. Target quantity: {}", relation_quantity);

            // TODO: Implement saving the state
            // Serialization::save_all(&gnfs);
        }

        gnfs
    }

    pub fn log_message(&mut self, message: String) {
        info!("{}", message);
    }
    
    pub fn log_message_slice(&mut self, message: &String) {
        info!("{}", message);
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

    pub fn get_prime_bounds_approximation(&mut self) {
        let base_10 = self.n.to_string().len();
        let _bound = if base_10 <= 10 {
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

    pub fn calculate_prime_factor_base_bounds(&mut self, bound: &BigInt) { // Validate this!!!_!!!
        self.prime_factor_base = FactorBase::default();

        self.prime_factor_base.rational_factor_base_max = bound.clone();
        self.prime_factor_base.algebraic_factor_base_max = &self.prime_factor_base.rational_factor_base_max * 3;

        self.prime_factor_base.quadratic_base_count = Self::calculate_quadratic_base_size(self.polynomial_degree).to_i32().unwrap();

        self.prime_factor_base.quadratic_factor_base_min = &self.prime_factor_base.algebraic_factor_base_max + 20;
        // TODO: Implement PrimeFactory::get_approximate_value_from_index
        // self.prime_factor_base.quadratic_factor_base_max = PrimeFactory::get_approximate_value_from_index(
        //     (self.prime_factor_base.quadratic_factor_base_min + self.prime_factor_base.quadratic_base_count) as u64,
        // );

        // TODO: Implement logging
        // info!(format!("Rational  Factor Base Bounds: Min: - Max: {}", self.prime_factor_base.rational_factor_base_max));
        // info!(format!("Algebraic Factor Base Bounds: Min: - Max: {}", self.prime_factor_base.algebraic_factor_base_max));
        // info!(format!("Quadratic Factor Base Bounds: Min: {} Max: {}", self.prime_factor_base.quadratic_factor_base_min, self.prime_factor_base.quadratic_factor_base_max));

        // TODO: Implement saving the state
        // Serialization::save_all(self);
        // info!("Saved prime factor base bounds.".to_string());
    }

    pub fn is_factored(&self) -> bool {
        self.factorization.is_some()
    }

    pub fn is_factor(&self, to_check: &BigInt) -> bool {
        &self.n % to_check == BigInt::zero()
    }

    pub fn set_prime_factor_bases(&mut self) {
        info!("Constructing new prime bases (- of 3)...");

        let mut prime_factory = PrimeFactory::new();
        self.prime_factor_base.rational_factor_base = PrimeFactory::get_primes_to(&mut prime_factory, &self.prime_factor_base.rational_factor_base_max)
            .collect::<Vec<BigInt>>(); // Collect the iterator into a Vec<BigInt>
        info!("Completed rational prime base (1 of 3).");

        self.prime_factor_base.algebraic_factor_base = PrimeFactory::get_primes_to(&mut prime_factory, &self.prime_factor_base.algebraic_factor_base_max)
            .collect::<Vec<BigInt>>(); // Collect the iterator into a Vec<BigInt>
        info!("Completed algebraic prime base (2 of 3).");

        self.prime_factor_base.quadratic_factor_base = PrimeFactory::get_primes_from(&mut prime_factory, &self.prime_factor_base.quadratic_factor_base_min)
            .take(self.prime_factor_base.quadratic_base_count as usize) // Convert i32 to usize
            .collect::<Vec<BigInt>>(); // Collect the iterator into a Vec<BigInt>
        info!("Completed quadratic prime base (3 of 3).");
    }
    
    fn calculate_quadratic_base_size(poly_degree: usize) -> usize {
        match poly_degree {
            d if d <= 3 => 10,
            4 => 20,
            5 | 6 => 40,
            7 => 80,
            _ => 100,
        }
    }

    pub fn construct_new_polynomial(&mut self, _polynomial_base: &BigInt, _poly_degree: usize) {
        self.current_polynomial = Polynomial::new(vec![Term::new(self.n.clone(), 0)]);

        self.polynomial_collection.push(self.current_polynomial.clone());
        // TODO: Implement saving the state
        // Serialization::save_all(self);
    }

    fn new_factor_pair_collections(&mut self, cancel_token: &CancellationToken) {
        if self.rational_factor_pair_collection.len() == 0 {
            // TODO: Implement saving the state
            // Serialization::save_factor_pair_algebraic(self);
            info!("Completed algebraic factor base (2 of 3).");
    
            if cancel_token.is_cancellation_requested() {
                return;
            }
            if self.quadratic_factor_pair_collection.len() == 0 {
                let cancel_token_arc = Arc::new(AtomicBool::new(cancel_token.is_cancellation_requested()));
                self.quadratic_factor_pair_collection = Factory::build_quadratic_factor_pair_collection(&cancel_token_arc, self);
            }
            // TODO: Implement saving the state
            // Serialization::save_factor_pair_quadratic(self);
            info!("Completed quadratic factor base (3 of 3).");
    
            if cancel_token.is_cancellation_requested() {
                return;
            }
        }
    }

    pub fn group_rough_numbers(rough_numbers: &[Relation]) -> Vec<Vec<Relation>> {
        let mut results = Vec::new();
        let mut last_index: Option<usize> = None;
    
        for (index, pair) in rough_numbers.iter().enumerate() {
            if let Some(last_idx) = last_index {
                let last = &rough_numbers[last_idx];
                if pair.algebraic_quotient == last.algebraic_quotient && pair.rational_quotient == last.rational_quotient {
                    results.push(vec![pair.clone(), last.clone()]);
                    last_index = None;  // Clear last_index as pair is grouped
                } else {
                    last_index = Some(index);  // Update last_index to current index
                }
            } else {
                last_index = Some(index);  // Initialize last_index with the first index
            }
        }
    
        results
    }    

    pub fn multiply_like_rough_numbers(gnfs: &GNFS, like_rough_numbers_groups: &[Vec<Relation>]) -> Vec<Relation> {
        let mut result = Vec::new();
    
        for like_pair in like_rough_numbers_groups {
            let as_vec: Vec<BigInt> = like_pair.iter().map(|lp| lp.a.clone()).collect();
            let bs_vec: Vec<BigInt> = like_pair.iter().map(|lp| lp.b.clone()).collect();
    
            let a = (as_vec[0].clone() + bs_vec[0].clone()) * (as_vec[0].clone() - bs_vec[0].clone());
            let b = (as_vec[1].clone() + bs_vec[1].clone()) * (as_vec[1].clone() - bs_vec[1].clone());
    
            if a > BigInt::zero() && b > BigInt::zero() {
                result.push(Relation::new(gnfs, &a, &b));
            }
        }
    
        result
    }

    pub fn set_factorization_solution(&mut self, p: &BigInt, q: &BigInt) -> bool {
        let n = p * q;
        if n == self.n {
            self.factorization = Some(Solution::new(p, q));
            let _path = PathBuf::from(&self.save_locations.save_directory).join("Solution.txt");
            // TODO: Implement writing the solution to a file
            true
        } else {
            false
        }
    }

    // fn calculate_quadratic_base_size(poly_degree: usize) -> usize {
    //     match poly_degree {
    //         d if d <= 3 => 10,
    //         4 => 20,
    //         5 | 6 => 40,
    //         7 => 80,
    //         _ => 100,
    //     }
    // }

    pub fn get_current_polynomial(&self) -> &Polynomial {
        &self.current_polynomial
    }

}

impl ToString for GNFS {
    fn to_string(&self) -> String {
        let mut result = String::new();

        result.push_str(&format!("N = {}\n\n", self.n));
        result.push_str(&format!("Polynomial(degree: {}, base: {}):\n", self.polynomial_degree, self.polynomial_base));
        result.push_str(&format!("ƒ(m) = {}\n\n", self.current_polynomial));
        result.push_str("Prime Factor Base Bounds:\n");
        result.push_str(&format!("RationalFactorBase : {}\n", self.prime_factor_base.rational_factor_base_max));
        result.push_str(&format!("AlgebraicFactorBase: {}\n", self.prime_factor_base.algebraic_factor_base_max));
        result.push_str(&format!("QuadraticPrimeBase Range: {} - {}\n", self.prime_factor_base.quadratic_factor_base_min, self.prime_factor_base.quadratic_factor_base_max));
        result.push_str(&format!("QuadraticPrimeBase Count: {}\n\n", self.prime_factor_base.quadratic_base_count));
        result.push_str(&format!("RFB - Rational Factor Base - Count: {} - Array of (p, m % p) with prime p\n", self.rational_factor_pair_collection.len()));
        result.push_str(&format!("{}\n\n", self.rational_factor_pair_collection.to_string()));
        result.push_str(&format!("AFB - Algebraic Factor Base - Count: {} - Array of (p, r) such that ƒ(r) ≡ 0 (mod p) and p is prime\n", self.algebraic_factor_pair_collection.len()));
        result.push_str(&format!("{}\n\n", self.algebraic_factor_pair_collection.to_string()));
        result.push_str(&format!("QFB - Quadratic Factor Base - Count: {} - Array of (p, r) such that ƒ(r) ≡ 0 (mod p) and p is prime\n", self.quadratic_factor_pair_collection.len()));
        result.push_str(&format!("{}\n\n", self.quadratic_factor_pair_collection.to_string()));

        result
    }
}

impl Default for GNFS {
    fn default() -> Self {
        GNFS {
            n: BigInt::from(0),
            factorization: None,
            polynomial_degree: 0,
            polynomial_base: BigInt::from(0),
            polynomial_collection: Vec::new(),
            current_polynomial: Polynomial::default(),
            current_relations_progress: PolyRelationsSieveProgress::default(),
            prime_factor_base: FactorBase::default(),
            rational_factor_pair_collection: FactorPairCollection::default(),
            algebraic_factor_pair_collection: FactorPairCollection::default(),
            quadratic_factor_pair_collection: FactorPairCollection::default(),
            save_locations: DirectoryLocations::default(),
        }
    }
}

impl AsRef<GNFS> for GNFS {
    fn as_ref(&self) -> &GNFS {
        self
    }
}