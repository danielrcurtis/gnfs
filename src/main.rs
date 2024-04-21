use gnfs::integer_math::prime_factory;
// src/main.rs
use log::{debug, info};
use env_logger::Env;
use gnfs::core::cpu_info::CPUInfo;
use gnfs::core::gnfs::GNFS;
use gnfs::core::cancellation_token::CancellationToken;
use num::BigInt;
use std::path::Path;

fn main() {
    // Initialize the logger
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "info")
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::Builder::from_env(env).init();

    // Fetching cache information
    let l1_cache_line_size = CPUInfo::l1_cache_line_size().unwrap_or(0);
    let l1_cache_size = CPUInfo::l1_cache_size().unwrap_or(0);
    let l2_cache_line_size = CPUInfo::l2_cache_line_size().unwrap_or(0);
    let l2_cache_size = CPUInfo::l2_cache_size().unwrap_or(0);
    let l3_cache_line_size = CPUInfo::l3_cache_line_size().unwrap_or(0);
    let l3_cache_size = CPUInfo::l3_cache_size().unwrap_or(0);

    // Logging cache sizes and line sizes
    info!("L1 cache size: {} bytes", l1_cache_size);
    info!("L1 cache line size: {} bytes", l1_cache_line_size);
    info!("L2 cache size: {} bytes", l2_cache_size);
    info!("L2 cache line size: {} bytes", l2_cache_line_size);
    info!("L3 cache size: {} bytes", l3_cache_size);
    info!("L3 cache line size: {} bytes", l3_cache_line_size);

    let prime_factory = prime_factory::PrimeFactory::new();
    let is_prime = prime_factory.is_prime(&BigInt::from(5));
    info!("Is 5 prime? {}", is_prime);
    // Create or load GNFS instance
    let n = BigInt::from(45113); // RNumber to test.
    let mut gnfs = create_or_load_gnfs(&n);

    // Start the factorization process
    let cancel_token = CancellationToken::new();
    gnfs = find_relations(&cancel_token, gnfs, false);

}

fn create_or_load_gnfs(n: &BigInt) -> GNFS {
    let save_directory = format!("gnfs_data_{}", n);
    let save_path = Path::new(&save_directory);

    if save_path.exists() {
        // Load existing GNFS instance
        info!("Loading existing GNFS instance...");
        // TODO: Implement loading GNFS instance from file
        // For now, create a new instance
        create_new_gnfs(n)
    } else {
        // Create a new GNFS instance
        create_new_gnfs(n)
    }
}

fn create_new_gnfs(n: &BigInt) -> GNFS {
    info!("Creating a new GNFS instance...");
    let cancel_token = CancellationToken::new();
    let polynomial_base = BigInt::from(31);
    let poly_degree = 3;
    let prime_bound = BigInt::from(100); // Adjust the prime bound as needed
    let relation_quantity = 1; // Adjust the relation quantity as needed
    let relation_value_range = 1000; // Adjust the relation value range as needed
    let created_new_data = true;

    info!("n: {}", n);
    info!("Polynomial Base: {}", polynomial_base.clone());
    info!("Polynomial Degree: {}", poly_degree);
    info!("Prime Bound: {}", prime_bound.clone());
    info!("Relation Target: {}", relation_quantity);
    info!("Relation Value: {}", relation_value_range);
    info!("GNFS: {}", created_new_data);

    GNFS::new(
        &cancel_token,
        n,
        &polynomial_base,
        poly_degree,
        &prime_bound,
        relation_quantity,
        relation_value_range,
        created_new_data,
    )
}

fn find_relations(cancel_token: &CancellationToken, mut gnfs: GNFS, one_round: bool) -> GNFS {
    info!("Sieving for relations...");
    while !cancel_token.is_cancellation_requested() {
        if gnfs.current_relations_progress.smooth_relations_counter >= gnfs.current_relations_progress.smooth_relations_target_quantity {
            gnfs.current_relations_progress.increase_target_quantity(1);
        }

        gnfs.current_relations_progress.generate_relations(cancel_token);

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
        info!("Relations found: {}", gnfs.current_relations_progress.smooth_relations_counter);
    } else {
        info!("Sieving complete.");
    }

    gnfs
}