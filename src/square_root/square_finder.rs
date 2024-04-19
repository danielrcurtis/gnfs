// src/square_root/square_finder.rs

use num::{BigInt, Zero, One, Integer};
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

    log_function: Box<dyn Fn(String)>,
}

impl SquareFinder {
    pub fn new(sieve: &GNFS) -> Self {
        let log_function = Box::new(|msg: String| sieve.log_message(msg));

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
            n: sieve.n.clone(),
            s: Polynomial::zero(),
            total_s: Polynomial::zero(),
            roots_of_s: Vec::new(),
            polynomial_ring: Polynomial::zero(),
            polynomial_ring_elements: Vec::new(),
            polynomial_base: sieve.polynomial_base.clone(),
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
            log_function,
        };

        square_finder.polynomial_derivative = Polynomial::get_derivative_polynomial(&sieve.current_polynomial);
        square_finder.polynomial_derivative_squared = Polynomial::square(&square_finder.polynomial_derivative);
        square_finder.polynomial_derivative_squared_in_field =
            Polynomial::field_modulus_from_polynomial(&square_finder.polynomial_derivative_squared, &sieve.current_polynomial);

        log_function("".to_string());
        log_function(format!("ƒ'(θ) = {}", square_finder.polynomial_derivative));
        log_function(format!("ƒ'(θ)² = {}", square_finder.polynomial_derivative_squared));
        log_function(format!("ƒ'(θ)² ∈ ℤ[θ] = {}", square_finder.polynomial_derivative_squared_in_field));

        square_finder.polynomial_derivative_value = square_finder.polynomial_derivative.evaluate(&sieve.polynomial_base);
        square_finder.polynomial_derivative_value_squared = square_finder.polynomial_derivative_value.pow(2);

        log_function("".to_string());
        log_function(format!("ƒ'(m) = {}", square_finder.polynomial_derivative_value));
        log_function(format!("ƒ'(m)² = {}", square_finder.polynomial_derivative_value_squared));

        let monic_polynomial = Polynomial::make_monic(&sieve.current_polynomial, &sieve.polynomial_base);
        square_finder.monic_polynomial = monic_polynomial;
        square_finder.monic_polynomial_derivative = Polynomial::get_derivative_polynomial(&square_finder.monic_polynomial);
        square_finder.monic_polynomial_derivative_squared = Polynomial::square(&square_finder.monic_polynomial_derivative);
        square_finder.monic_polynomial_derivative_squared_in_field =
            Polynomial::field_modulus_from_polynomial(&square_finder.monic_polynomial_derivative_squared, &square_finder.monic_polynomial);

        square_finder.monic_polynomial_derivative_value = square_finder.monic_polynomial_derivative.evaluate(&sieve.polynomial_base);
        square_finder.monic_polynomial_derivative_value_squared = square_finder.monic_polynomial_derivative_squared.evaluate(&sieve.polynomial_base);

        log_function("".to_string());
        log_function(format!("MonicPolynomial: {}", square_finder.monic_polynomial));
        log_function(format!("MonicPolynomialDerivative: {}", square_finder.monic_polynomial_derivative));
        log_function(format!("MonicPolynomialDerivativeSquared: {}", square_finder.monic_polynomial_derivative_squared));
        log_function(format!("MonicPolynomialDerivativeSquaredInField: {}", square_finder.monic_polynomial_derivative_squared_in_field));

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

        (self.log_function)("".to_string());
        (self.log_function)("Rational Square Dependency:".to_string());
        (self.log_function)(rational_square_factorization_string);

        if cancel_token.is_cancellation_requested() {
            return;
        }

        self.rational_product = self.rational_norms.iter().product();

        (self.log_function)("".to_string());
        (self.log_function)(format!("δᵣ = {} = {}", self.rational_product, self.rational_norms.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(" * ")));

        let rational_product_square_root = self.rational_product.sqrt();

        let product = &self.polynomial_derivative_value * &rational_product_square_root;

        self.rational_square_root_residue = product.mod_floor(&self.n);

        (self.log_function)("".to_string());
        (self.log_function)(format!("δᵣ = {}^2 = {}", rational_product_square_root, self.rational_product));
        (self.log_function)(format!("χ  = {} ≡ {} * {} (mod {})", self.rational_square_root_residue, self.polynomial_derivative_value, rational_product_square_root, self.n));
        (self.log_function)("".to_string());

        self.is_rational_square = self.rational_product.is_square();
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
            let new_poly = Polynomial::from_terms(vec![
                Term::new(rel.b.clone(), 1),
                Term::new(rel.a.clone(), 0),
            ]);
            self.polynomial_ring_elements.push(new_poly);
        }

        if cancel_token.is_cancellation_requested() {
            return (BigInt::one(), BigInt::one());
        }

        self.polynomial_ring = Polynomial::product(&self.polynomial_ring_elements);
        let polynomial_ring_in_field = Polynomial::field_modulus(&self.polynomial_ring, &self.monic_polynomial);

        (self.log_function)("".to_string());
        (self.log_function)(format!("∏ Sᵢ = {}", self.polynomial_ring));
        (self.log_function)("".to_string());
        (self.log_function)(format!("∏ Sᵢ = {}", polynomial_ring_in_field));
        (self.log_function)(" in ℤ".to_string());
        (self.log_function)("".to_string());

        if cancel_token.is_cancellation_requested() {
            return (BigInt::one(), BigInt::one());
        }

        self.total_s = Polynomial::multiply(&self.polynomial_ring, &self.monic_polynomial_derivative_squared);
        self.s = Polynomial::field_modulus(&self.total_s, &self.monic_polynomial);

        (self.log_function)("".to_string());
        (self.log_function)(format!("δᵨ = {}", self.total_s));
        (self.log_function)(format!("δᵨ = {}", self.s));
        (self.log_function)(" in ℤ".to_string());

        let mut solution_found = false;

        let degree = self.monic_polynomial.degree();
        let f = &self.monic_polynomial;

        let mut last_p = self.gnfs.quadratic_factor_pair_collection.last().unwrap().p.clone();
        last_p = PrimeFactory::get_next_prime(&(&last_p + 1));

        let mut primes = Vec::new();
        let mut values = Vec::new();

        let mut attempts = 7;
        while !solution_found && attempts > 0 {
            if !primes.is_empty() && !values.is_empty() {
                primes.clear();
                values.clear();
            }

            loop {
                if cancel_token.is_cancellation_requested() {
                    return (BigInt::one(), BigInt::one());
                }

                last_p = PrimeFactory::get_next_prime(&(&last_p + 1));

                let g = Polynomial::parse(&format!("X^{} - X", last_p));
                let h = finite_field_arithmetic::mod_mod(&g, f, &last_p);

                let gcd = Polynomial::field_gcd(&h, f, &last_p);

                let is_irreducible = gcd.cmp(&Polynomial::one()) == Ordering::Equal;
                if !is_irreducible {
                    continue;
                }

                primes.push(last_p.clone());

                if primes.len() >= degree as usize {
                    break;
                }
            }

            if primes.len() > degree as usize {
                primes.remove(0);
                values.remove(0);
            }

            let prime_product = primes.iter().product();

            if &prime_product < &self.n {
                continue;
            }

            if cancel_token.is_cancellation_requested() {
                return (BigInt::one(), BigInt::one());
            }

            let mut take_inverse = false;
            for p in &primes {
                let chosen_poly = finite_field_arithmetic::square_root(&self.s, f, p, degree, &self.gnfs.polynomial_base);
                let eval = chosen_poly.evaluate(&self.gnfs.polynomial_base);
                let x = eval.mod_floor(p);

                values.push(x);

                (self.log_function)("".to_string());
                (self.log_function)(format!(" β = {}", chosen_poly));
                (self.log_function)(format!("xi = {}", x));
                (self.log_function)(format!(" p = {}", p));
                (self.log_function)(format!("{}", &prime_product / p));
                (self.log_function)("".to_string());

                take_inverse = !take_inverse;
            }

            let common_modulus = algorithms::chinese_remainder_theorem(&primes, &values);
            self.algebraic_square_root_residue = common_modulus.mod_floor(&self.n);

            (self.log_function)("".to_string());

            for (i, &p) in primes.iter().enumerate() {
                let tv = &values[i];
                (self.log_function)(format!("{} ≡ {} (mod {})", p, tv, self.algebraic_square_root_residue));
            }

            (self.log_function)("".to_string());
            (self.log_function)(format!("γ = {}", self.algebraic_square_root_residue));

            let min = BigInt::min(self.rational_square_root_residue, self.algebraic_square_root_residue);
            let max = BigInt::max(self.rational_square_root_residue, self.algebraic_square_root_residue);

            let a = &max + &min;
            let b = &max - &min;

            let u = GCD::find_gcd(&[self.n.clone(), a.clone()]);
            let v = GCD::find_gcd(&[self.n.clone(), b.clone()]);

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
                GNFS::log_message(format!("No solution found amongst the algebraic square roots {{ {} }} mod primes {{ {} }}",
                    values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "),
                    primes.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")));
                attempts -= 1;
            }
        }

        (BigInt::one(), BigInt::one())
    }

    pub fn solve(cancel_token: &CancellationToken, gnfs: &mut GNFS) -> bool {
        let mut tried_free_relation_indices = Vec::new();
    
        let poly_base = gnfs.polynomial_base.clone();
        let free_relations = &gnfs.current_relations_progress.free_relations;
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
    
            let selected_relation_set = &free_relations[free_relation_index];
    
            gnfs.log_message("".to_string());
            gnfs.log_message(format!("Selected solution set index # {}", free_relation_index + 1));
            gnfs.log_message("".to_string());
            gnfs.log_message("Calculating Rational Square Root β ∈ ℤ[θ] ...".to_string());
            gnfs.log_message("".to_string());
            square_root_finder.calculate_rational_side(cancel_token, selected_relation_set.clone());
    
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
    
            let non_trivial_factors_found = &p != &BigInt::one() || &q != &BigInt::one();
            if non_trivial_factors_found {
                solution_found = gnfs.set_factorization_solution(&p, &q);
    
                gnfs.log_message(format!("Selected solution set index # {}", free_relation_index + 1));
                gnfs.log_message("".to_string());
    
                if solution_found {
                    gnfs.log_message("NON-TRIVIAL FACTORS FOUND!".to_string());
                    gnfs.log_message("".to_string());
                    gnfs.log_message(square_root_finder.to_string_pretty());
                    gnfs.log_message("".to_string());
                    gnfs.log_message("".to_string());
                    gnfs.log_message(gnfs.factorization.to_string());
                    gnfs.log_message("".to_string());
                }
                break;
            } else if cancel_token.is_cancellation_requested() {
                gnfs.log_message("Abort: Task canceled by user!".to_string());
                break;
            } else {
                gnfs.log_message("".to_string());
                gnfs.log_message("Unable to locate a square root in solution set!".to_string());
                gnfs.log_message("".to_string());
                gnfs.log_message("Trying a different solution set...".to_string());
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
        result.push_str(&format!("X² / ƒ(m) = {}  IsSquare? {}\n", self.algebraic_product_mod_f, self.algebraic_product_mod_f.is_square()));
        result.push_str(&format!("S (x)       = {}  IsSquare? {}\n", self.algebraic_square_residue, self.algebraic_square_residue.is_square()));
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

        let min = BigInt::min(&self.rational_square_root_residue, &self.algebraic_square_root_residue);
        let max = BigInt::max(&self.rational_square_root_residue, &self.algebraic_square_root_residue);

        let add = &max + &min;
        let sub = &max - &min;

        let gcd_add = GCD::find_gcd(&[self.n.clone(), add.clone()]);
        let gcd_sub = GCD::find_gcd(&[self.n.clone(), sub.clone()]);

        let answer = BigInt::max(&gcd_add, &gcd_sub);

        result.push_str("\n");
        result.push_str(&format!("GCD(N, γ+χ) = {}\n", gcd_add));
        result.push_str(&format!("GCD(N, γ-χ) = {}\n", gcd_sub));
        result.push_str("\n");
        result.push_str(&format!("Solution? {}\n", (answer != BigInt::one()).to_uppercase()));

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

fn algebraic_square_root(f: &Polynomial, m: &BigInt, degree: i32, dd: &Polynomial, p: &BigInt) -> (BigInt, BigInt) {
    let start_polynomial = Polynomial::field_modulus(dd, p);
    let start_inverse_polynomial = modular_inverse(&start_polynomial, p);

    let start_squared1 = Polynomial::mod_mod(&Polynomial::square(&start_polynomial), f, p);
    let start_squared2 = Polynomial::mod_mod(&Polynomial::square(&start_inverse_polynomial), f, p);

    let result_poly1 = finite_field_arithmetic::square_root(&start_polynomial, f, p, degree, m);
    let result_poly2 = modular_inverse(&result_poly1, p);

    let result_squared1 = Polynomial::mod_mod(&Polynomial::square(&result_poly1), f, p);
    let result_squared2 = Polynomial::mod_mod(&Polynomial::square(&result_poly2), f, p);

    let both_results_agree = result_squared1.cmp(&result_squared2) == Ordering::Equal;

    let result_squared_equals_input1 = start_polynomial.cmp(&result_squared1) == Ordering::Equal;
    let result_squared_equals_input2 = start_inverse_polynomial.cmp(&result_squared1) == Ordering::Equal;

    let result1 = result_poly1.evaluate(m).mod_floor(p);
    let result2 = result_poly2.evaluate(m).mod_floor(p);

    let inverse_prime = p - &result1;
    let test_evaluations_are_modular_inverses = inverse_prime == result2;

    if both_results_agree && test_evaluations_are_modular_inverses {
        (BigInt::min(result1, result2), BigInt::max(result1, result2))
    } else {
        (BigInt::zero(), BigInt::zero())
    }
}

fn modular_inverse(poly: &Polynomial, mod_: &BigInt) -> Polynomial {
    let terms = poly.terms().iter()
        .map(|trm| Term::new((mod_ - &trm.coefficient()).mod_floor(mod_), trm.degree()))
        .collect();
    Polynomial::from_terms(terms)
}

pub fn is_square(n: &BigInt) -> bool {
    let zero = BigInt::from(0);
    let one = BigInt::from(1);
    let two = BigInt::from(2);

    if n < &zero {
        return false;
    }

    let mut x = n.clone();
    while &x % &two == zero {
        x /= &two;
    }

    if &x == &one {
        return true;
    }

    let mut i = BigInt::from(3);
    while &i * &i <= x {
        if &x % &i == zero {
            return false;
        }
        i += &two;
    }

    true
}