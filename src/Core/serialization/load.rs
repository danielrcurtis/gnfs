// src/core/serialization/load.rs

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json;
use crate::polynomial::{Polynomial, Term};
use crate::factor_base::{RationalFactorBase, AlgebraicFactorBase, QuadraticFactorBase};
use crate::factor_pair::{FactorPairCollection, FactorPair};
use crate::relations::{Relation, RelationContainer};
use crate::gnfs::{GNFS, DirectoryLocations, RelationsProgress};

pub mod serialization {
    use super::*;

    pub mod load {
        use super::*;

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
            let mut gnfs: GNFS = serde_json::from_str(&load_json).expect("Failed to deserialize GNFS");

            let directory_name = Path::new(filename).parent().unwrap();
            gnfs.save_locations = DirectoryLocations::new(directory_name);

            let mut counter = 0;
            let mut finished = false;
            while !finished {
                counter += 1;
                let poly_filename = gnfs.save_locations.save_directory.join(format!("Polynomial.{:02}", counter));
                if poly_filename.exists() {
                    let deserialized_poly = load::polynomial(&poly_filename);
                    gnfs.polynomial_collection.push(deserialized_poly);
                } else {
                    finished = true;
                }
            }

            gnfs.current_polynomial = gnfs.polynomial_collection.first().unwrap().clone();
            gnfs.polynomial_degree = gnfs.current_polynomial.degree;

            load::factor_base(&mut gnfs);

            load::factor_pair::rational(&mut gnfs);
            load::factor_pair::algebraic(&mut gnfs);
            load::factor_pair::quadratic(&mut gnfs);

            gnfs.current_relations_progress.gnfs = Some(gnfs.clone());

            load::relations::smooth(&mut gnfs);
            load::relations::rough(&mut gnfs);
            load::relations::free(&mut gnfs);

            gnfs
        }

        pub fn polynomial(filename: &str) -> Polynomial {
            let poly_json = fs::read_to_string(filename).expect("Failed to read polynomial file");
            serde_json::from_str(&poly_json).expect("Failed to deserialize polynomial")
        }

        pub fn factor_base(gnfs: &mut GNFS) {
            gnfs.set_prime_factor_bases();
        }

        pub mod factor_pair {
            use super::*;

            pub fn rational(gnfs: &mut GNFS) {
                if gnfs.save_locations.rational_factor_pair_save_file.exists() {
                    gnfs.rational_factor_pair_collection = load::generic(&gnfs.save_locations.rational_factor_pair_save_file);
                }
            }

            pub fn algebraic(gnfs: &mut GNFS) {
                if gnfs.save_locations.algebraic_factor_pair_save_file.exists() {
                    gnfs.algebraic_factor_pair_collection = load::generic(&gnfs.save_locations.algebraic_factor_pair_save_file);
                }
            }

            pub fn quadratic(gnfs: &mut GNFS) {
                if gnfs.save_locations.quadratic_factor_pair_save_file.exists() {
                    gnfs.quadratic_factor_pair_collection = load::generic(&gnfs.save_locations.quadratic_factor_pair_save_file);
                }
            }
        }

        pub mod relations {
            use super::*;

            pub fn smooth(gnfs: &mut GNFS) {
                if gnfs.save_locations.smooth_relations_save_file.exists() {
                    let mut temp: Vec<Relation> = load::generic_fixed_array(&gnfs.save_locations.smooth_relations_save_file);
                    let null_rels: Vec<&Relation> = temp.iter().filter(|x| x.is_none()).collect();
                    if !null_rels.is_empty() {
                        temp = temp.into_iter().filter(|x| x.is_some()).map(|x| x.unwrap()).collect();
                    }
                    temp.iter_mut().for_each(|rel| rel.is_persisted = true);
                    gnfs.current_relations_progress.smooth_relations_counter = temp.len();
                    gnfs.current_relations_progress.relations.smooth_relations = temp;
                }
            }

            pub fn rough(gnfs: &mut GNFS) {
                if gnfs.save_locations.rough_relations_save_file.exists() {
                    let mut temp: Vec<Relation> = load::generic_fixed_array(&gnfs.save_locations.rough_relations_save_file);
                    temp.iter_mut().for_each(|rel| rel.is_persisted = true);
                    gnfs.current_relations_progress.relations.rough_relations = temp;
                }
            }

            pub fn free(gnfs: &mut GNFS) {
                let unsaved: Vec<&Vec<Relation>> = gnfs.current_relations_progress.relations.free_relations
                    .iter()
                    .filter(|lst| lst.iter().any(|rel| !rel.is_persisted))
                    .collect();

                for solution in unsaved {
                    let unsaved_file = gnfs.save_locations.save_directory.join(format!("!!UNSAVED__free_relations.json"));
                    save::object(solution, &unsaved_file);
                }

                gnfs.current_relations_progress.relations.free_relations.clear();
                gnfs.current_relations_progress.free_relations_counter = 0;

                let free_relations = gnfs.save_locations.enumerate_free_relation_files();
                for solution in free_relations {
                    let temp: Vec<Relation> = load::generic(&solution);
                    temp.iter_mut().for_each(|rel| rel.is_persisted = true);
                    gnfs.current_relations_progress.relations.free_relations.push(temp);
                    gnfs.current_relations_progress.free_relations_counter += 1;
                }
            }
        }
    }
}