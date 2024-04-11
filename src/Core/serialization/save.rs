// src/core/serialization/save.rs

use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde_json;
use crate::polynomial::Polynomial;
use crate::factor_pair::{FactorPairCollection, FactorPair};
use crate::relations::{Relation, RelationContainer};
use crate::core::gnfs::{GNFS, DirectoryLocations, RelationsProgress};

pub mod serialization {
    use super::*;

    pub mod save {
        use super::*;

        pub fn object<T: Serialize>(obj: &T, filename: &str) {
            let save_json = serde_json::to_string_pretty(obj).expect("Failed to serialize object");
            fs::write(filename, save_json).expect("Failed to write file");
        }

        pub fn all(gnfs: &GNFS) {
            save::gnfs(gnfs);

            let mut counter = 1;
            for poly in &gnfs.polynomial_collection {
                let filename = format!("Polynomial.{:02}", counter);
                save::object(poly, &gnfs.save_locations.save_directory.join(filename));
                counter += 1;
            }

            save::factor_pair::rational(gnfs);
            save::factor_pair::algebraic(gnfs);
            save::factor_pair::quadratic(gnfs);

            save::relations::smooth::append(gnfs);
            save::relations::rough::append(gnfs);
            save::relations::free::all_solutions(gnfs);
        }

        pub fn gnfs(gnfs: &GNFS) {
            save::object(gnfs, &gnfs.save_locations.gnfs_parameters_save_file);
        }

        pub mod factor_pair {
            use super::*;

            pub fn rational(gnfs: &GNFS) {
                if !gnfs.rational_factor_pair_collection.is_empty() {
                    save::object(&gnfs.rational_factor_pair_collection, &gnfs.save_locations.rational_factor_pair_save_file);
                }
            }

            pub fn algebraic(gnfs: &GNFS) {
                if !gnfs.algebraic_factor_pair_collection.is_empty() {
                    save::object(&gnfs.algebraic_factor_pair_collection, &gnfs.save_locations.algebraic_factor_pair_save_file);
                }
            }

            pub fn quadratic(gnfs: &GNFS) {
                if !gnfs.quadratic_factor_pair_collection.is_empty() {
                    save::object(&gnfs.quadratic_factor_pair_collection, &gnfs.save_locations.quadratic_factor_pair_save_file);
                }
            }
        }

        pub mod relations {
            use super::*;

            pub mod smooth {
                use super::*;

                pub fn append(gnfs: &mut GNFS) {
                    if !gnfs.current_relations_progress.relations.smooth_relations.is_empty() {
                        let to_save: Vec<&Relation> = gnfs.current_relations_progress.relations.smooth_relations
                            .iter()
                            .filter(|rel| !rel.is_persisted)
                            .collect();
                        for rel in to_save {
                            append_relation(gnfs, rel);
                        }
                    }
                }

                fn append_relation(gnfs: &mut GNFS, relation: &Relation) {
                    if relation.is_smooth && !relation.is_persisted {
                        let mut json = serde_json::to_string_pretty(relation).expect("Failed to serialize relation");

                        if Path::new(&gnfs.save_locations.smooth_relations_save_file).exists() {
                            json.insert_str(0, ",");
                        }

                        fs::write(&gnfs.save_locations.smooth_relations_save_file, json)
                            .expect("Failed to append smooth relation");

                        gnfs.current_relations_progress.smooth_relations_counter += 1;

                        relation.is_persisted = true;
                    }
                }
            }

            pub mod rough {
                use super::*;

                pub fn append(gnfs: &mut GNFS) {
                    if !gnfs.current_relations_progress.relations.rough_relations.is_empty() {
                        let to_save: Vec<&Relation> = gnfs.current_relations_progress.relations.rough_relations
                            .iter()
                            .filter(|rel| !rel.is_persisted)
                            .collect();
                        for rel in to_save {
                            append_relation(gnfs, rel);
                        }
                    }
                }

                fn append_relation(gnfs: &mut GNFS, rough_relation: &Relation) {
                    if !rough_relation.is_smooth && !rough_relation.is_persisted {
                        let mut json = serde_json::to_string_pretty(rough_relation).expect("Failed to serialize rough relation");

                        if Path::new(&gnfs.save_locations.rough_relations_save_file).exists() {
                            json.push(',');
                        }

                        fs::write(&gnfs.save_locations.rough_relations_save_file, json)
                            .expect("Failed to append rough relation");
                        rough_relation.is_persisted = true;
                    }
                }
            }

            pub mod free {
                use super::*;

                pub fn all_solutions(gnfs: &mut GNFS) {
                    if !gnfs.current_relations_progress.relations.free_relations.is_empty() {
                        gnfs.current_relations_progress.free_relations_counter = 1;
                        for solution in &gnfs.current_relations_progress.relations.free_relations {
                            single_solution(gnfs, solution);
                        }
                    }
                }

                fn single_solution(gnfs: &mut GNFS, solution: &[Relation]) {
                    if !solution.is_empty() {
                        for rel in solution {
                            rel.is_persisted = true;
                        }
                        let filename = format!("free_relations_{}.json", gnfs.current_relations_progress.free_relations_counter);
                        save::object(solution, &gnfs.save_locations.save_directory.join(filename));
                        gnfs.current_relations_progress.free_relations_counter += 1;
                    }
                }
            }
        }
    }
}