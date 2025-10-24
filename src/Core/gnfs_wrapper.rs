// src/core/gnfs_wrapper.rs

use num::BigInt;
use log::info;
use crate::core::gnfs::GNFS;
use crate::core::gnfs_integer::{GnfsInteger, select_backend, BackendType};
use crate::core::cancellation_token::CancellationToken;
use crate::backends::*;
use crate::config::BufferConfig;

/// Runtime wrapper for GNFS that selects the optimal backend based on input size
///
/// This enum allows the GNFS algorithm to adaptively choose the most efficient
/// integer representation at runtime, unlocking significant memory and performance improvements:
/// - Native64Signed for 11-13 digit numbers (186x memory reduction, 50-100x speedup)
/// - Native128Signed for 14-19 digit numbers
/// - Fixed256 for 31-77 digit numbers
/// - Fixed512 for 78-154 digit numbers
/// - Arbitrary (BigInt) for 155+ digit numbers
pub enum GNFSWrapper {
    Native64Signed(GNFS<Native64Signed>),
    Native128Signed(GNFS<Native128Signed>),
    Fixed256(GNFS<Fixed256>),
    Fixed512(GNFS<Fixed512>),
    Arbitrary(GNFS<BigIntBackend>),
}

impl GNFSWrapper {
    /// Create a new GNFS instance with automatic backend selection
    pub fn new(
        cancel_token: &CancellationToken,
        n: &BigInt,
        polynomial_base: &BigInt,
        poly_degree: i32,
        prime_bound: &BigInt,
        relation_quantity: usize,
        relation_value_range: usize,
        created_new_data: bool,
    ) -> Self {
        Self::with_config(
            cancel_token,
            n,
            polynomial_base,
            poly_degree,
            prime_bound,
            relation_quantity,
            relation_value_range,
            created_new_data,
            BufferConfig::default(),
        )
    }

    /// Create a new GNFS instance with custom buffer configuration
    pub fn with_config(
        cancel_token: &CancellationToken,
        n: &BigInt,
        polynomial_base: &BigInt,
        poly_degree: i32,
        prime_bound: &BigInt,
        relation_quantity: usize,
        relation_value_range: usize,
        created_new_data: bool,
        buffer_config: BufferConfig,
    ) -> Self {
        // Select backend based on number size and polynomial degree
        let backend_type = select_backend(n, poly_degree.abs() as usize);

        info!("Selected backend: {} for {}-digit number (n = {})",
              backend_type.name(), n.to_string().len(), n);

        match backend_type {
            BackendType::Native64Signed => {
                info!("Using {} backend: {} bytes per value",
                      backend_type.name(),
                      std::mem::size_of::<i64>());

                let gnfs = GNFS::<Native64Signed>::with_config(
                    cancel_token,
                    n,
                    polynomial_base,
                    poly_degree,
                    prime_bound,
                    relation_quantity,
                    relation_value_range,
                    created_new_data,
                    buffer_config,
                );
                GNFSWrapper::Native64Signed(gnfs)
            },

            BackendType::Native128Signed => {
                info!("Using {} backend: {} bytes per value",
                      backend_type.name(),
                      std::mem::size_of::<i128>());

                let gnfs = GNFS::<Native128Signed>::with_config(
                    cancel_token,
                    n,
                    polynomial_base,
                    poly_degree,
                    prime_bound,
                    relation_quantity,
                    relation_value_range,
                    created_new_data,
                    buffer_config,
                );
                GNFSWrapper::Native128Signed(gnfs)
            },

            BackendType::Fixed256 => {
                info!("Using {} backend: {} bytes per value",
                      backend_type.name(),
                      std::mem::size_of::<crypto_bigint::U256>());

                let gnfs = GNFS::<Fixed256>::with_config(
                    cancel_token,
                    n,
                    polynomial_base,
                    poly_degree,
                    prime_bound,
                    relation_quantity,
                    relation_value_range,
                    created_new_data,
                    buffer_config,
                );
                GNFSWrapper::Fixed256(gnfs)
            },

            BackendType::Fixed512 => {
                info!("Using {} backend: {} bytes per value",
                      backend_type.name(),
                      std::mem::size_of::<crypto_bigint::U512>());

                let gnfs = GNFS::<Fixed512>::with_config(
                    cancel_token,
                    n,
                    polynomial_base,
                    poly_degree,
                    prime_bound,
                    relation_quantity,
                    relation_value_range,
                    created_new_data,
                    buffer_config,
                );
                GNFSWrapper::Fixed512(gnfs)
            },

            BackendType::Arbitrary => {
                info!("Using {} backend: dynamic allocation",
                      backend_type.name());

                let gnfs = GNFS::<BigIntBackend>::with_config(
                    cancel_token,
                    n,
                    polynomial_base,
                    poly_degree,
                    prime_bound,
                    relation_quantity,
                    relation_value_range,
                    created_new_data,
                    buffer_config,
                );
                GNFSWrapper::Arbitrary(gnfs)
            },
        }
    }

    /// Get the number being factored
    pub fn n(&self) -> &BigInt {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => &gnfs.n,
            GNFSWrapper::Native128Signed(gnfs) => &gnfs.n,
            GNFSWrapper::Fixed256(gnfs) => &gnfs.n,
            GNFSWrapper::Fixed512(gnfs) => &gnfs.n,
            GNFSWrapper::Arbitrary(gnfs) => &gnfs.n,
        }
    }

    /// Get polynomial base
    pub fn polynomial_base(&self) -> &BigInt {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => &gnfs.polynomial_base,
            GNFSWrapper::Native128Signed(gnfs) => &gnfs.polynomial_base,
            GNFSWrapper::Fixed256(gnfs) => &gnfs.polynomial_base,
            GNFSWrapper::Fixed512(gnfs) => &gnfs.polynomial_base,
            GNFSWrapper::Arbitrary(gnfs) => &gnfs.polynomial_base,
        }
    }

    /// Get polynomial degree
    pub fn polynomial_degree(&self) -> usize {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => gnfs.polynomial_degree,
            GNFSWrapper::Native128Signed(gnfs) => gnfs.polynomial_degree,
            GNFSWrapper::Fixed256(gnfs) => gnfs.polynomial_degree,
            GNFSWrapper::Fixed512(gnfs) => gnfs.polynomial_degree,
            GNFSWrapper::Arbitrary(gnfs) => gnfs.polynomial_degree,
        }
    }

    /// Get current polynomial as string (for display)
    pub fn current_polynomial_display(&self) -> String {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => gnfs.current_polynomial.to_string(),
            GNFSWrapper::Native128Signed(gnfs) => gnfs.current_polynomial.to_string(),
            GNFSWrapper::Fixed256(gnfs) => gnfs.current_polynomial.to_string(),
            GNFSWrapper::Fixed512(gnfs) => gnfs.current_polynomial.to_string(),
            GNFSWrapper::Arbitrary(gnfs) => gnfs.current_polynomial.to_string(),
        }
    }

    /// Check if factorization is complete
    pub fn is_factored(&self) -> bool {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => gnfs.is_factored(),
            GNFSWrapper::Native128Signed(gnfs) => gnfs.is_factored(),
            GNFSWrapper::Fixed256(gnfs) => gnfs.is_factored(),
            GNFSWrapper::Fixed512(gnfs) => gnfs.is_factored(),
            GNFSWrapper::Arbitrary(gnfs) => gnfs.is_factored(),
        }
    }

    /// Get the name of the backend being used
    pub fn backend_name(&self) -> &'static str {
        match self {
            GNFSWrapper::Native64Signed(_) => "Native64Signed",
            GNFSWrapper::Native128Signed(_) => "Native128Signed",
            GNFSWrapper::Fixed256(_) => "Fixed256",
            GNFSWrapper::Fixed512(_) => "Fixed512",
            GNFSWrapper::Arbitrary(_) => "Arbitrary",
        }
    }

    /// Get factor base information
    pub fn get_factor_base_info(&self) -> (usize, usize, usize) {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => (
                gnfs.prime_factor_base.rational_factor_base.len(),
                gnfs.prime_factor_base.algebraic_factor_base.len(),
                gnfs.prime_factor_base.quadratic_factor_base.len(),
            ),
            GNFSWrapper::Native128Signed(gnfs) => (
                gnfs.prime_factor_base.rational_factor_base.len(),
                gnfs.prime_factor_base.algebraic_factor_base.len(),
                gnfs.prime_factor_base.quadratic_factor_base.len(),
            ),
            GNFSWrapper::Fixed256(gnfs) => (
                gnfs.prime_factor_base.rational_factor_base.len(),
                gnfs.prime_factor_base.algebraic_factor_base.len(),
                gnfs.prime_factor_base.quadratic_factor_base.len(),
            ),
            GNFSWrapper::Fixed512(gnfs) => (
                gnfs.prime_factor_base.rational_factor_base.len(),
                gnfs.prime_factor_base.algebraic_factor_base.len(),
                gnfs.prime_factor_base.quadratic_factor_base.len(),
            ),
            GNFSWrapper::Arbitrary(gnfs) => (
                gnfs.prime_factor_base.rational_factor_base.len(),
                gnfs.prime_factor_base.algebraic_factor_base.len(),
                gnfs.prime_factor_base.quadratic_factor_base.len(),
            ),
        }
    }

    /// Get factor pair collection information
    pub fn get_factor_pair_info(&self) -> (usize, usize, usize) {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => (
                gnfs.rational_factor_pair_collection.len(),
                gnfs.algebraic_factor_pair_collection.len(),
                gnfs.quadratic_factor_pair_collection.len(),
            ),
            GNFSWrapper::Native128Signed(gnfs) => (
                gnfs.rational_factor_pair_collection.len(),
                gnfs.algebraic_factor_pair_collection.len(),
                gnfs.quadratic_factor_pair_collection.len(),
            ),
            GNFSWrapper::Fixed256(gnfs) => (
                gnfs.rational_factor_pair_collection.len(),
                gnfs.algebraic_factor_pair_collection.len(),
                gnfs.quadratic_factor_pair_collection.len(),
            ),
            GNFSWrapper::Fixed512(gnfs) => (
                gnfs.rational_factor_pair_collection.len(),
                gnfs.algebraic_factor_pair_collection.len(),
                gnfs.quadratic_factor_pair_collection.len(),
            ),
            GNFSWrapper::Arbitrary(gnfs) => (
                gnfs.rational_factor_pair_collection.len(),
                gnfs.algebraic_factor_pair_collection.len(),
                gnfs.quadratic_factor_pair_collection.len(),
            ),
        }
    }

    /// Get relations progress information
    pub fn get_relations_info(&self) -> (usize, usize) {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => (
                gnfs.current_relations_progress.smooth_relations_counter,
                gnfs.current_relations_progress.smooth_relations_target_quantity,
            ),
            GNFSWrapper::Native128Signed(gnfs) => (
                gnfs.current_relations_progress.smooth_relations_counter,
                gnfs.current_relations_progress.smooth_relations_target_quantity,
            ),
            GNFSWrapper::Fixed256(gnfs) => (
                gnfs.current_relations_progress.smooth_relations_counter,
                gnfs.current_relations_progress.smooth_relations_target_quantity,
            ),
            GNFSWrapper::Fixed512(gnfs) => (
                gnfs.current_relations_progress.smooth_relations_counter,
                gnfs.current_relations_progress.smooth_relations_target_quantity,
            ),
            GNFSWrapper::Arbitrary(gnfs) => (
                gnfs.current_relations_progress.smooth_relations_counter,
                gnfs.current_relations_progress.smooth_relations_target_quantity,
            ),
        }
    }

    /// Get save directory path
    pub fn save_directory(&self) -> &str {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => &gnfs.save_locations.save_directory,
            GNFSWrapper::Native128Signed(gnfs) => &gnfs.save_locations.save_directory,
            GNFSWrapper::Fixed256(gnfs) => &gnfs.save_locations.save_directory,
            GNFSWrapper::Fixed512(gnfs) => &gnfs.save_locations.save_directory,
            GNFSWrapper::Arbitrary(gnfs) => &gnfs.save_locations.save_directory,
        }
    }

    /// Get parameters filepath
    pub fn parameters_filepath(&self) -> &str {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => &gnfs.save_locations.parameters_filepath,
            GNFSWrapper::Native128Signed(gnfs) => &gnfs.save_locations.parameters_filepath,
            GNFSWrapper::Fixed256(gnfs) => &gnfs.save_locations.parameters_filepath,
            GNFSWrapper::Fixed512(gnfs) => &gnfs.save_locations.parameters_filepath,
            GNFSWrapper::Arbitrary(gnfs) => &gnfs.save_locations.parameters_filepath,
        }
    }

    /// Dispatch relation sieving to the appropriate backend
    pub fn find_relations(&mut self, cancel_token: &CancellationToken, one_round: bool) {
        use log::debug;

        info!("Starting find_relations with {} backend...", self.backend_name());

        match self {
            GNFSWrapper::Native64Signed(gnfs) => {
                Self::find_relations_impl(gnfs, cancel_token, one_round);
            },
            GNFSWrapper::Native128Signed(gnfs) => {
                Self::find_relations_impl(gnfs, cancel_token, one_round);
            },
            GNFSWrapper::Fixed256(gnfs) => {
                Self::find_relations_impl(gnfs, cancel_token, one_round);
            },
            GNFSWrapper::Fixed512(gnfs) => {
                Self::find_relations_impl(gnfs, cancel_token, one_round);
            },
            GNFSWrapper::Arbitrary(gnfs) => {
                Self::find_relations_impl(gnfs, cancel_token, one_round);
            },
        }
    }

    /// Generic implementation of find_relations that works with any backend
    fn find_relations_impl<T: GnfsInteger>(
        gnfs: &mut GNFS<T>,
        cancel_token: &CancellationToken,
        one_round: bool
    ) {
        use log::debug;

        info!("Current smooth relations: {}", gnfs.current_relations_progress.smooth_relations_counter);
        info!("Target smooth relations: {}", gnfs.current_relations_progress.smooth_relations_target_quantity);
        info!("Sieving for relations...");

        while !cancel_token.is_cancellation_requested() {
            if gnfs.current_relations_progress.smooth_relations_counter >= gnfs.current_relations_progress.smooth_relations_target_quantity {
                gnfs.current_relations_progress.increase_target_quantity(1);
                debug!("Saving progress after increasing target quantity...");
                crate::core::serialization::save::progress(gnfs);
            }

            // Temporarily extract progress to avoid borrow checker issues
            let mut progress = std::mem::replace(
                &mut gnfs.current_relations_progress,
                crate::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress::default()
            );
            progress.generate_relations(gnfs, cancel_token);
            gnfs.current_relations_progress = progress;

            // Save smooth relations that were just found
            debug!("Saving smooth relations...");
            crate::core::serialization::save::relations::smooth::append(gnfs);

            // Save progress after each B increment
            debug!("Saving progress...");
            crate::core::serialization::save::progress(gnfs);

            debug!("");
            debug!("Sieving progress saved at:");
            debug!(" A = {}", gnfs.current_relations_progress.a);
            debug!(" B = {}", gnfs.current_relations_progress.b);
            debug!("");

            if one_round {
                break;
            }

            if gnfs.current_relations_progress.smooth_relations_counter >= gnfs.current_relations_progress.smooth_relations_target_quantity {
                break;
            }
        }

        if cancel_token.is_cancellation_requested() {
            info!("Sieving cancelled.");
            info!("Saving progress...");
            crate::core::serialization::save::progress(gnfs);
            info!("Relations found: {}", gnfs.current_relations_progress.smooth_relations_counter);
        } else {
            info!("Sieving complete.");
            info!("Saving final progress...");
            crate::core::serialization::save::progress(gnfs);
        }
    }

    /// Dispatch matrix solving to the appropriate backend
    pub fn matrix_solve(&mut self, cancel_token_arc: &std::sync::Arc<std::sync::atomic::AtomicBool>) {
        use crate::matrix::matrix_solve::MatrixSolve;

        match self {
            GNFSWrapper::Native64Signed(gnfs) => MatrixSolve::gaussian_solve(cancel_token_arc, gnfs),
            GNFSWrapper::Native128Signed(gnfs) => MatrixSolve::gaussian_solve(cancel_token_arc, gnfs),
            GNFSWrapper::Fixed256(gnfs) => MatrixSolve::gaussian_solve(cancel_token_arc, gnfs),
            GNFSWrapper::Fixed512(gnfs) => MatrixSolve::gaussian_solve(cancel_token_arc, gnfs),
            GNFSWrapper::Arbitrary(gnfs) => MatrixSolve::gaussian_solve(cancel_token_arc, gnfs),
        }
    }

    /// Dispatch square root finding to the appropriate backend
    pub fn square_finder_solve(&mut self, cancel_token: &CancellationToken) -> bool {
        use crate::square_root::square_finder::SquareFinder;

        match self {
            GNFSWrapper::Native64Signed(gnfs) => SquareFinder::solve(cancel_token, gnfs),
            GNFSWrapper::Native128Signed(gnfs) => SquareFinder::solve(cancel_token, gnfs),
            GNFSWrapper::Fixed256(gnfs) => SquareFinder::solve(cancel_token, gnfs),
            GNFSWrapper::Fixed512(gnfs) => SquareFinder::solve(cancel_token, gnfs),
            GNFSWrapper::Arbitrary(gnfs) => SquareFinder::solve(cancel_token, gnfs),
        }
    }

    /// Get the factorization result if available
    pub fn get_solution(&self) -> Option<&crate::core::solution::Solution> {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => gnfs.factorization.as_ref(),
            GNFSWrapper::Native128Signed(gnfs) => gnfs.factorization.as_ref(),
            GNFSWrapper::Fixed256(gnfs) => gnfs.factorization.as_ref(),
            GNFSWrapper::Fixed512(gnfs) => gnfs.factorization.as_ref(),
            GNFSWrapper::Arbitrary(gnfs) => gnfs.factorization.as_ref(),
        }
    }

    /// Get number of free relations found
    pub fn free_relations_count(&self) -> usize {
        match self {
            GNFSWrapper::Native64Signed(gnfs) => gnfs.current_relations_progress.relations.free_relations.len(),
            GNFSWrapper::Native128Signed(gnfs) => gnfs.current_relations_progress.relations.free_relations.len(),
            GNFSWrapper::Fixed256(gnfs) => gnfs.current_relations_progress.relations.free_relations.len(),
            GNFSWrapper::Fixed512(gnfs) => gnfs.current_relations_progress.relations.free_relations.len(),
            GNFSWrapper::Arbitrary(gnfs) => gnfs.current_relations_progress.relations.free_relations.len(),
        }
    }
}
