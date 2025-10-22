// src/matrix/gaussian_matrix.rs

use std::collections::HashMap;
use crate::core::gnfs::GNFS;
use crate::relation_sieve::relation::Relation;
use crate::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress;
use crate::matrix::gaussian_row::GaussianRow;
use crate::matrix::sparse_matrix::SparseMatrix;
use num::ToPrimitive;

pub struct GaussianMatrix<'a> {
    pub m: SparseMatrix,
    pub free_cols: Vec<bool>,
    pub elimination_step: bool,
    pub _gnfs: &'a mut GNFS,  // Apply the lifetime to this reference
    pub relations: Vec<Relation>,
    pub column_index_relation_dictionary: HashMap<usize, Relation>,
    pub relation_matrix_tuple: Vec<(Relation, Vec<bool>)>,
}

impl GaussianMatrix<'_> {
    
    pub fn new<'a>(gnfs: &'a mut GNFS, rels: &[Relation]) -> GaussianMatrix<'a> {
        let mut relation_matrix_tuple = Vec::new();
        let elimination_step = false;
        let free_cols = Vec::new();

        let relations = rels.to_vec();

        let mut relations_as_rows: Vec<GaussianRow> = relations
            .iter()
            .map(|rel| GaussianRow::new(&gnfs, rel.clone()))
            .collect();

        let mut selected_rows: Vec<GaussianRow> = relations_as_rows
            .iter_mut()
            .take(PolyRelationsSieveProgress::smooth_relations_required_for_matrix_step(&gnfs).to_usize().unwrap())
            .map(|row| row.to_owned())
            .collect();

        let max_index_rat = selected_rows.iter().map(|row| row.last_index_of_rational().unwrap_or(0)).max().unwrap();
        let max_index_alg = selected_rows.iter().map(|row| row.last_index_of_algebraic().unwrap_or(0)).max().unwrap();
        let max_index_qua = selected_rows.iter().map(|row| row.last_index_of_quadratic().unwrap_or(0)).max().unwrap();

        for row in &mut selected_rows {
            row.resize_rational_part(max_index_rat);
            row.resize_algebraic_part(max_index_alg);
            row.resize_quadratic_part(max_index_qua);
        }

        let example_row = selected_rows.first().unwrap();
        let mut new_length = example_row.get_bool_array().len();

        new_length += 1;

        let selected_rows: Vec<GaussianRow> = selected_rows.into_iter().take(new_length).collect();

        for row in selected_rows {
            relation_matrix_tuple.push((row.source_relation.clone(), row.get_bool_array()));
        }

        // Initialize sparse matrix with dimensions (will be populated in transpose_append)
        let m = SparseMatrix::new(0, 0);

        GaussianMatrix {
            m,
            free_cols,
            elimination_step,
            _gnfs: gnfs,
            relations,
            column_index_relation_dictionary: HashMap::new(),
            relation_matrix_tuple,
        }
    }

    pub fn transpose_append(&mut self) {
        self.column_index_relation_dictionary = HashMap::new();

        let num_rows = self.relation_matrix_tuple[0].1.len();
        let num_cols = self.relation_matrix_tuple.len() + 1; // +1 for the appended column

        // Create sparse matrix with proper dimensions
        self.m = SparseMatrix::new(num_rows, num_cols);

        for index in 0..num_rows {
            self.column_index_relation_dictionary.insert(index, self.relation_matrix_tuple[index].0.clone());

            // Transpose: take the index-th element from each relation's bool array
            let new_row: Vec<bool> = self.relation_matrix_tuple.iter().map(|bv| bv.1[index]).collect();

            // Append false to the row
            let mut full_row = new_row;
            full_row.push(false);

            // Set the row in the sparse matrix
            self.m.set_row_from_dense(index, &full_row);
        }

        self.free_cols = vec![false; num_cols];
    }

    pub fn elimination(&mut self) {
        if self.elimination_step {
            return;
        }

        let num_rows = self.m.num_rows;
        let num_cols = self.m.num_cols;

        self._gnfs.log_message_slice(&format!("Gaussian elimination: matrix dimensions = {} rows x {} cols", num_rows, num_cols));
        self._gnfs.log_message_slice(&format!("Matrix sparsity: {:.2}%", self.m.sparsity_percentage()));

        self.free_cols = vec![false; num_cols];

        let mut h = 0;
        let mut i = 0;

        while i < num_rows && h < num_cols {
            let mut next = false;

            // Find pivot using sparse find_pivot (optimized for sparse data)
            if !self.m.get(i, h) {
                if let Some(pivot_row) = self.m.find_pivot(h, i + 1) {
                    self.m.swap_rows(i, pivot_row);
                } else {
                    self.free_cols[h] = true;
                    next = true;
                }
            }

            if !next {
                // Forward elimination - use sparse row_xor
                // Key optimization: row_xor only touches non-zero entries (10-100x speedup)
                for j in (i + 1)..num_rows {
                    if self.m.get(j, h) {
                        self.m.row_xor(j, i);
                    }
                }

                // Back substitution - use sparse row_xor
                for j in 0..i {
                    if self.m.get(j, h) {
                        self.m.row_xor(j, i);
                    }
                }

                i += 1;  // Only increment i when we processed the row
            }

            h += 1;
        }

        let free_count = self.free_cols.iter().filter(|&&x| x).count();
        self._gnfs.log_message_slice(&format!("Gaussian elimination complete: {} free variables found", free_count));

        self.elimination_step = true;
    }

    pub fn get_solution_set(&self, number_of_solutions: usize) -> Vec<Relation> {
        let solution_set = self.get_solution_flags(number_of_solutions);

        let mut index = 0;
        let max = self.column_index_relation_dictionary.len();

        let mut result = Vec::new();
        while index < max {
            if solution_set[index] {
                result.push(self.column_index_relation_dictionary[&index].clone());
            }
            index += 1;
        }

        result
    }

    fn get_solution_flags(&self, num_solutions: usize) -> Vec<bool> {
        if !self.elimination_step {
            panic!("Must call elimination() method first!");
        }

        if num_solutions < 1 {
            panic!("num_solutions must be greater than 1.");
        }

        let num_rows = self.m.num_rows;
        let num_cols = self.m.num_cols;

        if num_solutions >= num_cols {
            panic!("num_solutions must be less than the column count.");
        }

        let mut result = vec![false; num_cols];

        let mut j = 0;
        let mut i = num_solutions;

        while i > 0 {
            while !self.free_cols[j] {
                j += 1;
            }
            i -= 1;
            j += 1;
        }

        result[j - 1] = true;

        for i in 0..num_rows - 1 {
            if self.m.get(i, j - 1) {
                let mut h = i;
                while h < j - 1 {
                    if self.m.get(i, h) {
                        result[h] = true;
                        break;
                    }
                    h += 1;
                }
            }
        }

        result
    }

    pub fn add(left: &[bool], right: &[bool]) -> Vec<bool> {
        if left.len() != right.len() {
            panic!("Both vectors must have the same length.");
        }

        let length = left.len();
        let mut result = vec![false; length];

        for (index, (l, r)) in left.iter().zip(right.iter()).enumerate() {
            result[index] = *l ^ *r;
        }

        result
    }

    pub fn vector_to_string(vector: &[bool]) -> String {
        vector.iter().map(|&b| if b { '1' } else { '0' }).collect()
    }

    pub fn matrix_to_string(matrix: &[Vec<bool>]) -> String {
        matrix.iter().map(|row| Self::vector_to_string(row)).collect::<Vec<String>>().join("\n")
    }

    pub fn to_string(&self) -> String {
        // Convert sparse matrix to dense representation for string output
        let mut dense_rows = Vec::new();
        for i in 0..self.m.num_rows {
            dense_rows.push(self.m.get_row_dense(i));
        }
        Self::matrix_to_string(&dense_rows)
    }

}
