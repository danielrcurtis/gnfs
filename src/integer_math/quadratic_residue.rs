// src/integer_math/quadratic_residue.rs

use num::{BigInt, One, Signed};
use crate::factor::factor_pair::FactorPair;
use crate::integer_math::legendre::Legendre;
use crate::relation_sieve::relation::Relation;

pub struct QuadraticResidue;

impl QuadraticResidue {
    // a^(p-1)/2 ≡ 1 (mod p)
    pub fn is_quadratic_residue(a: &BigInt, p: &BigInt) -> bool {
        let quotient = (p - 1) / 2;
        let mod_pow = a.modpow(&quotient, p);
        mod_pow == BigInt::one()
    }

    pub fn get_quadratic_character<T: crate::core::gnfs_integer::GnfsInteger>(rel: &crate::relation_sieve::relation::Relation<T>, quadratic_factor: &FactorPair) -> bool {
        let a_big = rel.a.to_bigint();
        let b_big = rel.b.to_bigint();
        let ab = &a_big + &b_big;
        let abp = (ab * &BigInt::from(quadratic_factor.p)).abs();
        let legendre_symbol = Legendre::symbol(&abp, &BigInt::from(quadratic_factor.r));
        legendre_symbol != 1
    }
}