// src/core/directory_locations.rs
use std::path::PathBuf;
use num::BigInt;

const SHOW_DIGITS: usize = 22;
const ELLIPSIS: &str = "[...]";

pub struct DirectoryLocations {
    base_directory: String,
    pub save_directory: String,
    rational_factor_pair_filepath: String,
    algebraic_factor_pair_filepath: String,
    quadratic_factor_pair_filepath: String,
    parameters_filepath: String,
    smooth_relations_filepath: String,
    rough_relations_filepath: String,
}

impl DirectoryLocations {
    pub fn new(save_location: &str) -> Self {
        let save_directory = save_location.to_string();
        let gnfs_parameters_save_file = format!("{}/GNFS.json", save_directory);
        let rational_factor_pair_save_file = format!("{}/RationalFactorPairCollection.json", save_directory);
        let algebraic_factor_pair_save_file = format!("{}/AlgebraicFactorPairCollection.json", save_directory);
        let quadratic_factor_pair_save_file = format!("{}/QuadraticFactorPairCollection.json", save_directory);
        let smooth_relations_save_file = format!("{}/SmoothRelations.json", save_directory);
        let rough_relations_save_file = format!("{}/RoughRelations.json", save_directory);

        DirectoryLocations {
            base_directory: "GNFS".to_string(),
            save_directory,
            rational_factor_pair_filepath: rational_factor_pair_save_file,
            algebraic_factor_pair_filepath: algebraic_factor_pair_save_file,
            quadratic_factor_pair_filepath: quadratic_factor_pair_save_file,
            parameters_filepath: gnfs_parameters_save_file,
            smooth_relations_filepath: smooth_relations_save_file,
            rough_relations_filepath: rough_relations_save_file,
        }
    }

    pub fn set_base_directory(&mut self, path: &str) {
        self.base_directory = path.to_string();
    }

    pub fn get_save_location(&self, n: &BigInt) -> String {
        let directory_name = Self::get_unique_name_from_n(n);
        format!("{}/{}", self.base_directory, directory_name)
    }

    fn get_unique_name_from_n(n: &BigInt) -> String {
        let result = n.to_string();
        if result.len() >= (SHOW_DIGITS * 2) + ELLIPSIS.len() {
            format!(
                "{}{}{}",
                &result[..SHOW_DIGITS],
                ELLIPSIS,
                &result[result.len() - SHOW_DIGITS..]
            )
        } else {
            result
        }
    }

    pub fn enumerate_free_relation_files(&self) -> Vec<String> {
        // Implement the logic to enumerate free relation files
        Vec::new()
    }
}