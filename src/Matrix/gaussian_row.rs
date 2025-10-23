// src/matrix/gaussian_row.rs

use num::{BigInt, Signed};
use crate::core::gnfs::GNFS;
use crate::core::gnfs_integer::GnfsInteger;
use crate::relation_sieve::relation::Relation;
use crate::core::count_dictionary::CountDictionary;
use crate::integer_math::prime_factory::PrimeFactory;
use crate::integer_math::quadratic_residue::QuadraticResidue;

#[derive(Clone)]
pub struct GaussianRow<T: GnfsInteger> {
    pub sign: bool,
    pub rational_part: Vec<bool>,
    pub algebraic_part: Vec<bool>,
    pub quadratic_part: Vec<bool>,
    pub source_relation: Relation<T>,
}

impl<T: GnfsInteger> GaussianRow<T> {
    pub fn new(gnfs: &GNFS<T>, relation: Relation<T>) -> Self {
        // Convert to BigInt for sign check (cheap conversion)
        let sign = relation.rational_norm.to_bigint().is_negative();

        let qfb = gnfs.quadratic_factor_pair_collection.clone();
        let rational_max_value = &gnfs.prime_factor_base.rational_factor_base_max;
        let algebraic_max_value = &gnfs.prime_factor_base.algebraic_factor_base_max;

        let rational_part = Self::get_vector(&relation.rational_factorization, rational_max_value);
        let algebraic_part = Self::get_vector(&relation.algebraic_factorization, algebraic_max_value);
        let quadratic_part = qfb.into_iter()
            .map(|qf| QuadraticResidue::get_quadratic_character(&relation, &qf))
            .collect();

        GaussianRow {
            sign,
            rational_part,
            algebraic_part,
            quadratic_part,
            source_relation: relation,
        }
    }

    fn get_vector(prime_factorization_dict: &CountDictionary, max_value: &BigInt) -> Vec<bool> {
        let mut prime_factory = PrimeFactory::new();
        let prime_index = prime_factory.get_index_from_value(max_value);
        let mut result = vec![false; prime_index as usize];
        if prime_factorization_dict.len() == 0 {
            return result;
        }
        for (key, value) in prime_factorization_dict.to_dict() {
            if key > *max_value || key == BigInt::from(-1) || value % 2 == BigInt::from(0) {
                continue;
            }
            let index = prime_factory.get_index_from_value(&key);
            result[index as usize] = true;
        }
        result
    }

    pub fn last_index_of_rational(&self) -> Option<usize> {
        self.rational_part.iter().rposition(|&x| x)
    }

    pub fn last_index_of_algebraic(&self) -> Option<usize> {
        self.algebraic_part.iter().rposition(|&x| x)
    }

    pub fn last_index_of_quadratic(&self) -> Option<usize> {
        self.quadratic_part.iter().rposition(|&x| x)
    }

    pub fn get_bool_array(&self) -> Vec<bool> {
        let mut result = vec![self.sign];
        result.extend_from_slice(&self.rational_part);
        result.extend_from_slice(&self.algebraic_part);
        result.extend_from_slice(&self.quadratic_part);
        result
    }

    pub fn resize_rational_part(&mut self, size: usize) {
        self.rational_part.truncate(size + 1);
    }

    pub fn resize_algebraic_part(&mut self, size: usize) {
        self.algebraic_part.truncate(size + 1);
    }

    pub fn resize_quadratic_part(&mut self, size: usize) {
        self.quadratic_part.truncate(size + 1);
    }
}