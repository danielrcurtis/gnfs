// src/integer_math/combinatorics.rs

use std::iter::empty;

pub struct Combinatorics;

impl Combinatorics {
    /// Returns the Cartesian product of two or more lists
    pub fn cartesian_product<T: Clone>(sequences: &[Vec<T>]) -> Vec<Vec<T>> {
        let empty: Vec<Vec<T>> = vec![vec![]];

        sequences.iter().fold(empty, |first, second| {
            first.iter().flat_map(|a| {
                second.iter().map(move |b| {
                    let mut concat = a.clone();
                    concat.push(b.clone());
                    concat
                })
            }).collect()
        })
    }
}