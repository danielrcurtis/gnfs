// src/matrix/gaussian_matrix.rs

use std::collections::HashMap;
use crate::core::gnfs::GNFS;
use crate::relation_sieve::relation::Relation;
use crate::matrix::gaussian_row::GaussianRow;

pub struct GaussianMatrix {
    m: Vec<Vec<bool>>,
    free_cols: Vec<bool>,
    elimination_step: bool,
    _gnfs: GNFS,
    relations: Vec<Relation>,
    column_index_relation_dictionary: HashMap<usize, Relation>,
    relation_matrix_tuple: Vec<(Relation, Vec<bool>)>,
}

impl GaussianMatrix {
    
    pub fn new(gnfs: GNFS, rels: &[Relation]) -> Self {
        let mut relation_matrix_tuple = Vec::new();
        let elimination_step = false;
        let free_cols = Vec::new();
        let m = Vec::new();

        let relations = rels.to_vec();

        let mut relations_as_rows: Vec<GaussianRow> = relations
            .iter()
            .map(|rel| GaussianRow::new(&gnfs, rel.clone()))
            .collect();

        let selected_rows: Vec<GaussianRow> = relations_as_rows
            .iter_mut()
            .take(gnfs.current_relations_progress.smooth_relations_required_for_matrix_step.to_usize().unwrap())
            .map(|row| row.to_owned())
            .collect();

        let max_index_rat = selected_rows.iter().map(|row| row.last_index_of_rational().unwrap_or(0)).max().unwrap();
        let max_index_alg = selected_rows.iter().map(|row| row.last_index_of_algebraic().unwrap_or(0)).max().unwrap();
        let max_index_qua = selected_rows.iter().map(|row| row.last_index_of_quadratic().unwrap_or(0)).max().unwrap();

        for row in &mut relations_as_rows {
            row.resize_rational_part(max_index_rat);
            row.resize_algebraic_part(max_index_alg);
            row.resize_quadratic_part(max_index_qua);
        }

        let example_row = relations_as_rows.first().unwrap();
        let mut new_length = example_row.get_bool_array().len();
        new_length += 1;

        relations_as_rows = relations_as_rows.into_iter().take(new_length).collect();

        for row in relations_as_rows {
            relation_matrix_tuple.push((row.source_relation, row.get_bool_array()));
        }

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
        let mut result = Vec::new();
        self.column_index_relation_dictionary = HashMap::new();

        let num_rows = self.relation_matrix_tuple[0].1.len();
        for index in 0..num_rows {
            self.column_index_relation_dictionary.insert(index, self.relation_matrix_tuple[index].0.clone());

            let mut new_row: Vec<bool> = self.relation_matrix_tuple.iter().map(|bv| bv.1[index]).collect();
            new_row.push(false);
            result.push(new_row);
        }

        self.m = result;
        self.free_cols = vec![false; self.m.len()];
    }

    pub fn elimination(&mut self) {
        if self.elimination_step {
            return;
        }

        let num_rows = self.m.len();
        let num_cols = self.m[0].len();

        self.free_cols = vec![false; num_cols];

        let mut h = 0;

        for i in 0..num_rows {
            if h >= num_cols {
                break;
            }

            let mut next = false;

            if !self.m[i][h] {
                let mut t = i + 1;

                while t < num_rows && !self.m[t][h] {
                    t += 1;
                }

                if t < num_rows {
                    self.m.swap(i, t);
                } else {
                    self.free_cols[h] = true;
                    next = true;
                }
            }

            if !next {
                for j in i + 1..num_rows {
                    if self.m[j][h] {
                        self.m[j] = Self::add(&self.m[j], &self.m[i]);
                    }
                }

                for j in 0..i {
                    if self.m[j][h] {
                        self.m[j] = Self::add(&self.m[j], &self.m[i]);
                    }
                }
            }

            h += 1;
        }

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

        let num_rows = self.m.len();
        let num_cols = self.m[0].len();

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
            if self.m[i][j - 1] {
                let mut h = i;
                while h < j - 1 {
                    if self.m[i][h] {
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
        Self::matrix_to_string(&self.m)
    }

}
