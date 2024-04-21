// src/core/serialization/load.rs

use std::fs;
use std::path::Path;
use std::sync::Arc;
use crate::core::directory_location::DirectoryLocations;
use crate::polynomial::polynomial::Polynomial;
use serde_json;
use crate::relation_sieve::relation::Relation;
use crate::factor::factor_pair_collection::FactorPairCollection;
use crate::core::gnfs::GNFS;
use crate::core::serialization::save;
use crate::core::serialization::load;
use crate::core::serialization::types::{
    SerializableGNFS, SerializablePolynomial, SerializableFactorPairCollection,
    SerializableRelation,
};

pub fn generic<T: serde::de::DeserializeOwned>(filename: &str) -> T {
    let load_json = fs::read_to_string(filename).expect("Failed to read file");
    serde_json::from_str(&load_json).expect("Failed to deserialize JSON")
}

pub fn generic_fixed_array<T: serde::de::DeserializeOwned>(filename: &str) -> T {
    let load_json = fs::read_to_string(filename).expect("Failed to read file");
    let fixed_json = fix_appended_json_arrays(&load_json);
    serde_json::from_str(&fixed_json).expect("Failed to deserialize JSON")
}

fn fix_appended_json_arrays(input: &str) -> String {
    format!("[{}]", input.trim_start_matches(','))
}

pub fn all(filename: &str) -> GNFS {
    let load_json = fs::read_to_string(filename).expect("Failed to read file");
    let serializable_gnfs: SerializableGNFS = serde_json::from_str(&load_json).expect("Failed to deserialize GNFS");
    let mut gnfs = GNFS::from(serializable_gnfs);

    let directory_name = Path::new(filename).parent().unwrap();
    let directory_str = directory_name.to_str().expect("Failed to convert path to string");
    gnfs.save_locations = DirectoryLocations::new(directory_str);

    let mut counter = 0;
    let mut finished = false;
    while !finished {
        counter += 1;
        let poly_filename = format!("{}/Polynomial.{:02}", gnfs.save_locations.save_directory, counter);
        if Path::new(&poly_filename).exists() {
            let deserialized_poly = load::polynomial(&poly_filename);
            gnfs.polynomial_collection.push(deserialized_poly);
        } else {
            finished = true;
        }
    }

    gnfs.current_polynomial = gnfs.polynomial_collection.first().unwrap().clone();
    gnfs.polynomial_degree = gnfs.current_polynomial.degree();

    load::factor_base(&mut gnfs);
    load::factor_pair::rational(&mut gnfs);
    load::factor_pair::algebraic(&mut gnfs);
    load::factor_pair::quadratic(&mut gnfs);

    gnfs.current_relations_progress.gnfs = Arc::downgrade(&Arc::new(gnfs.clone()));

    load::relations::smooth(&mut gnfs);
    load::relations::rough(&mut gnfs);
    load::relations::free(&mut gnfs);

    gnfs
}

pub fn polynomial(filename: &str) -> Polynomial {
    let poly_json = fs::read_to_string(filename).expect("Failed to read polynomial file");
    let serializable_poly: SerializablePolynomial = serde_json::from_str(&poly_json).expect("Failed to deserialize polynomial");
    Polynomial::from(serializable_poly)
}

pub fn factor_base(gnfs: &mut GNFS) {
    gnfs.set_prime_factor_bases();
}

pub mod factor_pair {
    use super::*;

    pub fn rational(gnfs: &mut GNFS) {
        if Path::new(&gnfs.save_locations.rational_factor_pair_filepath).exists() {
            let serializable_collection: SerializableFactorPairCollection = load::generic(&gnfs.save_locations.rational_factor_pair_filepath);
            gnfs.rational_factor_pair_collection = FactorPairCollection::from(serializable_collection);
        }
    }

    pub fn algebraic(gnfs: &mut GNFS) {
        if Path::new(&gnfs.save_locations.algebraic_factor_pair_filepath).exists() {
            let serializable_collection: SerializableFactorPairCollection = load::generic(&gnfs.save_locations.algebraic_factor_pair_filepath);
            gnfs.algebraic_factor_pair_collection = FactorPairCollection::from(serializable_collection);
        }
    }

    pub fn quadratic(gnfs: &mut GNFS) {
        if Path::new(&gnfs.save_locations.quadratic_factor_pair_filepath).exists() {
            let serializable_collection: SerializableFactorPairCollection = load::generic(&gnfs.save_locations.quadratic_factor_pair_filepath);
            gnfs.quadratic_factor_pair_collection = FactorPairCollection::from(serializable_collection);
        }
    }
}

pub mod relations {
    use super::*;

    pub fn smooth(gnfs: &mut GNFS) {
        if Path::new(&gnfs.save_locations.smooth_relations_filepath).exists() {
            let mut temp: Vec<SerializableRelation> = load::generic_fixed_array(&gnfs.save_locations.smooth_relations_filepath);
            
            // Filter out relations where any field is empty
            temp.retain(|rel| 
                !(rel.a.is_empty() || rel.b.is_empty() || rel.algebraic_norm.is_empty() || rel.rational_norm.is_empty())
            );
            
            let mut relations: Vec<Relation> = temp.into_iter().map(|rel| Relation::from(rel)).collect();
            relations.iter_mut().for_each(|rel| rel.is_persisted = true);
            gnfs.current_relations_progress.smooth_relations_counter = relations.len();
            gnfs.current_relations_progress.relations.smooth_relations = relations;
        }
    }
    

    pub fn rough(gnfs: &mut GNFS) {
        if Path::new(&gnfs.save_locations.rough_relations_filepath).exists() {
            let temp: Vec<SerializableRelation> = load::generic_fixed_array(&gnfs.save_locations.rough_relations_filepath);
            let mut relations: Vec<Relation> = temp.into_iter().map(|rel| Relation::from(rel)).collect();
            relations.iter_mut().for_each(|rel| rel.is_persisted = true);
            gnfs.current_relations_progress.relations.rough_relations = relations;
        }
    }

    pub fn free(gnfs: &mut GNFS) {
        let unsaved: Vec<&Vec<Relation>> = gnfs.current_relations_progress.relations.free_relations
            .iter()
            .filter(|lst| lst.iter().any(|rel| !rel.is_persisted))
            .collect();

        for solution in unsaved {
            let serializable_solution: Vec<SerializableRelation> = solution.iter().map(|rel| SerializableRelation::from(rel.clone())).collect();
            let unsaved_file = format!("{}/!!UNSAVED__free_relations.json", gnfs.save_locations.save_directory);
            save::object(&serializable_solution, &unsaved_file);
        }

        gnfs.current_relations_progress.relations.free_relations.clear();
        gnfs.current_relations_progress.free_relations_counter = 0;

        let free_relations = gnfs.save_locations.enumerate_free_relation_files();
        for solution in free_relations {
            let temp: Vec<SerializableRelation> = load::generic(&solution);
            let mut relations: Vec<Relation> = temp.into_iter().map(|rel| Relation::from(rel)).collect();
            relations.iter_mut().for_each(|rel| rel.is_persisted = true);
            gnfs.current_relations_progress.relations.free_relations.push(relations);
            gnfs.current_relations_progress.free_relations_counter += 1;
        }
    }
}