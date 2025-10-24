// src/core/gnfs.rs

use log::{debug, info};
use num::{BigInt, ToPrimitive, Zero, Signed};
use std::path::{Path,PathBuf};
use std::sync::{atomic::AtomicBool, Arc};
use std::iter::Iterator;
use std::marker::PhantomData;
use crate::core::factor_base::FactorBase;
use crate::core::gnfs_integer::GnfsInteger;
use crate::factor::factor_pair_collection::{FactorPairCollection, Factory};
use crate::polynomial::polynomial::Polynomial;
use crate::polynomial::polynomial::Term;
use crate::polynomial::polynomial_construction::find_optimal_base;
use crate::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress;
use crate::relation_sieve::relation::Relation;
use crate::core::solution::Solution;
use crate::core::directory_location::DirectoryLocations;
use crate::core::cancellation_token::CancellationToken;
use crate::integer_math::prime_factory::PrimeFactory;
use crate::config::BufferConfig;

#[derive(Debug, Clone)]
pub struct GNFS<T: GnfsInteger> {
    pub n: BigInt,
    _phantom: PhantomData<T>,
    pub factorization: Option<Solution>,
    pub polynomial_degree: usize,
    pub polynomial_base: BigInt,
    pub polynomial_collection: Vec<Polynomial>,
    pub current_polynomial: Polynomial,
    pub current_relations_progress: PolyRelationsSieveProgress<T>,
    pub prime_factor_base: FactorBase,
    pub rational_factor_pair_collection: FactorPairCollection,
    pub algebraic_factor_pair_collection: FactorPairCollection,
    pub quadratic_factor_pair_collection: FactorPairCollection,
    pub save_locations: DirectoryLocations,
    pub buffer_config: BufferConfig,
}

impl<T: GnfsInteger> GNFS<T> {
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
        Self::with_config(
            cancel_token,
            n,
            polynomial_base,
            poly_degree,
            prime_bound,
            relation_quantity,
            relation_value_range,
            created_new_data,
            BufferConfig::default(),
        )
    }

    pub fn with_config(
        cancel_token: &CancellationToken,
        n: &BigInt,
        polynomial_base: &BigInt,
        poly_degree: i32,
        prime_bound: &BigInt,
        relation_quantity: usize,
        relation_value_range: usize,
        created_new_data: bool,
        buffer_config: BufferConfig,
    ) -> Self {
        let mut gnfs = GNFS {
            n: n.clone(),
            _phantom: PhantomData,
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
            buffer_config: buffer_config.clone(),
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

            // Use optimized polynomial selection for better quality
            info!("Selecting optimal polynomial using Montgomery's method...");
            let (optimal_poly, optimal_base, quality_metrics) = find_optimal_base(&gnfs.n, gnfs.polynomial_degree as u32);

            gnfs.current_polynomial = optimal_poly;
            gnfs.polynomial_base = optimal_base.clone();
            gnfs.polynomial_collection.push(gnfs.current_polynomial.clone());

            info!("Polynomial constructed: {}", gnfs.current_polynomial);
            info!("Polynomial base: {}", gnfs.polynomial_base);
            info!("Polynomial quality score: {:.2}", quality_metrics.overall_score);
            info!("Root sum of squares: {:.2}", quality_metrics.root_sum_squares);
            info!("Max coefficient: {}", quality_metrics.max_coefficient);

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

            gnfs.current_relations_progress = PolyRelationsSieveProgress::with_config(
                &gnfs,
                relation_quantity.try_into().unwrap(),
                relation_value_range.into(),
                buffer_config,
            );
            info!("Relations container initialized. Target quantity: {}", relation_quantity);

            // Initialize relation streaming to disk
            let streaming_path = PathBuf::from(&gnfs.save_locations.streamed_relations_filepath);
            gnfs.current_relations_progress.relations.init_streaming(streaming_path);

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
        // TODO: Implement PrimeFactory::get_approximate_value_from_index for more accurate upper bound
        // For now, use a simple approximation: min + (count * 15) as an upper bound estimate
        // This assumes average prime gap of ~15 in this range, which works for smaller primes
        self.prime_factor_base.quadratic_factor_base_max = &self.prime_factor_base.quadratic_factor_base_min + (self.prime_factor_base.quadratic_base_count * 15);

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
        debug!("Prime factory initialized.");
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

    pub fn construct_new_polynomial(&mut self, polynomial_base: &BigInt, poly_degree: usize) {
        // Use the base-m method: express N in base m to get polynomial coefficients
        // f(x) = a₀ + a₁x + a₂x² + ... + aₐxᵈ where f(m) ≡ 0 (mod N)

        let mut coefficients: Vec<BigInt> = Vec::with_capacity(poly_degree + 1);
        let mut remainder = self.n.clone();

        // Convert N to base m to get coefficients
        for _ in 0..=poly_degree {
            let coeff = &remainder % polynomial_base;
            coefficients.push(coeff);
            remainder = &remainder / polynomial_base;
        }

        // Handle case where N requires more digits in base m than poly_degree
        // Add any remaining value to the highest degree coefficient
        if remainder > BigInt::zero() {
            if let Some(last_coeff) = coefficients.last_mut() {
                *last_coeff = last_coeff.clone() + (remainder * polynomial_base.pow(0));
            }
        }

        // Create polynomial terms from coefficients, filtering out zero coefficients
        let terms: Vec<Term> = coefficients
            .iter()
            .enumerate()
            .filter(|(_, coeff)| !coeff.is_zero())
            .map(|(degree, coeff)| Term::new(coeff.clone(), degree))
            .collect();

        self.current_polynomial = Polynomial::new(terms);

        // Verify the polynomial satisfies f(m) ≈ N
        let evaluation = self.current_polynomial.evaluate(polynomial_base);
        info!("Polynomial evaluation at base m: f({}) = {}", polynomial_base, evaluation);
        info!("Original N: {}", self.n);
        info!("Difference: {}", (&evaluation - &self.n).abs());

        self.polynomial_collection.push(self.current_polynomial.clone());
        // TODO: Implement saving the state
        // Serialization::save_all(self);
    }

    fn new_factor_pair_collections(&mut self, cancel_token: &CancellationToken) {
        // Build rational factor pair collection (1 of 3)
        if self.rational_factor_pair_collection.len() == 0 {
            self.rational_factor_pair_collection = Factory::build_rational_factor_pair_collection(self);
        }
        // TODO: Implement saving the state
        // Serialization::save_factor_pair_rational(self);
        info!("Completed rational factor base (1 of 3).");

        if cancel_token.is_cancellation_requested() {
            return;
        }

        // Build algebraic factor pair collection (2 of 3)
        if self.algebraic_factor_pair_collection.len() == 0 {
            let cancel_token_arc = Arc::new(AtomicBool::new(cancel_token.is_cancellation_requested()));
            self.algebraic_factor_pair_collection = Factory::build_algebraic_factor_pair_collection(&cancel_token_arc, self);
        }
        // TODO: Implement saving the state
        // Serialization::save_factor_pair_algebraic(self);
        info!("Completed algebraic factor base (2 of 3).");

        if cancel_token.is_cancellation_requested() {
            return;
        }

        // Build quadratic factor pair collection (3 of 3)
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

    pub fn group_rough_numbers(rough_numbers: &[Relation<T>]) -> Vec<Vec<Relation<T>>> {
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

    pub fn multiply_like_rough_numbers(gnfs: &GNFS<T>, like_rough_numbers_groups: &[Vec<Relation<T>>]) -> Vec<Relation<T>> {
        let mut result = Vec::new();

        for like_pair in like_rough_numbers_groups {
            let as_vec: Vec<BigInt> = like_pair.iter().map(|lp| lp.a.to_bigint()).collect();
            let bs_vec: Vec<BigInt> = like_pair.iter().map(|lp| lp.b.to_bigint()).collect();

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

impl<T: GnfsInteger> ToString for GNFS<T> {
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

impl<T: GnfsInteger> Default for GNFS<T> {
    fn default() -> Self {
        GNFS {
            n: BigInt::from(0),
            _phantom: PhantomData,
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
            buffer_config: BufferConfig::default(),
        }
    }
}

impl<T: GnfsInteger> AsRef<GNFS<T>> for GNFS<T> {
    fn as_ref(&self) -> &GNFS<T> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::bigint_backend::BigIntBackend;

    #[test]
    fn test_construct_polynomial_base_m_method() {
        // Test case 1: N = 45113, m = 31, degree = 3
        let cancel_token = CancellationToken::new();
        let n = BigInt::from(45113);
        let polynomial_base = BigInt::from(31);
        let poly_degree = 3;
        let prime_bound = BigInt::from(100);

        let gnfs = GNFS::<BigIntBackend>::new(
            &cancel_token,
            &n,
            &polynomial_base,
            poly_degree,
            &prime_bound,
            1,
            1000,
            true,
        );

        // Verify polynomial was constructed
        assert_eq!(gnfs.current_polynomial.degree(), 3);

        // Verify f(m) = N
        let evaluation = gnfs.current_polynomial.evaluate(&polynomial_base);
        assert_eq!(evaluation, n, "Polynomial should satisfy f(m) = N");

        // Verify expected coefficients (base-31 representation of 45113)
        // 45113 = 8 + 29*31 + 15*31^2 + 1*31^3
        assert_eq!(gnfs.current_polynomial[0], BigInt::from(8));
        assert_eq!(gnfs.current_polynomial[1], BigInt::from(29));
        assert_eq!(gnfs.current_polynomial[2], BigInt::from(15));
        assert_eq!(gnfs.current_polynomial[3], BigInt::from(1));
    }

    #[test]
    fn test_construct_polynomial_smaller_number() {
        // Test case 2: N = 1000, m = 10, degree = 3
        // 1000 in base 10 = 0 + 0*10 + 0*10^2 + 1*10^3
        let n = BigInt::from(1000);
        let polynomial_base = BigInt::from(10);
        let poly_degree = 3;

        let mut gnfs = GNFS::<BigIntBackend> {
            n: n.clone(),
            _phantom: PhantomData,
            polynomial_degree: poly_degree,
            polynomial_base: polynomial_base.clone(),
            ..Default::default()
        };

        gnfs.construct_new_polynomial(&polynomial_base, poly_degree);

        // Verify f(m) = N
        let evaluation = gnfs.current_polynomial.evaluate(&polynomial_base);
        assert_eq!(evaluation, n);

        // Verify coefficients
        assert_eq!(gnfs.current_polynomial[0], BigInt::from(0));
        assert_eq!(gnfs.current_polynomial[1], BigInt::from(0));
        assert_eq!(gnfs.current_polynomial[2], BigInt::from(0));
        assert_eq!(gnfs.current_polynomial[3], BigInt::from(1));
    }

    #[test]
    fn test_construct_polynomial_uses_parameters() {
        // Verify that the function no longer ignores parameters
        let n = BigInt::from(12345);
        let polynomial_base = BigInt::from(17);
        let poly_degree = 4;

        let mut gnfs = GNFS::<BigIntBackend> {
            n: n.clone(),
            _phantom: PhantomData,
            polynomial_degree: poly_degree,
            polynomial_base: polynomial_base.clone(),
            ..Default::default()
        };

        gnfs.construct_new_polynomial(&polynomial_base, poly_degree);

        // Should create polynomial of specified degree
        assert_eq!(gnfs.current_polynomial.degree(), poly_degree);

        // Should satisfy f(m) = N
        let evaluation = gnfs.current_polynomial.evaluate(&polynomial_base);
        assert_eq!(evaluation, n);

        // Should not be just a constant term equal to N (the old broken behavior)
        let is_constant_only = gnfs.current_polynomial.degree() == 0
            && gnfs.current_polynomial[0] == n;
        assert!(!is_constant_only, "Polynomial should not be just a constant term");
    }

    #[test]
    fn test_construct_polynomial_large_number() {
        // Test with a larger number that requires multiple digits in the base
        let n = BigInt::from(987654321_i64);
        let polynomial_base = BigInt::from(100);
        let poly_degree = 5;

        let mut gnfs = GNFS::<BigIntBackend> {
            n: n.clone(),
            _phantom: PhantomData,
            polynomial_degree: poly_degree,
            polynomial_base: polynomial_base.clone(),
            ..Default::default()
        };

        gnfs.construct_new_polynomial(&polynomial_base, poly_degree);

        // Verify f(m) = N
        let evaluation = gnfs.current_polynomial.evaluate(&polynomial_base);
        assert_eq!(evaluation, n, "Polynomial evaluation should equal N for large number");

        // Polynomial should have the correct degree
        assert!(gnfs.current_polynomial.degree() <= poly_degree,
                "Polynomial degree should not exceed specified degree");

        // All coefficients should be in range [0, m) except possibly the leading coefficient
        for i in 0..poly_degree {
            let coeff = &gnfs.current_polynomial[i];
            if coeff >= &polynomial_base {
                // This is acceptable only for the highest degree with remainder
                assert_eq!(i, gnfs.current_polynomial.degree(),
                          "Only leading coefficient can exceed base");
            }
        }
    }

    #[test]
    fn test_factor_pair_collections_initialization() {
        // Test that all three factor pair collections are properly initialized
        let cancel_token = CancellationToken::new();
        let n = BigInt::from(45113);
        let polynomial_base = BigInt::from(31);
        let poly_degree = 3;
        let prime_bound = BigInt::from(100);

        let gnfs = GNFS::<BigIntBackend>::new(
            &cancel_token,
            &n,
            &polynomial_base,
            poly_degree,
            &prime_bound,
            1,
            1000,
            true,
        );

        // Verify all three factor pair collections are non-empty
        assert!(gnfs.rational_factor_pair_collection.len() > 0,
                "Rational factor pair collection should be initialized");
        assert!(gnfs.algebraic_factor_pair_collection.len() > 0,
                "Algebraic factor pair collection should be initialized");
        assert!(gnfs.quadratic_factor_pair_collection.len() > 0,
                "Quadratic factor pair collection should be initialized");

        // Verify the rational factor base has expected structure (p, m % p)
        // The rational factor base should have one entry for each prime <= prime_bound
        let expected_rational_count = gnfs.prime_factor_base.rational_factor_base.len();
        assert_eq!(gnfs.rational_factor_pair_collection.len(), expected_rational_count,
                  "Rational factor pair count should match prime base count");

        info!("Rational factor pairs: {}", gnfs.rational_factor_pair_collection.len());
        info!("Algebraic factor pairs: {}", gnfs.algebraic_factor_pair_collection.len());
        info!("Quadratic factor pairs: {}", gnfs.quadratic_factor_pair_collection.len());
    }
}