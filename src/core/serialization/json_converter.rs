// src/core/serialization/json_converter.rs

use serde::{Deserialize, Deserializer};
use serde::de::{self, Visitor, SeqAccess};
use serde::ser::{Serializer, SerializeStruct};
use crate::core::serialization::types::{SerializablePolynomial, SerializableTerm};

pub mod json_converters {
    use super::*;

    pub fn serialize_polynomial<S>(polynomial: &SerializablePolynomial, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("SerializablePolynomial", 1)?;
        state.serialize_field("terms", &polynomial.terms)?;
        state.end()
    }

    pub fn deserialize_polynomial<'de, D>(deserializer: D) -> Result<SerializablePolynomial, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Terms,
        }

        struct PolynomialVisitor;

        impl<'de> Visitor<'de> for PolynomialVisitor {
            type Value = SerializablePolynomial;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct SerializablePolynomial")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<SerializablePolynomial, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let terms = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                Ok(SerializablePolynomial { terms })
            }
        }

        const FIELDS: &[&str] = &["terms"];
        deserializer.deserialize_struct("SerializablePolynomial", FIELDS, PolynomialVisitor)
    }

    pub fn serialize_term<S>(term: &SerializableTerm, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("SerializableTerm", 2)?;
        state.serialize_field("coefficient", &term.coefficient)?;
        state.serialize_field("exponent", &term.exponent)?;
        state.end()
    }

    pub fn deserialize_term<'de, D>(deserializer: D) -> Result<SerializableTerm, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Coefficient,
            Exponent,
        }

        struct TermVisitor;

        impl<'de> Visitor<'de> for TermVisitor {
            type Value = SerializableTerm;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct SerializableTerm")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<SerializableTerm, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let coefficient = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let exponent = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(SerializableTerm { coefficient, exponent })
            }
        }

        const FIELDS: &[&str] = &["coefficient", "exponent"];
        deserializer.deserialize_struct("SerializableTerm", FIELDS, TermVisitor)
    }
}