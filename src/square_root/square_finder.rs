// src/square_root/square_finder.rs

use log::info;
use num::bigint::ToBigInt;
use num::{BigInt, Zero, One, Integer, ToPrimitive};
use crate::polynomial::polynomial::Polynomial;
use crate::core::gnfs::GNFS;
use crate::relation_sieve::relation::Relation;
use std::cmp::Ordering;
use crate::core::count_dictionary::CountDictionary;
use crate::polynomial::polynomial::Term;
use crate::integer_math::gcd::GCD;
use crate::integer_math::prime_factory::PrimeFactory;
use crate::core::static_random::StaticRandom;
use crate::square_root::finite_field_arithmetic;
use crate::core::cancellation_token::CancellationToken;
use crate::polynomial::algorithms;
use rayon::prelude::*;
use std::time::Instant;

pub struct SquareFinder {
    pub rational_product: BigInt,
    pub rational_square: BigInt,
    pub rational_square_root_residue: BigInt,
    pub is_rational_square: bool,
    pub is_rational_irreducible: bool,

    pub algebraic_product: BigInt,
    pub algebraic_square: BigInt,
    pub algebraic_product_mod_f: BigInt,
    pub algebraic_square_residue: BigInt,
    pub algebraic_square_root_residue: BigInt,
    pub algebraic_primes: Vec<BigInt>,
    pub algebraic_results: Vec<BigInt>,
    pub is_algebraic_square: bool,
    pub is_algebraic_irreducible: bool,

    pub n: BigInt,
    pub s: Polynomial,
    pub total_s: Polynomial,
    pub roots_of_s: Vec<(BigInt, BigInt)>,
    pub polynomial_ring: Polynomial,
    pub polynomial_ring_elements: Vec<Polynomial>,

    pub polynomial_base: BigInt,
    pub monic_polynomial: Polynomial,
    pub polynomial_derivative: Polynomial,
    pub monic_polynomial_derivative: Polynomial,

    pub polynomial_derivative_squared: Polynomial,
    pub polynomial_derivative_squared_in_field: Polynomial,

    pub polynomial_derivative_value: BigInt,
    pub polynomial_derivative_value_squared: BigInt,

    pub monic_polynomial_derivative_squared: Polynomial,
    pub monic_polynomial_derivative_squared_in_field: Polynomial,

    pub monic_polynomial_derivative_value: BigInt,
    pub monic_polynomial_derivative_value_squared: BigInt,

    gnfs: GNFS,
    rational_norms: Vec<BigInt>,
    algebraic_norm_collection: Vec<BigInt>,
    relations_set: Vec<Relation>,

}

/// Generate a batch of consecutive prime numbers starting from a given value
fn generate_prime_batch(start_from: i128, batch_size: usize) -> Vec<BigInt> {
    let mut primes = Vec::with_capacity(batch_size);
    let mut current = start_from;

    for _ in 0..batch_size {
        current = PrimeFactory::get_next_prime_from_i128(current).to_i128().unwrap();
        primes.push(current.to_bigint().unwrap());
        current += 1; // Move to next candidate for next iteration
    }

    primes
}

impl SquareFinder {
    pub fn new(sieve: &GNFS) -> Self {
        let sieve_ref = sieve;

        let mut square_finder = SquareFinder {
            rational_product: BigInt::zero(),
            rational_square: BigInt::zero(),
            rational_square_root_residue: BigInt::from(-1),
            is_rational_square: false,
            is_rational_irreducible: false,
            algebraic_product: BigInt::zero(),
            algebraic_square: BigInt::zero(),
            algebraic_product_mod_f: BigInt::zero(),
            algebraic_square_residue: BigInt::zero(),
            algebraic_square_root_residue: BigInt::zero(),
            algebraic_primes: Vec::new(),
            algebraic_results: Vec::new(),
            is_algebraic_square: false,
            is_algebraic_irreducible: false,
            n: sieve_ref.n.clone(),
            s: Polynomial::zero(),
            total_s: Polynomial::zero(),
            roots_of_s: Vec::new(),
            polynomial_ring: Polynomial::zero(),
            polynomial_ring_elements: Vec::new(),
            polynomial_base: sieve_ref.polynomial_base.clone(),
            monic_polynomial: Polynomial::zero(),
            polynomial_derivative: Polynomial::zero(),
            monic_polynomial_derivative: Polynomial::zero(),
            polynomial_derivative_squared: Polynomial::zero(),
            polynomial_derivative_squared_in_field: Polynomial::zero(),
            polynomial_derivative_value: BigInt::zero(),
            polynomial_derivative_value_squared: BigInt::zero(),
            monic_polynomial_derivative_squared: Polynomial::zero(),
            monic_polynomial_derivative_squared_in_field: Polynomial::zero(),
            monic_polynomial_derivative_value: BigInt::zero(),
            monic_polynomial_derivative_value_squared: BigInt::zero(),
            gnfs: sieve.clone(),
            rational_norms: Vec::new(),
            algebraic_norm_collection: Vec::new(),
            relations_set: Vec::new(),
        };

        square_finder.polynomial_derivative = Polynomial::get_derivative_polynomial(&sieve.current_polynomial);
        square_finder.polynomial_derivative_squared = Polynomial::square(&square_finder.polynomial_derivative);
        square_finder.polynomial_derivative_squared_in_field =
            Polynomial::field_modulus_from_polynomial(&square_finder.polynomial_derivative_squared, &sieve.current_polynomial);

        info!("{}", "".to_string());
        info!("{}", format!("ƒ'(θ) = {}", square_finder.polynomial_derivative));
        info!("{}", format!("ƒ'(θ)² = {}", square_finder.polynomial_derivative_squared));
        info!("{}", format!("ƒ'(θ)² ∈ ℤ[θ] = {}", square_finder.polynomial_derivative_squared_in_field));

        square_finder.polynomial_derivative_value = square_finder.polynomial_derivative.evaluate(&sieve.polynomial_base);
        square_finder.polynomial_derivative_value_squared = square_finder.polynomial_derivative_value.pow(2);

        info!("{}", "".to_string());
        info!("{}", format!("ƒ'(m) = {}", square_finder.polynomial_derivative_value));
        info!("{}", format!("ƒ'(m)² = {}", square_finder.polynomial_derivative_value_squared));

        let monic_polynomial = Polynomial::make_monic(&sieve.current_polynomial, &sieve.polynomial_base);
        square_finder.monic_polynomial = monic_polynomial;
        square_finder.monic_polynomial_derivative = Polynomial::get_derivative_polynomial(&square_finder.monic_polynomial);
        square_finder.monic_polynomial_derivative_squared = Polynomial::square(&square_finder.monic_polynomial_derivative);
        square_finder.monic_polynomial_derivative_squared_in_field =
            Polynomial::field_modulus_from_polynomial(&square_finder.monic_polynomial_derivative_squared, &square_finder.monic_polynomial);

        square_finder.monic_polynomial_derivative_value = square_finder.monic_polynomial_derivative.evaluate(&sieve.polynomial_base);
        square_finder.monic_polynomial_derivative_value_squared = square_finder.monic_polynomial_derivative_squared.evaluate(&sieve.polynomial_base);

        info!("{}", "".to_string());
        info!("{}", format!("MonicPolynomial: {}", square_finder.monic_polynomial));
        info!("{}", format!("MonicPolynomialDerivative: {}", square_finder.monic_polynomial_derivative));
        info!("{}", format!("MonicPolynomialDerivativeSquared: {}", square_finder.monic_polynomial_derivative_squared));
        info!("{}", format!("MonicPolynomialDerivativeSquaredInField: {}", square_finder.monic_polynomial_derivative_squared_in_field));

        square_finder
    }

    pub fn calculate_rational_side(&mut self, cancel_token: &CancellationToken, relations: Vec<Relation>) {
        self.relations_set = relations;
        self.rational_norms = self.relations_set.iter().map(|rel| rel.rational_norm.clone()).collect();

        let mut rational_square_factorization = CountDictionary::new();
        for rel in &self.relations_set {
            rational_square_factorization.combine(&rel.rational_factorization);
        }

        let rational_square_factorization_string = rational_square_factorization.format_string_as_factorization();

        info!("{}", "".to_string());
        info!("{}", "Rational Square Dependency:".to_string());
        info!("{}", rational_square_factorization_string);

        if cancel_token.is_cancellation_requested() {
            return;
        }

        self.rational_product = self.rational_norms.iter().product();

        info!("{}", "".to_string());
        info!("{}", format!("δᵣ = {} = {}", self.rational_product, self.rational_norms.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(" * ")));

        let rational_product_square_root = self.rational_product.sqrt();

        let product = &self.polynomial_derivative_value * &rational_product_square_root;

        self.rational_square_root_residue = product.mod_floor(&self.n);

        info!("{}", "".to_string());
        info!("{}", format!("δᵣ = {}^2 = {}", rational_product_square_root, self.rational_product));
        info!("{}", format!("χ  = {} ≡ {} * {} (mod {})", self.rational_square_root_residue, self.polynomial_derivative_value, rational_product_square_root, self.n));
        info!("{}", "".to_string());

        self.is_rational_square = is_square(&self.rational_product);
        if !self.is_rational_square {
            panic!("is_rational_square evaluated to false. This is a sign that there is a bug in the implementation, as this should never be the case if the algorithm has been correctly implemented.");
        }
    }

    pub fn calculate_algebraic_side(&mut self, cancel_token: &CancellationToken) -> (BigInt, BigInt) {
        for rel in &self.relations_set {
            self.roots_of_s.push((rel.a.clone(), rel.b.clone()));
        }

        if cancel_token.is_cancellation_requested() {
            return (BigInt::one(), BigInt::one());
        }

        self.polynomial_ring_elements.clear();
        for rel in &self.relations_set {
            let new_poly = Polynomial::new(vec![
                Term::new(rel.b.clone(), 1),
                Term::new(rel.a.clone(), 0),
            ]);
            self.polynomial_ring_elements.push(new_poly);
        }

        if cancel_token.is_cancellation_requested() {
            return (BigInt::one(), BigInt::one());
        }

        self.polynomial_ring = Polynomial::product(&self.polynomial_ring_elements);
        let polynomial_ring_in_field = Polynomial::field_modulus_from_polynomial(&self.polynomial_ring, &self.monic_polynomial);

        info!("{}", "".to_string());
        info!("{}", format!("∏ Sᵢ = {}", self.polynomial_ring));
        info!("{}", "".to_string());
        info!("{}", format!("∏ Sᵢ = {}", polynomial_ring_in_field));
        info!("{}", " in ℤ".to_string());
        info!("{}", "".to_string());

        if cancel_token.is_cancellation_requested() {
            return (BigInt::one(), BigInt::one());
        }

        self.total_s = Polynomial::multiply(&self.polynomial_ring, &self.monic_polynomial_derivative_squared);
        self.s = Polynomial::field_modulus_from_polynomial(&self.total_s, &self.monic_polynomial);

        info!("{}", "".to_string());
        info!("{}", format!("δᵨ = {}", self.total_s));
        info!("{}", format!("δᵨ = {}", self.s));
        info!("{}", " in ℤ".to_string());

        let degree = self.monic_polynomial.degree();
        let f = &self.monic_polynomial;

        // Get batch size from environment variable, default to 10
        let batch_size: usize = std::env::var("GNFS_STAGE4_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        // Initialize last_p ONCE from quadratic factor base (outside the loop)
        let mut last_p_i128 = self.gnfs.quadratic_factor_pair_collection.clone().last().unwrap().p;
        let sqrt_n_times_2: BigInt = self.n.sqrt() * 2;
        if let Some(sqrt_n_i128) = sqrt_n_times_2.to_i128() {
            last_p_i128 = i128::max(last_p_i128, sqrt_n_i128);
        }

        info!("{}", format!("Starting search for irreducible primes from p = {}", last_p_i128));
        info!("{}", format!("Need {} irreducible primes with product > N", degree));
        info!("{}", format!("Using batch size: {} primes per batch", batch_size));
        info!("{}", format!("Rayon threads: {}", rayon::current_num_threads()));
        info!("{}", "".to_string());

        let mut primes = Vec::new();
        let mut values = Vec::new();
        let mut total_primes_tested = 0;
        let mut batch_number = 0;

        loop {
            if cancel_token.is_cancellation_requested() {
                return (BigInt::one(), BigInt::one());
            }

            batch_number += 1;

            // Generate a batch of candidate primes
            let prime_batch = generate_prime_batch(last_p_i128, batch_size);

            // Update last_p_i128 for next batch
            if let Some(last_prime) = prime_batch.last() {
                last_p_i128 = last_prime.to_i128().unwrap();
            }

            info!("{}", format!("Batch #{}: Testing {} primes in parallel (starting from p = {})",
                batch_number, prime_batch.len(), prime_batch.first().unwrap()));

            let batch_start = Instant::now();

            // Parallel test for irreducibility using Rayon
            let irreducible_results: Vec<(BigInt, BigInt)> = prime_batch.par_iter()
                .filter_map(|test_p| {
                    let start_total = Instant::now();

                    // Time polynomial parsing
                    let start_parse = Instant::now();
                    let g = Polynomial::parse(&format!("X^{} - X", test_p));
                    let parse_time = start_parse.elapsed();

                    // Time mod_mod operation
                    let start_mod = Instant::now();
                    let h = finite_field_arithmetic::mod_mod(&g, f, test_p);
                    let mod_time = start_mod.elapsed();

                    // Time GCD computation
                    let start_gcd = Instant::now();
                    let gcd = Polynomial::field_gcd(&h, f, test_p);
                    let gcd_time = start_gcd.elapsed();

                    let total_time = start_total.elapsed();
                    let is_irreducible = gcd.cmp(&Polynomial::one()) == Ordering::Equal;

                    // Log timing for first few primes in each batch
                    if test_p < &BigInt::from(450) || (batch_number <= 3) {
                        info!("Prime {}: parse={}µs, mod={}µs, gcd={}µs, total={}µs, irreducible={}",
                            test_p, parse_time.as_micros(), mod_time.as_micros(),
                            gcd_time.as_micros(), total_time.as_micros(), is_irreducible);
                    }

                    if is_irreducible {
                        // Time square root computation
                        let start_sqrt = Instant::now();
                        let chosen_poly = finite_field_arithmetic::square_root(&self.s, f, test_p, degree.try_into().unwrap(), &self.gnfs.polynomial_base);
                        let eval = chosen_poly.evaluate(&self.gnfs.polynomial_base);
                        let x = eval.mod_floor(test_p);
                        let sqrt_time = start_sqrt.elapsed();

                        info!("Found irreducible p={}, square_root took {}ms", test_p, sqrt_time.as_millis());

                        Some((test_p.clone(), x))
                    } else {
                        None
                    }
                })
                .collect();

            let batch_elapsed = batch_start.elapsed();
            total_primes_tested += prime_batch.len();

            info!("{}", format!("Batch #{} completed in {:.2}s ({} primes tested, {} irreducible found, {:.0} primes/sec)",
                batch_number, batch_elapsed.as_secs_f64(), prime_batch.len(), irreducible_results.len(),
                prime_batch.len() as f64 / batch_elapsed.as_secs_f64()));

            // Add found primes to our collection
            for (p, x) in irreducible_results {
                // Remove oldest entry if at capacity
                if primes.len() == degree as usize {
                    primes.remove(0);
                    values.remove(0);
                }

                primes.push(p.clone());
                values.push(x.clone());

                info!("{}", "".to_string());
                info!("{}", format!("Found irreducible prime! p = {}", p));
                info!("{}", format!("xi = {}", x));
                info!("{}", "".to_string());
            }

            // Check if we have enough primes
            if primes.len() == degree as usize {
                let prime_product: BigInt = primes.iter().product();

                if &prime_product < &self.n {
                    info!("{}", "".to_string());
                    info!("{}", format!("Prime product {} < N ({}). Continuing search...", prime_product, self.n));
                    info!("{}", "".to_string());
                    primes.clear();
                    values.clear();
                    continue;
                }

                if cancel_token.is_cancellation_requested() {
                    return (BigInt::one(), BigInt::one());
                }

                // Compute CRT result
                let common_modulus = algorithms::chinese_remainder_theorem(&primes, &values);
                self.algebraic_square_root_residue = common_modulus.mod_floor(&self.n);

                info!("{}", "".to_string());

                for (i, p) in primes.iter().enumerate() {
                    let tv = &values[i];
                    info!("{}", format!("{} ≡ {} (mod {})", p, tv, self.algebraic_square_root_residue));
                }

                info!("{}", "".to_string());
                info!("{}", format!("γ = {}", self.algebraic_square_root_residue));

                let min = BigInt::min(self.rational_square_root_residue.clone(), self.algebraic_square_root_residue.clone());
                let max = BigInt::max(self.rational_square_root_residue.clone(), self.algebraic_square_root_residue.clone());

                let a = &max + &min;
                let b = &max - &min;

                let u = GCD::find_gcd(&[self.n.clone(), a.clone()]);
                let v = GCD::find_gcd(&[self.n.clone(), b.clone()]);

                let mut solution_found = false;
                let mut p = BigInt::zero();
                if &u > &BigInt::one() && &u != &self.n {
                    p = u;
                    solution_found = true;
                } else if &v > &BigInt::one() && &v != &self.n {
                    p = v;
                    solution_found = true;
                }

                if solution_found {
                    let (q, rem) = self.n.div_rem(&p);
                    if rem.is_zero() {
                        self.algebraic_results = values;
                        self.algebraic_primes = primes;
                        return (p, q);
                    } else {
                        solution_found = false;
                    }
                }

                if !solution_found {
                    info!("{}", format!("No solution found amongst the algebraic square roots {{ {} }} mod primes {{ {} }}",
                        values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "),
                        primes.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")));
                    info!("{}", "".to_string());
                    info!("{}", "Clearing primes and values to retry with new primes...".to_string());
                    primes.clear();
                    values.clear();
                    continue;
                }
            }
        }
    }

    pub fn solve(cancel_token: &CancellationToken, gnfs: &mut GNFS) -> bool {
        let mut tried_free_relation_indices = Vec::new();
    
        let poly_base = gnfs.polynomial_base.clone();
        let free_relations = gnfs.current_relations_progress.relations.free_relations.clone();
        let mut square_root_finder = SquareFinder::new(gnfs);
    
        let mut free_relation_index = 0;
        let mut solution_found = false;
    
        while !solution_found {
            if cancel_token.is_cancellation_requested() {
                return solution_found;
            }
    
            if tried_free_relation_indices.len() == free_relations.len() {
                gnfs.log_message("ERROR: ALL RELATION SETS HAVE BEEN TRIED...?".to_string());
                gnfs.log_message(format!("If the number of solution sets ({}) is low, you may need to sieve some more and then re-run the matrix solving step.", free_relations.len()));
                gnfs.log_message("If there are many solution sets, and you have tried them all without finding non-trivial factors, then something is wrong...".to_string());
                gnfs.log_message("".to_string());
                break;
            }
    
            let mut static_random = StaticRandom::new();
            loop {
                free_relation_index = static_random.next_range(0, free_relations.len() as u32);
                if !tried_free_relation_indices.contains(&free_relation_index) {
                    break;
                }
            }
    
            tried_free_relation_indices.push(free_relation_index);
    
            let selected_relation_set: &_ = &free_relations[free_relation_index as usize];
    
            gnfs.log_message("".to_string());
            gnfs.log_message(format!("Selected solution set index # {}", free_relation_index + 1));
            gnfs.log_message("".to_string());
            gnfs.log_message("Calculating Rational Square Root β ∈ ℤ[θ] ...".to_string());
            gnfs.log_message("".to_string());
            square_root_finder.calculate_rational_side(cancel_token, selected_relation_set.clone() as Vec<Relation>);
    
            if cancel_token.is_cancellation_requested() {
                gnfs.log_message("Abort: Task canceled by user!".to_string());
                break;
            }
    
            gnfs.log_message("SquareFinder.CalculateRationalSide() Completed.".to_string());
            gnfs.log_message("".to_string());
            gnfs.log_message("Calculating Algebraic Square Root...".to_string());
            gnfs.log_message("                    y ∈ ℤ ...".to_string());
            gnfs.log_message("δ in a finite field 𝔽ᵨ(θᵨ) ...".to_string());
            gnfs.log_message("".to_string());
    
            let found_factors = square_root_finder.calculate_algebraic_side(cancel_token);
    
            if cancel_token.is_cancellation_requested() {
                gnfs.log_message("Abort: Task canceled by user!".to_string());
                break;
            }
    
            gnfs.log_message("SquareFinder.CalculateAlgebraicSide() Completed.".to_string());
            gnfs.log_message("".to_string());
            gnfs.log_message(format!("{}² ≡ {}² (mod {})", square_root_finder.algebraic_square_root_residue, square_root_finder.rational_square_root_residue, square_root_finder.n));
            gnfs.log_message("".to_string());
    
            let p = found_factors.0;
            let q = found_factors.1;

            gnfs.log_message("".to_string());
            gnfs.log_message(format!("Factors returned: p = {}, q = {}", p, q));
            gnfs.log_message("".to_string());

            let non_trivial_factors_found = &p != &BigInt::one() && &q != &BigInt::one();
            if non_trivial_factors_found {
                solution_found = gnfs.set_factorization_solution(&p, &q);

                gnfs.log_message(format!("Selected solution set index # {}", free_relation_index + 1));
                gnfs.log_message("".to_string());

                if solution_found {
                    gnfs.log_message("NON-TRIVIAL FACTORS FOUND!".to_string());
                    gnfs.log_message("".to_string());
                    gnfs.log_message(square_root_finder.to_string());
                    gnfs.log_message("".to_string());
                    gnfs.log_message("".to_string());
                    match &gnfs.factorization {
                        Some(solution) => gnfs.log_message(solution.to_string()),
                        None => gnfs.log_message("No solution found.".to_string()),
                    }
                    gnfs.log_message("".to_string());
                }
                break;
            } else if cancel_token.is_cancellation_requested() {
                gnfs.log_message("Abort: Task canceled by user!".to_string());
                break;
            } else {
                gnfs.log_message("TRIVIAL FACTORS DETECTED: Both p and q are 1.".to_string());
                gnfs.log_message("This means the algebraic square root computation failed for this relation set.".to_string());
                gnfs.log_message("Trying a different solution set...".to_string());
                gnfs.log_message("".to_string());
                gnfs.log_message("Unable to locate a square root in solution set!".to_string());
                gnfs.log_message("".to_string());
            }
        }
    
        solution_found
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();

        result.push_str("Polynomial ring:\n");
        result.push_str(&format!("({})\n", self.polynomial_ring_elements.iter()
            .map(|ply| ply.to_string())
            .collect::<Vec<_>>()
            .join(") * (")));
        result.push_str("\n");
        result.push_str("∏ Sᵢ =\n");
        result.push_str(&format!("{}\n", self.polynomial_ring));
        result.push_str("\n");
        result.push_str(&format!("ƒ         = {}\n", self.gnfs.current_polynomial));
        result.push_str(&format!("ƒ(m)      = {}\n", self.monic_polynomial));
        result.push_str(&format!("ƒ'(m)     = {}\n", self.monic_polynomial_derivative));
        result.push_str(&format!("ƒ'(m)^2   = {}\n", self.monic_polynomial_derivative_squared));
        result.push_str("\n");
        result.push_str("∏ Sᵢ(m)  *  ƒ'(m)² =\n");
        result.push_str(&format!("{}\n", self.total_s));
        result.push_str("\n");
        result.push_str("∏ Sᵢ(m)  *  ƒ'(m)² (mod ƒ) =\n");
        result.push_str(&format!("{}\n", self.s));
        result.push_str("\n");
        result.push_str("\n");
        result.push_str("Square finder, Rational:\n");
        result.push_str("γ² = √(  Sᵣ(m)  *  ƒ'(m)²  )\n");
        result.push_str(&format!("γ² = √( {} * {} )\n", self.rational_product, self.polynomial_derivative_value_squared));
        result.push_str(&format!("γ² = √( {} )\n", self.rational_square));
        result.push_str(&format!("IsRationalSquare  ? {}\n", self.is_rational_square));
        result.push_str(&format!("γ  =    {} mod N\n", self.rational_square_root_residue));
        result.push_str(&format!("IsRationalIrreducible  ? {}\n", self.is_rational_irreducible));
        result.push_str("\n");
        result.push_str("\n");
        result.push_str("Square finder, Algebraic:\n");
        result.push_str(&format!("    Sₐ(m) * ƒ'(m)  =  {} * {}\n", self.algebraic_product, self.polynomial_derivative_value));
        result.push_str(&format!("    Sₐ(m) * ƒ'(m)  =  {}\n", self.algebraic_square));
        result.push_str(&format!("IsAlgebraicSquare ? {}\n", self.is_algebraic_square));
        result.push_str(&format!("χ = Sₐ(m) * ƒ'(m) mod N = {}\n", self.algebraic_square_root_residue));
        result.push_str(&format!("IsAlgebraicIrreducible ? {}\n", self.is_algebraic_irreducible));
        result.push_str("\n");
        result.push_str(&format!("X² / ƒ(m) = {}  IsSquare? {}\n", self.algebraic_product_mod_f, is_square(&self.algebraic_product_mod_f)));
        result.push_str(&format!("S (x)       = {}  IsSquare? {}\n", self.algebraic_square_residue, is_square(&self.algebraic_square_residue)));
        result.push_str("AlgebraicResults:\n");
        result.push_str(&format!("{}\n", self.algebraic_results.iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join(", ")));
        result.push_str("\n");
        result.push_str("\n");

        result.push_str("Primes:\n");
        result.push_str(&format!("{}\n", self.algebraic_primes.iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(" * ")));
        result.push_str("\n");
        result.push_str("\n");

        let min = BigInt::min(self.rational_square_root_residue.clone(), self.algebraic_square_root_residue.clone());
        let max = BigInt::max(self.rational_square_root_residue.clone(), self.algebraic_square_root_residue.clone());

        let add = &max + &min;
        let sub = &max - &min;

        let gcd_add = GCD::find_gcd(&[self.n.clone(), add.clone()]);
        let gcd_sub = GCD::find_gcd(&[self.n.clone(), sub.clone()]);

        let answer = BigInt::max(gcd_add.clone(), gcd_sub.clone());

        result.push_str("\n");
        result.push_str(&format!("GCD(N, γ+χ) = {}\n", gcd_add));
        result.push_str(&format!("GCD(N, γ-χ) = {}\n", gcd_sub));
        result.push_str("\n");
        result.push_str(&format!("Solution? {}\n", (answer != BigInt::one()).to_string().to_ascii_uppercase()));

        if answer != BigInt::one() {
            result.push_str("\n");
            result.push_str("\n");
            result.push_str("*********************\n");
            result.push_str("\n");
            result.push_str(&format!(" SOLUTION = {} \n", answer));
            result.push_str("\n");
            result.push_str("*********************\n");
            result.push_str("\n");
            result.push_str("\n");
        }

        result.push_str("\n");

        result
    }

}

pub fn algebraic_square_root(f: &Polynomial, m: &BigInt, degree: i32, dd: &Polynomial, p: &BigInt) -> (BigInt, BigInt) {
    let start_polynomial = Polynomial::field_modulus(dd, p);
    //let start_inverse_polynomial = modular_inverse(&start_polynomial, p);

    let result_poly1 = finite_field_arithmetic::square_root(&start_polynomial, f, p, degree, m);
    let result_poly2 = modular_inverse(&result_poly1, p);

    let result_squared1 = Polynomial::mod_mod(&Polynomial::square(&result_poly1), f, p);
    let result_squared2 = Polynomial::mod_mod(&Polynomial::square(&result_poly2), f, p);

    let both_results_agree = result_squared1 == result_squared2;

    let result1 = result_poly1.evaluate(m).mod_floor(p);
    let result2 = result_poly2.evaluate(m).mod_floor(p);

    let inverse_prime = p - &result1;
    let test_evaluations_are_modular_inverses = inverse_prime == result2;

    if both_results_agree && test_evaluations_are_modular_inverses {
        (result1, result2)
    } else {
        (BigInt::zero(), BigInt::zero())
    }
}

pub fn modular_inverse(poly: &Polynomial, mod_: &BigInt) -> Polynomial {
    let terms = poly.terms.iter()
        .map(|(&exp, coef)| (exp, (mod_ - coef).mod_floor(mod_)))
        .collect();
    Polynomial { terms }
}

pub fn is_square(n: &BigInt) -> bool {
    use num::Signed;

    let zero = BigInt::from(0);
    let four = BigInt::from(4);
    let fifteen = BigInt::from(15);

    // Handle zero and negative numbers
    if n == &zero {
        return false;
    }

    let input = n.abs();

    // Numbers less than 4 (except 0, 1) can be handled quickly
    if &input < &four {
        return input == BigInt::from(1);
    }

    // Quick base-16 check: squares in base 16 end in 0, 1, 4, or 9
    let base16 = (&input & &fifteen).to_i32().unwrap_or(0);
    if base16 > 9 {
        return false; // Quickly reject 6 cases out of 16
    }

    // Squares in base 16 end in 0, 1, 4, or 9
    // So reject if base16 is 2, 3, 5, 6, 7, or 8
    if base16 == 2 || base16 == 3 || base16 == 5 || base16 == 6 || base16 == 7 || base16 == 8 {
        return false;
    }

    // Actually compute the square root and check if it's exact
    let sqrt = input.sqrt();
    &sqrt * &sqrt == input
}

#[cfg(test)]
mod tests {
    use super::*;
    use num::BigInt;
    use crate::polynomial::polynomial::{Polynomial, Term};

    #[test]
    fn test_irreducibility_optimization() {
        // Test that the optimized X^p mod f computation produces valid results
        // We can't compare to the old method for p > ~20 because it's too slow

        // Test polynomial: f(x) = x^3 - 2
        let f = Polynomial::new(vec![
            Term::new(BigInt::from(1), 3),
            Term::new(BigInt::from(-2), 0),
        ]);

        // Test with various primes
        let test_primes = vec![
            BigInt::from(5),
            BigInt::from(7),
            BigInt::from(11),
            BigInt::from(431), // Large prime that would be impossible with old method
        ];

        for p in test_primes {
            // Optimized method: Use exponentiate_mod
            let x_poly = Polynomial::from_term(BigInt::one(), 1); // X
            let x_pow_p = Polynomial::exponentiate_mod(&x_poly, &p, &f, &p); // X^p mod f
            let h = x_pow_p - x_poly.clone(); // X^p - X (mod f)
            let h = h.field_modulus(&p); // Apply prime modulus to coefficients

            // Verify the result is a valid polynomial with degree < f.degree()
            assert!(h.degree() < f.degree(),
                "Result polynomial degree {} should be less than f.degree() {}",
                h.degree(), f.degree());

            // Compute GCD to test irreducibility
            let gcd = Polynomial::field_gcd(&h, &f, &p);

            // For this test, we just verify the GCD computation completes successfully
            // and returns a valid polynomial
            assert!(gcd.degree() <= f.degree(),
                "GCD degree {} should be <= f.degree() {}",
                gcd.degree(), f.degree());
        }
    }

    #[test]
    fn test_performance_large_prime() {
        use std::time::Instant;

        // Test with a large prime to see if there's a performance issue
        let f = Polynomial::new(vec![
            Term::new(BigInt::from(1), 3),
            Term::new(BigInt::from(-2), 0),
        ]);

        let p = BigInt::from(431);

        println!("Testing irreducibility with p=431...");

        let start = Instant::now();
        let g = Polynomial::parse(&format!("X^{} - X", p));
        let parse_time = start.elapsed();
        println!("Parse time: {:?}", parse_time);

        let start = Instant::now();
        let h = finite_field_arithmetic::mod_mod(&g, &f, &p);
        let modmod_time = start.elapsed();
        println!("ModMod time: {:?}", modmod_time);

        let start = Instant::now();
        let gcd = Polynomial::field_gcd(&h, &f, &p);
        let gcd_time = start.elapsed();
        println!("GCD time: {:?}", gcd_time);

        println!("Total time: {:?}", parse_time + modmod_time + gcd_time);
        println!("GCD result: {:?}", gcd);

        // Just verify it completes without error
        assert!(gcd.degree() <= f.degree());
    }
}