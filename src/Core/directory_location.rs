// src/core/directory_locations.rs
use num::BigInt;
use serde::{Serialize, Deserialize};
const SHOW_DIGITS: usize = 22;
const ELLIPSIS: &str = "[...]";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryLocations {
    pub base_directory: String,
    pub save_directory: String,
    pub rational_factor_pair_filepath: String,
    pub algebraic_factor_pair_filepath: String,
    pub quadratic_factor_pair_filepath: String,
    pub parameters_filepath: String,
    pub progress_filepath: String,
    pub smooth_relations_filepath: String,
    pub rough_relations_filepath: String,
    pub streamed_relations_filepath: String,
}

impl DirectoryLocations {
    pub fn new(save_location: &str) -> Self {
        // Allow output directory to be configured via environment variable
        // Default to current directory (not /tmp) for safety
        let base_dir = std::env::var("GNFS_OUTPUT_DIR")
            .unwrap_or_else(|_| ".".to_string());

        let save_directory = if base_dir == "." {
            save_location.to_string()
        } else {
            format!("{}/{}", base_dir.trim_end_matches('/'), save_location)
        };
        let gnfs_parameters_save_file = format!("{}/parameters.json", save_directory);
        let progress_save_file = format!("{}/progress.json", save_directory);
        let rational_factor_pair_save_file = format!("{}/RationalFactorPairCollection.json", save_directory);
        let algebraic_factor_pair_save_file = format!("{}/AlgebraicFactorPairCollection.json", save_directory);
        let quadratic_factor_pair_save_file = format!("{}/QuadraticFactorPairCollection.json", save_directory);
        let smooth_relations_save_file = format!("{}/smooth_relations.json", save_directory);
        let rough_relations_save_file = format!("{}/RoughRelations.json", save_directory);
        let streamed_relations_save_file = format!("{}/streamed_relations.jsonl", save_directory);

        DirectoryLocations {
            base_directory: "GNFS".to_string(),
            save_directory,
            rational_factor_pair_filepath: rational_factor_pair_save_file,
            algebraic_factor_pair_filepath: algebraic_factor_pair_save_file,
            quadratic_factor_pair_filepath: quadratic_factor_pair_save_file,
            parameters_filepath: gnfs_parameters_save_file,
            progress_filepath: progress_save_file,
            smooth_relations_filepath: smooth_relations_save_file,
            rough_relations_filepath: rough_relations_save_file,
            streamed_relations_filepath: streamed_relations_save_file,
        }
    }

    pub fn set_base_directory(&mut self, path: &str) {
        self.base_directory = path.to_string();
    }

    pub fn get_save_location(&self, n: &BigInt) -> String {
        let directory_name = Self::get_unique_name_from_n(n);
        format!("{}/{}", self.base_directory, directory_name)
    }

    pub fn get_unique_name_from_n(n: &BigInt) -> String {
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

impl Default for DirectoryLocations {
    fn default() -> Self {
        DirectoryLocations {
            base_directory: "GNFS".to_string(),
            save_directory: "".to_string(),
            rational_factor_pair_filepath: "".to_string(),
            algebraic_factor_pair_filepath: "".to_string(),
            quadratic_factor_pair_filepath: "".to_string(),
            parameters_filepath: "".to_string(),
            progress_filepath: "".to_string(),
            smooth_relations_filepath: "".to_string(),
            rough_relations_filepath: "".to_string(),
            streamed_relations_filepath: "".to_string(),
        }
    }
}