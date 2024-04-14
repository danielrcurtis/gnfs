// src/relation_sieve/relation_container.rs

use std::vec::Vec;

use super::relation::Relation;

#[derive(Default, Debug, Clone)]
pub struct RelationContainer {
    pub smooth_relations: Vec<Relation>,
    pub rough_relations: Vec<Relation>,
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