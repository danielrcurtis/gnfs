// src/matrix/matrix_solve.rs

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use num::{BigInt, ToPrimitive};
use crate::core::gnfs::GNFS;
use crate::core::gnfs_integer::GnfsInteger;
use crate::core::count_dictionary::CountDictionary;
use crate::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress;
use crate::matrix::gaussian_matrix::GaussianMatrix;
use crate::core::serialization::save;
use crate::core::serialization::load;
use crate::square_root::square_finder::is_square;
pub struct MatrixSolve;

impl MatrixSolve {
    pub fn gaussian_solve<T: GnfsInteger>(cancel_token: &Arc<AtomicBool>, gnfs: &mut GNFS<T>) {
        // TODO: Re-enable serialization once it's properly fixed
        // Relations are already in memory from the sieving stage, no need to save/load
        // save::relations::smooth::append(gnfs);
        // load::relations::smooth(gnfs);

        let smooth_relations = gnfs.current_relations_progress.relations.smooth_relations.clone();
        let smooth_count = smooth_relations.len();
        let required_relations_count = PolyRelationsSieveProgress::smooth_relations_required_for_matrix_step(gnfs);
    
        gnfs.log_message_slice(&format!("Total relations count: {}", smooth_count));
        gnfs.log_message_slice(&format!("Relations required to proceed: {}", required_relations_count));
    
        if smooth_relations.len() >= required_relations_count.to_usize().unwrap() {
            let selected_relations = smooth_relations.clone();

            // Collect valid solutions first, then add them after dropping the matrix
            let mut valid_solutions = Vec::new();

            {
                // Create and set up the Gaussian matrix with proper workflow
                let mut gaussian_reduction = GaussianMatrix::new(gnfs, &selected_relations);

                // Step 1: Build the matrix structure by transposing
                gaussian_reduction.transpose_append();

                let num_rows = gaussian_reduction.m.num_rows;
                let num_cols = gaussian_reduction.m.num_cols;
                gaussian_reduction._gnfs.log_message_slice(&format!("Matrix after transpose: {} rows x {} cols", num_rows, num_cols));

                // Step 2: Perform Gaussian elimination
                gaussian_reduction.elimination();

                // Step 3: Calculate appropriate solution count based on free variables
                // The number of solutions is the number of free variables minus 1
                let num_free_cols = gaussian_reduction.free_cols.iter().filter(|&&x| x).count();
                let solution_count = if num_free_cols > 0 { num_free_cols - 1 } else { 0 };

                gaussian_reduction._gnfs.log_message_slice(&format!("Free variables: {}, Solution sets to test: {}", num_free_cols, solution_count));

                // Step 4: Extract and test each solution
                for number in 1..=solution_count {
                    let relations = gaussian_reduction.get_solution_set(number);

                    gaussian_reduction._gnfs.log_message_slice(&format!("Testing solution set {} with {} relations", number, relations.len()));

                    let algebraic: BigInt = relations.iter().map(|rel| rel.algebraic_norm.to_bigint()).product();
                    let rational: BigInt = relations.iter().map(|rel| rel.rational_norm.to_bigint()).product();

                    let mut alg_count_dict = CountDictionary::new();

                    for rel in &relations {
                        alg_count_dict.combine(&rel.algebraic_factorization);
                    }

                    let is_algebraic_square = is_square(&algebraic);
                    let is_rational_square = is_square(&rational);

                    gaussian_reduction._gnfs.log_message_slice(&format!("  Algebraic norm is square: {}", is_algebraic_square));
                    gaussian_reduction._gnfs.log_message_slice(&format!("  Rational norm is square: {}", is_rational_square));

                    if is_algebraic_square && is_rational_square {
                        gaussian_reduction._gnfs.log_message_slice(&format!("Found valid solution {} (both norms are perfect squares)", number));
                        valid_solutions.push((number, relations));
                    }

                    if cancel_token.load(Ordering::SeqCst) {
                        break;
                    }
                }
                // Matrix dropped here, releasing the mutable borrow of gnfs
            }

            // Now add the valid solutions using the original gnfs reference
            for (_number, relations) in valid_solutions {
                gnfs.current_relations_progress.add_free_relation_solution(relations);
            }
        }
    }    
}