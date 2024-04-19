use log::{info, warn, debug, trace, error};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use rand::Rng;
use num::{BigInt, ToPrimitive};
use crate::core::gnfs::GNFS;
use crate::core::count_dictionary::CountDictionary;
use crate::matrix::gaussian_matrix::GaussianMatrix;
use crate::core::serialization::save;
use crate::core::serialization::load;
use crate::square_root::square_finder::is_square;
pub struct MatrixSolve;

impl MatrixSolve {
    pub fn gaussian_solve(cancel_token: &Arc<AtomicBool>, gnfs: &mut GNFS) {
        save::relations::smooth::append(gnfs);
        load::relations::smooth(gnfs);

        let mut smooth_relations = gnfs.current_relations_progress.relations.smooth_relations.clone();
        let smooth_count = smooth_relations.len();
        let required_relations_count = &gnfs.current_relations_progress.smooth_relations_required_for_matrix_step();

        gnfs.log_message_slice(&format!("Total relations count: {}", smooth_count));
        gnfs.log_message_slice(&format!("Relations required to proceed: {}", required_relations_count));

        while smooth_relations.len() >= required_relations_count.to_usize().unwrap() {
            let mut selected_relations = Vec::new();
            while selected_relations.len() < required_relations_count.to_usize().unwrap() || selected_relations.len() % 2 != 0 {
                let random_index = rand::thread_rng().gen_range(0..smooth_relations.len());
                selected_relations.push(smooth_relations.remove(random_index));
            }

            let mut gaussian_reduction = GaussianMatrix::new(*gnfs, &selected_relations);
            gaussian_reduction.transpose_append();
            gaussian_reduction.elimination();

            let mut number = 1;
            let solution_count = gaussian_reduction.free_cols.iter().filter(|&&b| b).count() - 1;
            let mut solution = Vec::new();

            while number <= solution_count {
                let relations = gaussian_reduction.get_solution_set(number);
                number += 1;

                let algebraic: BigInt = relations.iter().map(|rel| &rel.algebraic_norm).product();
                let rational: BigInt = relations.iter().map(|rel| &rel.rational_norm).product();

                let mut alg_count_dict = CountDictionary::new();
                for rel in &relations {
                    alg_count_dict.combine(&rel.algebraic_factorization);
                }

                let is_algebraic_square =  is_square(&algebraic);
                let is_rational_square = is_square(&rational);

                if is_algebraic_square && is_rational_square {
                    solution.push(relations);
                    gnfs.current_relations_progress.add_free_relation_solution(relations);
                }

                if cancel_token.load(Ordering::SeqCst) {
                    break;
                }
            }

            if cancel_token.load(Ordering::SeqCst) {
                break;
            }
        }
    }
}