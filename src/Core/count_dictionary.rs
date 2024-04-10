// src/core/count_dictionary.rs

use num::{BigInt, BigUint, One, Zero};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountDictionary(BTreeMap<BigInt, BigInt>);

impl CountDictionary {
    pub fn new() -> Self {
        CountDictionary(BTreeMap::new())
    }

    pub fn add(&mut self, key: &BigInt) {
        self.add_safe(key, BigInt::one());
    }

    fn add_safe(&mut self, key: &BigInt, value: BigInt) {
        let entry = self.0.entry(key.clone()).or_insert_with(BigInt::zero);
        *entry += value;
    }

    pub fn combine(&mut self, other: &CountDictionary) {
        for (key, value) in &other.0 {
            self.add_safe(key, value.clone());
        }
    }

    pub fn to_dict(&self) -> BTreeMap<BigInt, BigInt> {
        self.0.clone()
    }

    pub fn clone_dict(&self) -> BTreeMap<BigInt, BigInt> {
        self.0.clone()
    }

    pub fn to_string(&self) -> String {
        let mut result = String::from("{\n");
        for (key, value) in &self.0 {
            result.push_str(&format!("\t{:5}: {:5}\n", key, value));
        }
        result.push('}');
        result
    }

    pub fn format_string_as_factorization(&self) -> String {
        let factors: Vec<String> = self.0.iter().map(|(key, value)| format!("{}^{}", key, value)).collect();
        format!(" -> {{\t{}\t}};", factors.join(" * "))
    }
}