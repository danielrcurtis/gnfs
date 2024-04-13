// src/core/serialization/json_converter.rs

use num::BigInt;
use serde::{Deserialize, Deserializer};
use serde::de::{self, Visitor, SeqAccess};
use serde::ser::{Serializer, SerializeStruct, Serialize};
use crate::polynomial::polynomial::{Polynomial, Term};



pub mod json_converters {
    use super::*;

    pub fn serialize_polynomial<S>(polynomial: &Polynomial, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Polynomial", 2)?;
        state.serialize_field("coefficient", &polynomial.coefficient)?;
        state.serialize_field("terms", &polynomial.terms)?;
        state.end()
    }

    pub fn deserialize_polynomial<'de, D>(deserializer: D) -> Result<Polynomial, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Coefficient, Terms }

        struct PolynomialVisitor;

        impl<'de> Visitor<'de> for PolynomialVisitor {
            type Value = Polynomial;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Polynomial")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Polynomial, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let terms = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                Ok(Polynomial::new(terms))
            }
        }

        const FIELDS: &[&str] = &["terms"];
        deserializer.deserialize_struct("Polynomial", FIELDS, PolynomialVisitor)
    }

    pub fn serialize_term<S>(term: &Term, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Term", 2)?;
        state.serialize_field("coefficient", &term.coefficient)?;
        state.serialize_field("exponent", &term.exponent)?;
        state.end()
    }

    pub fn deserialize_term<'de, D>(deserializer: D) -> Result<Term, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Coefficient, Exponent }

        struct TermVisitor;

        impl<'de> Visitor<'de> for TermVisitor {
            type Value = Term;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct Term")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Term, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let coefficient = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let exponent = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Term::new(coefficient, exponent))
            }
        }

        const FIELDS: &[&str] = &["coefficient", "exponent"];
        deserializer.deserialize_struct("Term", FIELDS, TermVisitor)
    }
}
