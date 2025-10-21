use gnfs::integer_math::prime_factory;
// src/main.rs
use log::{debug, info};
use env_logger::Env;
use gnfs::core::cpu_info::CPUInfo;
use gnfs::core::gnfs::GNFS;
use gnfs::core::cancellation_token::CancellationToken;
use gnfs::matrix::matrix_solve::MatrixSolve;
use gnfs::square_root::square_finder::SquareFinder;
use num::BigInt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::str::FromStr;

fn main() {
    // Parse command-line argument for the number to factor
    let args: Vec<String> = std::env::args().collect();
    let n = if args.len() > 1 {
        match BigInt::from_str(&args[1]) {
            Ok(num) => {
                info!("Factoring number from command line: {}", num);
                num
            },
            Err(e) => {
                eprintln!("Error parsing number '{}': {}", args[1], e);
                eprintln!("Usage: {} <number_to_factor>", args[0]);
                eprintln!("Example: {} 45113", args[0]);
                std::process::exit(1);
            }
        }
    } else {
        // Default number if no argument provided
        BigInt::from(45113)
    };
    // Configure Rayon thread pool based on environment variable or default to 25% of cores
    let env_var_value = std::env::var("GNFS_THREADS").ok();
    println!("DEBUG: GNFS_THREADS environment variable = {:?}", env_var_value);

    let num_threads = env_var_value
        .and_then(|s| {
            println!("DEBUG: Attempting to parse '{}' as usize", s);
            s.parse::<usize>().ok()
        })
        .unwrap_or_else(|| {
            let total_cores = num_cpus::get();
            let default_threads = (total_cores / 4).max(1); // 25% of cores, minimum 1
            println!("DEBUG: No valid GNFS_THREADS found, using default: {} threads ({}% of {} cores)",
                     default_threads, (default_threads * 100) / total_cores, total_cores);
            default_threads
        });

    println!("DEBUG: Configuring Rayon with {} threads", num_threads);

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .expect("Failed to configure thread pool");

    let actual_threads = rayon::current_num_threads();
    println!("GNFS using {} threads (Rayon reports: {}, total cores: {})", num_threads, actual_threads, num_cpus::get());

    // Initialize the logger
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "debug")
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
    let mut gnfs = create_or_load_gnfs(&n);

    // Log factor base information
    info!("Rational factor base size: {}", gnfs.prime_factor_base.rational_factor_base.len());
    info!("Algebraic factor base size: {}", gnfs.prime_factor_base.algebraic_factor_base.len());
    info!("Quadratic factor base size: {}", gnfs.prime_factor_base.quadratic_factor_base.len());
    info!("Rational factor pairs: {}", gnfs.rational_factor_pair_collection.len());
    info!("Algebraic factor pairs: {}", gnfs.algebraic_factor_pair_collection.len());
    info!("Quadratic factor pairs: {}", gnfs.quadratic_factor_pair_collection.len());

    // Debug: Show first 10 primes in each factor base
    let rat_fb: Vec<_> = gnfs.prime_factor_base.rational_factor_base.iter().take(15).collect();
    info!("Rational factor base (first 15): {:?}", rat_fb);
    let alg_fb: Vec<_> = gnfs.prime_factor_base.algebraic_factor_base.iter().take(20).collect();
    info!("Algebraic factor base (first 20): {:?}", alg_fb);

    // Test a specific relation manually
    let test_a = BigInt::from(1);
    let test_b = BigInt::from(3);
    let rational_norm = &test_a + &test_b * &gnfs.polynomial_base;
    let algebraic_norm = gnfs.current_polynomial.evaluate(&test_a);
    info!("Test relation (a=1, b=3): rational_norm={}, algebraic_norm={}", rational_norm, algebraic_norm);

    // Start the factorization process
    let cancel_token = CancellationToken::new();

    // Stage 1: Relation Sieving
    info!("");
    info!("========================================");
    info!("STAGE 1: RELATION SIEVING");
    info!("========================================");
    gnfs = find_relations(&cancel_token, gnfs, false);

    // Stage 2: Check if we have enough relations
    info!("");
    info!("========================================");
    info!("STAGE 2: CHECKING RELATIONS");
    info!("========================================");

    if gnfs.current_relations_progress.smooth_relations_counter >= gnfs.current_relations_progress.smooth_relations_target_quantity {
        info!("Enough smooth relations found! Proceeding to matrix construction and solving...");
        info!("Smooth relations found: {}", gnfs.current_relations_progress.smooth_relations_counter);
        info!("Target quantity: {}", gnfs.current_relations_progress.smooth_relations_target_quantity);
    } else {
        info!("Not enough relations found. Need more sieving.");
        info!("Smooth relations found: {}", gnfs.current_relations_progress.smooth_relations_counter);
        info!("Target quantity: {}", gnfs.current_relations_progress.smooth_relations_target_quantity);
        info!("Exiting - run again to continue sieving.");
        return;
    }

    // Stage 3: Matrix Construction and Solving
    info!("");
    info!("========================================");
    info!("STAGE 3: MATRIX SOLVING");
    info!("========================================");

    let cancel_token_arc = Arc::new(AtomicBool::new(cancel_token.is_cancellation_requested()));
    MatrixSolve::gaussian_solve(&cancel_token_arc, &mut gnfs);

    // Check if we found any free relations (solution sets)
    let free_relations_count = gnfs.current_relations_progress.relations.free_relations.len();
    info!("");
    info!("Matrix solving complete.");
    info!("Free relation sets found: {}", free_relations_count);

    if free_relations_count == 0 {
        info!("No solution sets found. Cannot proceed to square root extraction.");
        info!("You may need to:");
        info!("  1. Sieve for more relations");
        info!("  2. Adjust the polynomial parameters");
        info!("  3. Use a larger prime bound");
        return;
    }

    // Stage 4: Square Root Extraction
    info!("");
    info!("========================================");
    info!("STAGE 4: SQUARE ROOT EXTRACTION");
    info!("========================================");

    let factors_found = SquareFinder::solve(&cancel_token, &mut gnfs);

    // Stage 5: Report Final Results
    info!("");
    info!("========================================");
    info!("STAGE 5: FACTORIZATION RESULTS");
    info!("========================================");

    if factors_found && gnfs.is_factored() {
        info!("");
        info!("*****************************************");
        info!("*** FACTORIZATION SUCCESSFUL! ***");
        info!("*****************************************");
        info!("");
        if let Some(solution) = &gnfs.factorization {
            info!("N = {} = {} × {}", gnfs.n, solution.p, solution.q);
            info!("");
            info!("Verification: {} × {} = {}", solution.p, solution.q, &solution.p * &solution.q);

            if &solution.p * &solution.q == gnfs.n {
                info!("VERIFIED: Factors are correct!");
            } else {
                info!("ERROR: Factors do not multiply to N!");
            }
        }
        info!("");
        info!("*****************************************");
    } else {
        info!("");
        info!("Factorization did not succeed.");
        info!("The square root extraction did not find non-trivial factors.");
        info!("");
        info!("Possible reasons:");
        info!("  1. All {} solution sets led to trivial factors", free_relations_count);
        info!("  2. The algebraic square root computation failed");
        info!("  3. May need more relations or different solution sets");
        info!("");
        info!("You can try:");
        info!("  1. Running again (uses different random selection of solution sets)");
        info!("  2. Sieving for more relations to get more solution sets");
        info!("  3. Adjusting the factorization parameters");
    }

    info!("");
    info!("========================================");
    info!("GNFS FACTORIZATION COMPLETE");
    info!("========================================");
    info!("");
}

fn create_or_load_gnfs(n: &BigInt) -> GNFS {
    // Note: The save directory format is just the number itself (e.g., "45113")
    // not "gnfs_data_45113"
    let save_directory = format!("{}", n);

    // Check if checkpoint exists by looking for parameters.json
    let params_file = format!("{}/parameters.json", save_directory);

    if Path::new(&params_file).exists() {
        info!("========================================");
        info!("Found existing checkpoint at {}/", save_directory);
        info!("Resuming from saved state...");
        info!("========================================");

        // Load the checkpoint
        let gnfs = gnfs::core::serialization::load::load_checkpoint(&save_directory, n);

        info!("");
        info!("Resume Summary:");
        info!("  Position: A={}, B={}", gnfs.current_relations_progress.a, gnfs.current_relations_progress.b);
        info!("  Relations found: {} / {}",
              gnfs.current_relations_progress.smooth_relations_counter,
              gnfs.current_relations_progress.smooth_relations_target_quantity);
        info!("  Polynomial: {}", gnfs.current_polynomial);
        info!("  Polynomial base: {}", gnfs.polynomial_base);
        info!("========================================");
        info!("");

        gnfs
    } else {
        info!("No checkpoint found at {}/", save_directory);
        info!("Starting fresh factorization...");
        info!("");
        create_new_gnfs(n)
    }
}

fn create_new_gnfs(n: &BigInt) -> GNFS {
    info!("Creating a new GNFS instance...");
    let cancel_token = CancellationToken::new();
    let polynomial_base = BigInt::from(31);
    let poly_degree = 3;
    let prime_bound = BigInt::from(100); // Adjust the prime bound as needed
    let relation_quantity = 5; // Adjust the relation quantity as needed
    let relation_value_range = 50; // Adjust the relation value range as needed
    let created_new_data = true;

    info!("n: {}", n);
    info!("Polynomial Base: {}", polynomial_base.clone());
    info!("Polynomial Degree: {}", poly_degree);
    info!("Prime Bound: {}", prime_bound.clone());
    info!("Relation Target: {}", relation_quantity);
    info!("Relation Value: {}", relation_value_range);
    info!("GNFS: {}", created_new_data);

    let gnfs = GNFS::new(
        &cancel_token,
        n,
        &polynomial_base,
        poly_degree,
        &prime_bound,
        relation_quantity,
        relation_value_range,
        created_new_data,
    );

    // Save initial parameters
    info!("Saving initial parameters to {}", gnfs.save_locations.parameters_filepath);
    gnfs::core::serialization::save::parameters(&gnfs);
    info!("Parameters saved successfully");

    gnfs
}

fn find_relations(cancel_token: &CancellationToken, mut gnfs: GNFS, one_round: bool) -> GNFS {
    info!("Starting find_relations function...");
    info!("Current smooth relations: {}", gnfs.current_relations_progress.smooth_relations_counter);
    info!("Target smooth relations: {}", gnfs.current_relations_progress.smooth_relations_target_quantity);
    info!("Sieving for relations...");
    while !cancel_token.is_cancellation_requested() {
        if gnfs.current_relations_progress.smooth_relations_counter >= gnfs.current_relations_progress.smooth_relations_target_quantity {
            gnfs.current_relations_progress.increase_target_quantity(1);
            // Save progress when target quantity increases
            debug!("Saving progress after increasing target quantity...");
            gnfs::core::serialization::save::progress(&gnfs);
        }

        // Temporarily extract progress to avoid borrow checker issues
        let mut progress = std::mem::replace(
            &mut gnfs.current_relations_progress,
            gnfs::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress::default()
        );
        progress.generate_relations(&gnfs, cancel_token);
        gnfs.current_relations_progress = progress;

        // Save smooth relations that were just found
        debug!("Saving smooth relations...");
        gnfs::core::serialization::save::relations::smooth::append(&mut gnfs);

        // Save progress after each B increment
        debug!("Saving progress...");
        gnfs::core::serialization::save::progress(&gnfs);

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
        gnfs::core::serialization::save::progress(&gnfs);
        info!("Relations found: {}", gnfs.current_relations_progress.smooth_relations_counter);
    } else {
        info!("Sieving complete.");
        info!("Saving final progress...");
        gnfs::core::serialization::save::progress(&gnfs);
    }

    gnfs
}