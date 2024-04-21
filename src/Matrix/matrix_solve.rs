// src/matrix/matrix_solve.rs

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use num::{BigInt, ToPrimitive};
use crate::core::gnfs::GNFS;
use crate::core::count_dictionary::CountDictionary;
use crate::matrix::gaussian_matrix::GaussianMatrix;
use crate::core::serialization::save;
use crate::core::serialization::load;
use crate::relation_sieve::relation::Relation;
use crate::square_root::square_finder::is_square;
pub struct MatrixSolve;

impl MatrixSolve {
    pub fn gaussian_solve(cancel_token: &Arc<AtomicBool>, gnfs: &mut GNFS) {
        save::relations::smooth::append(gnfs);
        load::relations::smooth(gnfs);
    
        let smooth_relations = gnfs.current_relations_progress.relations.smooth_relations.clone();
        let smooth_count = smooth_relations.len();
        let required_relations_count = &gnfs.current_relations_progress.smooth_relations_required_for_matrix_step();
    
        gnfs.log_message_slice(&format!("Total relations count: {}", smooth_count));
        gnfs.log_message_slice(&format!("Relations required to proceed: {}", required_relations_count));
    
        while smooth_relations.len() >= required_relations_count.to_usize().unwrap() {
            let selected_relations = smooth_relations.clone();
            let solution_count = 10; // This should be retrieved appropriately
            let mut number = 0;
    
            while number <= solution_count {
                let (relations, algebraic, rational) = {
                    let gaussian_reduction = GaussianMatrix::new(gnfs, &selected_relations);
                    let relations = gaussian_reduction.get_solution_set(number);
    
                    let algebraic: BigInt = relations.iter().map(|rel| &rel.algebraic_norm).product();
                    let rational: BigInt = relations.iter().map(|rel| &rel.rational_norm).product();
                    (relations, algebraic, rational)
                };
    
                number += 1;
    
                let mut alg_count_dict = CountDictionary::new();
                let mut solution: Vec<Vec<Relation>> = Vec::new();
    
                for rel in &relations {
                    alg_count_dict.combine(&rel.algebraic_factorization);
                }
    
                let is_algebraic_square = is_square(&algebraic);
                let is_rational_square = is_square(&rational);
    
                if is_algebraic_square && is_rational_square {
                    solution.push(relations.clone());
                    gnfs.current_relations_progress.add_free_relation_solution(relations.clone());
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