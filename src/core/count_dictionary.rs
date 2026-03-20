// src/core/count_dictionary.rs
use num::{BigInt, One, Zero};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountDictionary(pub BTreeMap<BigInt, BigInt>);

impl Default for CountDictionary {
    fn default() -> Self {
        CountDictionary(BTreeMap::new())
    }
}

impl CountDictionary {
    pub fn new() -> Self {
        Self::default()
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

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn to_dict(&self) -> BTreeMap<BigInt, BigInt> {
        self.0.clone()
    }

    pub fn clone_dict(&self) -> BTreeMap<BigInt, BigInt> {
        self.0.clone()
    }

    pub fn format_as_string(&self) -> String {
        format!("{}", self)
    }

    pub fn retain<F>(&mut self, predicate: F)
    where
        F: FnMut(&BigInt, &mut BigInt) -> bool,
    {
        self.0.retain(predicate);
    }

    pub fn format_string_as_factorization(&self) -> String {
        let factors: Vec<String> = self.0.iter().map(|(key, value)| format!("{}^{}", key, value)).collect();
        format!(" -> {{\t{}\t}};", factors.join(" * "))
    }
}

impl fmt::Display for CountDictionary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\n")?;
        for (key, value) in &self.0 {
            write!(f, "\t{:5}: {:5}\n", key, value)?;
        }
        write!(f, "}}")
    }
}