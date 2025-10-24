// src/core/serialization/load.rs

use std::fs;
use std::path::Path;
use crate::core::directory_location::DirectoryLocations;
use crate::polynomial::polynomial::Polynomial;
use serde_json;
use crate::relation_sieve::relation::Relation;
use crate::factor::factor_pair_collection::FactorPairCollection;
use crate::core::gnfs::GNFS;
use crate::core::gnfs_integer::GnfsInteger;
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

pub fn parameters(filename: &str) -> SerializableGNFS {
    generic(filename)
}

pub fn progress(filename: &str) -> crate::core::serialization::types::SerializablePolyRelationsSieveProgress {
    generic(filename)
}

/// Load a complete GNFS checkpoint from the given directory
/// TODO: Re-implement with proper generics in Phase 3
pub fn load_checkpoint<T: GnfsInteger>(save_directory: &str, n: &num::BigInt) -> GNFS<T> {
    use log::info;

    // Load parameters.json
    let params_path = format!("{}/parameters.json", save_directory);
    let serializable_gnfs: SerializableGNFS = parameters(&params_path);

    // Validate that n matches
    let loaded_n = num::BigInt::parse_bytes(serializable_gnfs.n.as_bytes(), 10)
        .expect("Failed to parse n from parameters");
    if &loaded_n != n {
        panic!("Checkpoint n ({}) does not match expected n ({})", loaded_n, n);
    }

    // TODO: Phase 3 - Implement proper deserialization with generics
    // For now, this is a placeholder that panics
    panic!("load_checkpoint needs to be re-implemented with generics support");

    /*
    // Convert serializable GNFS to GNFS struct
    let mut gnfs = GNFS::from(serializable_gnfs);

    // Ensure the save_locations points to the correct directory
    gnfs.save_locations = DirectoryLocations::new(save_directory);

    // Load progress.json
    let progress_path = format!("{}/progress.json", save_directory);
    if Path::new(&progress_path).exists() {
        let serializable_progress = progress(&progress_path);

        // Convert and apply progress
        let mut loaded_progress = crate::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress::from(serializable_progress);

        // Preserve the relations container from the current progress
        // (we'll load relations separately)
        let relations_container = loaded_progress.relations.clone();
        gnfs.current_relations_progress = loaded_progress;
        gnfs.current_relations_progress.relations = relations_container;

        info!("Loaded progress: A={}, B={}", gnfs.current_relations_progress.a, gnfs.current_relations_progress.b);
        info!("Progress counters: smooth={}/{}, free={}",
              gnfs.current_relations_progress.smooth_relations_counter,
              gnfs.current_relations_progress.smooth_relations_target_quantity,
              gnfs.current_relations_progress.free_relations_counter);
    }

    // Load factor pair collections (they were saved in parameters.json, so already loaded)
    // But we can optionally reload them from separate files if they exist
    factor_pair::rational(&mut gnfs);
    factor_pair::algebraic(&mut gnfs);
    factor_pair::quadratic(&mut gnfs);

    // Reconstruct factor bases from the loaded parameters
    gnfs.set_prime_factor_bases();

    // Load smooth relations
    relations::smooth(&mut gnfs);

    // Validate polynomial
    let poly_result = gnfs.current_polynomial.evaluate(&gnfs.polynomial_base);
    if poly_result != gnfs.n {
        info!("Warning: Polynomial evaluation f(m) != n");
        info!("  f(m) = {}", poly_result);
        info!("  n    = {}", gnfs.n);
    } else {
        info!("Polynomial validation: f(m) = n ✓");
    }

    info!("Successfully loaded checkpoint from {}", save_directory);
    info!("  Relations loaded: {}", gnfs.current_relations_progress.smooth_relations_counter);
    info!("  Position: A={}, B={}", gnfs.current_relations_progress.a, gnfs.current_relations_progress.b);

    gnfs
    */
}

pub fn generic_fixed_array<T: serde::de::DeserializeOwned>(filename: &str) -> T {
    let load_json = fs::read_to_string(filename).expect("Failed to read file");
    let fixed_json = fix_appended_json_arrays(&load_json);
    serde_json::from_str(&fixed_json).expect("Failed to deserialize JSON")
}

fn fix_appended_json_arrays(input: &str) -> String {
    format!("[{}]", input.trim_start_matches(','))
}

// TODO: Temporarily disabled - requires GNFS serialization to be re-implemented
/*
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

    // TODO: Removed Weak<GNFS> field - no longer needed as we pass &GNFS by reference
    // gnfs.current_relations_progress.gnfs = Arc::downgrade(&Arc::new(gnfs.clone()));

    load::relations::smooth(&mut gnfs);
    load::relations::rough(&mut gnfs);
    load::relations::free(&mut gnfs);

    gnfs
}
*/

pub fn polynomial(filename: &str) -> Polynomial {
    let poly_json = fs::read_to_string(filename).expect("Failed to read polynomial file");
    let serializable_poly: SerializablePolynomial = serde_json::from_str(&poly_json).expect("Failed to deserialize polynomial");
    Polynomial::from(serializable_poly)
}

pub fn factor_base<T: GnfsInteger>(gnfs: &mut GNFS<T>) {
    gnfs.set_prime_factor_bases();
}

pub mod factor_pair {
    use super::*;

    pub fn rational<T: GnfsInteger>(gnfs: &mut GNFS<T>) {
        if Path::new(&gnfs.save_locations.rational_factor_pair_filepath).exists() {
            let serializable_collection: SerializableFactorPairCollection = load::generic(&gnfs.save_locations.rational_factor_pair_filepath);
            gnfs.rational_factor_pair_collection = FactorPairCollection::from(serializable_collection);
        }
    }

    pub fn algebraic<T: GnfsInteger>(gnfs: &mut GNFS<T>) {
        if Path::new(&gnfs.save_locations.algebraic_factor_pair_filepath).exists() {
            let serializable_collection: SerializableFactorPairCollection = load::generic(&gnfs.save_locations.algebraic_factor_pair_filepath);
            gnfs.algebraic_factor_pair_collection = FactorPairCollection::from(serializable_collection);
        }
    }

    pub fn quadratic<T: GnfsInteger>(gnfs: &mut GNFS<T>) {
        if Path::new(&gnfs.save_locations.quadratic_factor_pair_filepath).exists() {
            let serializable_collection: SerializableFactorPairCollection = load::generic(&gnfs.save_locations.quadratic_factor_pair_filepath);
            gnfs.quadratic_factor_pair_collection = FactorPairCollection::from(serializable_collection);
        }
    }
}

pub mod relations {
    use super::*;

    pub fn smooth<T: GnfsInteger>(gnfs: &mut GNFS<T>) {
        if Path::new(&gnfs.save_locations.smooth_relations_filepath).exists() {
            let mut temp: Vec<SerializableRelation> = load::generic(&gnfs.save_locations.smooth_relations_filepath);

            // Filter out relations where any field is empty
            temp.retain(|rel|
                !(rel.a.is_empty() || rel.b.is_empty() || rel.algebraic_norm.is_empty() || rel.rational_norm.is_empty())
            );

            let mut relations: Vec<Relation<T>> = temp.into_iter().map(|rel| rel.to_relation::<T>()).collect();
            relations.iter_mut().for_each(|rel| rel.is_persisted = true);
            gnfs.current_relations_progress.smooth_relations_counter = relations.len();
            gnfs.current_relations_progress.relations.smooth_relations = relations;
        }
    }


    pub fn rough<T: GnfsInteger>(gnfs: &mut GNFS<T>) {
        if Path::new(&gnfs.save_locations.rough_relations_filepath).exists() {
            let temp: Vec<SerializableRelation> = load::generic_fixed_array(&gnfs.save_locations.rough_relations_filepath);
            let mut relations: Vec<Relation<T>> = temp.into_iter().map(|rel| rel.to_relation::<T>()).collect();
            relations.iter_mut().for_each(|rel| rel.is_persisted = true);
            gnfs.current_relations_progress.relations.rough_relations = relations;
        }
    }

    pub fn free<T: GnfsInteger>(gnfs: &mut GNFS<T>) {
        let unsaved: Vec<&Vec<Relation<T>>> = gnfs.current_relations_progress.relations.free_relations
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
            let mut relations: Vec<Relation<T>> = temp.into_iter().map(|rel| rel.to_relation::<T>()).collect();
            relations.iter_mut().for_each(|rel| rel.is_persisted = true);
            gnfs.current_relations_progress.relations.free_relations.push(relations);
            gnfs.current_relations_progress.free_relations_counter += 1;
        }
    }
}