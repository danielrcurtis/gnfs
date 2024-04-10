// src/relation_sieve/relation_container.rs

use serde::{Deserialize, Serialize};
use std::vec::Vec;

use super::relation::Relation;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct RelationContainer {
    #[serde(skip)]
    pub smooth_relations: Vec<Relation>,
    #[serde(skip)]
    pub rough_relations: Vec<Relation>,
    #[serde(skip)]
    pub free_relations: Vec<Vec<Relation>>,
}

impl RelationContainer {
    pub fn new() -> Self {
        RelationContainer {
            smooth_relations: Vec::new(),
            rough_relations: Vec::new(),
            free_relations: Vec::new(),
        }
    }
}