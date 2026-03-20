// src/core/factor_base.rs

use num::BigInt;

#[derive(Debug, Clone, Default)]
pub struct FactorBase {
    // #[serde(rename = "RationalFactorBaseMax")]
    pub rational_factor_base_max: BigInt,

    // #[serde(rename = "AlgebraicFactorBaseMax")]
    pub algebraic_factor_base_max: BigInt,

    // #[serde(rename = "QuadraticFactorBaseMin")]
    pub quadratic_factor_base_min: BigInt,

    // #[serde(rename = "QuadraticFactorBaseMax")]
    pub quadratic_factor_base_max: BigInt,

    // #[serde(rename = "QuadraticBaseCount")]
    pub quadratic_base_count: i32,

    // #[serde(skip)]
    pub rational_factor_base: Vec<BigInt>,

    // #[serde(skip)]
    pub algebraic_factor_base: Vec<BigInt>,

    // #[serde(skip)]
    pub quadratic_factor_base: Vec<BigInt>,
}

