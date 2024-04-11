// src/integer_math/quadratic_residue.rs

use num::{BigInt, One};
use crate::factors::factor_pair::FactorPair;
use crate::integer_math::legendre::Legendre;
use crate::core::relations::Relation;

pub struct QuadraticResidue;

impl QuadraticResidue {
    // a^(p-1)/2 ≡ 1 (mod p)
    pub fn is_quadratic_residue(a: &BigInt, p: &BigInt) -> bool {
        let quotient = (p - 1) / 2;
        let mod_pow = a.modpow(&quotient, p);
        mod_pow == BigInt::one()
    }

    pub fn get_quadratic_character(rel: &Relation, quadratic_factor: &FactorPair) -> bool {
        let ab = &rel.a + &rel.b;
        let abp = (ab * &BigInt::from(quadratic_factor.p)).abs();
        let legendre_symbol = Legendre::symbol(&abp, &BigInt::from(quadratic_factor.r));
        legendre_symbol != 1
    }
}