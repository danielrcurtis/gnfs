// src/core/serialization/save.rs

use std::fs;
use std::path::Path;
use std::io::Write;
use serde::Serialize;
use serde_json;
use crate::core::gnfs::GNFS;
use crate::core::serialization::save;
use crate::relation_sieve::relation::Relation;
use crate::core::serialization::types::{
    SerializableGNFS, SerializablePolynomial, SerializableFactorPairCollection,
    SerializableRelation
};

pub fn object<T: Serialize>(obj: &T, filename: &str) {
    let save_json = serde_json::to_string_pretty(obj).expect("Failed to serialize object");
    fs::write(filename, save_json).expect("Failed to write file");
}

pub fn all(gnfs: &GNFS) {
    save::gnfs(gnfs);

    let mut counter = 1;
    for poly in &gnfs.polynomial_collection {
        let filename = format!("Polynomial.{:02}", counter);
        let serializable_poly = SerializablePolynomial::from(poly.clone());
        save::object(&serializable_poly, &format!("{}/{}", gnfs.save_locations.save_directory, filename));
        counter += 1;
    }

    save::factor_pair::rational(gnfs);
    save::factor_pair::algebraic(gnfs);
    save::factor_pair::quadratic(gnfs);

    let gnfs = &mut gnfs.clone();
    save::relations::smooth::append(gnfs);
    save::relations::rough::append(gnfs);
    save::relations::free::all_solutions(gnfs);
}

pub fn gnfs(gnfs: &GNFS) {
    let serializable_gnfs = SerializableGNFS::from(gnfs.clone());
    save::object(&serializable_gnfs, &gnfs.save_locations.parameters_filepath);
}

pub mod factor_pair {
    use super::*;

    pub fn rational(gnfs: &GNFS) {
        if !gnfs.rational_factor_pair_collection.len() == 0 {
            let serializable_collection = SerializableFactorPairCollection::from(gnfs.rational_factor_pair_collection.clone());
            save::object(&serializable_collection, &gnfs.save_locations.rational_factor_pair_filepath);
        }
    }

    pub fn algebraic(gnfs: &GNFS) {
        if !gnfs.algebraic_factor_pair_collection.len() == 0 {
            let serializable_collection = SerializableFactorPairCollection::from(gnfs.algebraic_factor_pair_collection.clone());
            save::object(&serializable_collection, &gnfs.save_locations.algebraic_factor_pair_filepath);
        }
    }

    pub fn quadratic(gnfs: &GNFS) {
        if !gnfs.quadratic_factor_pair_collection.len() == 0 {
            let serializable_collection = SerializableFactorPairCollection::from(gnfs.quadratic_factor_pair_collection.clone());
            save::object(&serializable_collection, &gnfs.save_locations.quadratic_factor_pair_filepath);
        }
    }
}

pub mod relations {
    use super::*;

    pub mod smooth {
        use super::*;

        pub fn append(gnfs: &mut GNFS) {
            let mut relations_to_update = Vec::new();
            let mut smooth_relations = Vec::new();
        
            // Extract the smooth relations into a separate vector
            std::mem::swap(&mut gnfs.current_relations_progress.relations.smooth_relations, &mut smooth_relations);
        
            // Collect relations that need updating
            for relation in &mut smooth_relations {
                if !relation.is_persisted {
                    relations_to_update.push((relation.a.clone(), relation.b.clone()));
                }
            }
        
            // Apply updates to each relation after collecting all necessary changes
            for (a, b) in relations_to_update {
                for relation in &mut smooth_relations {
                    if relation.a == a && relation.b == b {
                        append_relation(gnfs, relation);
                    }
                }
            }
        
            // Swap the updated smooth relations back into GNFS
            std::mem::swap(&mut gnfs.current_relations_progress.relations.smooth_relations, &mut smooth_relations);
        }
        
        fn append_relation(gnfs: &mut GNFS, relation: &mut Relation) {
            if relation.is_smooth() && !relation.is_persisted {
                let serializable_relation = SerializableRelation::from(relation.clone());
                let mut json = serde_json::to_string_pretty(&serializable_relation)
                    .expect("Failed to serialize relation");
        
                let smooth_relations_filepath = &gnfs.save_locations.smooth_relations_filepath;
        
                if Path::new(smooth_relations_filepath).exists() {
                    json.insert_str(0, ",");
                    fs::OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open(smooth_relations_filepath)
                        .expect("Failed to open smooth relations file for appending")
                        .write_all(json.as_bytes())
                        .expect("Failed to append smooth relation");
                } else {
                    fs::write(smooth_relations_filepath, format!("[{}]", json))
                        .expect("Failed to write smooth relation");
                }
        
                gnfs.current_relations_progress.smooth_relations_counter += 1;
                relation.is_persisted = true;
            }
        }
        
    }

    pub mod rough {
        use super::*;
    
        pub fn append(gnfs: &mut GNFS) {
            let mut relations_to_update = Vec::new();
            let mut smooth_relations = Vec::new();
            
            // Extract the smooth relations into a separate vector
            std::mem::swap(&mut gnfs.current_relations_progress.relations.smooth_relations, &mut smooth_relations);
            
            // Collect relations that need updating
            for relation in &mut smooth_relations {
                if !relation.is_persisted && relation.is_smooth() {
                    relations_to_update.push((relation.a.clone(), relation.b.clone()));
                }
            }
            
            // Apply updates to each relation after collecting all necessary changes
            for (a, b) in relations_to_update {
                for relation in &mut smooth_relations {
                    if relation.a == a && relation.b == b {
                        append_relation(gnfs, relation);
                    }
                }
            }
            
            // Swap the updated smooth relations back into GNFS
            std::mem::swap(&mut gnfs.current_relations_progress.relations.smooth_relations, &mut smooth_relations);
        }
    
        fn append_relation(gnfs: &mut GNFS, relation: &mut Relation) {
            let serializable_relation = SerializableRelation::from(relation.clone());
            let mut json = serde_json::to_string_pretty(&serializable_relation).expect("Failed to serialize relation");
    
            if Path::new(&gnfs.save_locations.smooth_relations_filepath).exists() {
                json.insert_str(0, ",");
            }
    
            fs::write(&gnfs.save_locations.smooth_relations_filepath, json)
                .expect("Failed to append smooth relation");
    
            gnfs.current_relations_progress.smooth_relations_counter += 1;
            relation.is_persisted = true;
        }
    }

    pub mod free {
        use super::*;
    
        pub fn all_solutions(gnfs: &mut GNFS) {
            let solutions_to_save = Vec::new();
            let mut free_relations = Vec::new();
            
            // Extract the free relations into a separate vector
            std::mem::swap(&mut gnfs.current_relations_progress.relations.free_relations, &mut free_relations);
            
            // Collect solutions that need saving
            for mut solution in solutions_to_save {
                single_solution(gnfs, &mut solution);
            }
            
            // Swap the updated free relations back into GNFS
            std::mem::swap(&mut gnfs.current_relations_progress.relations.free_relations, &mut free_relations);
        }
    
        pub fn single_solution(gnfs: &mut GNFS, solution: &mut Vec<Relation>) {
            if !solution.is_empty() {
                for rel in solution.iter_mut() {
                    rel.is_persisted = true;
                }
                let serializable_solution: Vec<SerializableRelation> = solution.iter().map(|rel| SerializableRelation::from(rel.clone())).collect();
                let filename = format!("free_relations_{}.json", gnfs.current_relations_progress.free_relations_counter);
                save::object(&serializable_solution, &format!("{}/{}", gnfs.save_locations.save_directory, filename));
                gnfs.current_relations_progress.free_relations_counter += 1;
            }
        }        
    }
    
}