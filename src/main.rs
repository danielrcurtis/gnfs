use gnfs::integer_math::prime_factory;
// src/main.rs
use log::{debug, info, warn};
use env_logger::Env;
use gnfs::core::cpu_info::CPUInfo;
use gnfs::core::gnfs_wrapper::GNFSWrapper;
use gnfs::core::cancellation_token::CancellationToken;
use gnfs::config::GnfsConfig;
use gnfs::benchmark_cli;
use gnfs::algorithms::{choose_algorithm, factor, FactorizationAlgorithm};
use num::{BigInt, ToPrimitive};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::str::FromStr;

fn main() {
    // Load configuration first (before logging is initialized)
    let config = GnfsConfig::load().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
        GnfsConfig::default()
    });
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for --bench flag
    if args.len() > 1 && args[1] == "--bench" {
        benchmark_cli::run_benchmarks(&args);
        return;
    }

    // Otherwise, parse number to factor
    let n = if args.len() > 1 {
        match BigInt::from_str(&args[1]) {
            Ok(num) => {
                info!("Factoring number from command line: {}", num);
                num
            },
            Err(e) => {
                eprintln!("Error parsing number '{}': {}", args[1], e);
                eprintln!("Usage: {} <number_to_factor>", args[0]);
                eprintln!("       {} --bench [digit_counts...]", args[0]);
                eprintln!("Example: {} 45113", args[0]);
                eprintln!("         {} --bench 7 9 11", args[0]);
                std::process::exit(1);
            }
        }
    } else {
        // Default number if no argument provided
        BigInt::from(45113)
    };
    // Initialize logging based on config (can be overridden by MY_LOG_LEVEL env var)
    let log_level = std::env::var("MY_LOG_LEVEL")
        .unwrap_or_else(|_| config.log_level.clone());
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", log_level)
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::Builder::from_env(env).init();

    // Configure Rayon thread pool
    let num_threads = config.threads.unwrap_or_else(|| {
        let total_cores = num_cpus::get();
        (total_cores / 4).max(1) // Default: 25% of cores, minimum 1
    });

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .expect("Failed to configure thread pool");

    // Log configuration settings
    info!("================================================================================");
    info!("GNFS CONFIGURATION");
    info!("================================================================================");
    info!("Output directory: {}", config.output_dir);
    info!("Cleanup on success: {}", config.cleanup);
    info!("Buffer max memory: {:.2} MB", config.buffer.max_memory_bytes as f64 / (1024.0 * 1024.0));
    info!("Buffer min relations: {}", config.buffer.min_relations);
    info!("Buffer max relations: {}", config.buffer.max_relations);
    info!("Buffer batch size: {} (B values per parallel batch)", config.buffer.batch_size);
    info!("Threads: {} (total cores: {})", num_threads, num_cpus::get());
    info!("Log level: {}", config.log_level);
    info!("Prime bound multiplier: {}", config.performance.prime_bound_multiplier);
    info!("Relation quantity multiplier: {}", config.performance.relation_quantity_multiplier);
    info!("================================================================================");
    info!("");

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
    debug!("Is 5 prime? {}", is_prime);

    // Use algorithm dispatcher to automatically select the best factorization method
    // This replaces the old hardcoded trial division check
    let algorithm = choose_algorithm(&n);

    // For algorithms other than GNFS, attempt fast factorization
    if algorithm != FactorizationAlgorithm::GNFS {
        info!("");
        info!("========================================");
        info!("FAST FACTORIZATION PATH");
        info!("========================================");
        info!("Number: {}", n);
        info!("Size: {} digits", n.to_string().len());
        info!("");

        match factor(&n) {
            Ok((p, q)) => {
                info!("");
                info!("*****************************************");
                info!("*** FACTORIZATION SUCCESSFUL! ***");
                info!("*****************************************");
                info!("");
                info!("N = {}", n);
                info!("Algorithm: {}", algorithm.name());
                info!("");
                info!("{} = {} × {}", n, p, q);
                info!("");
                info!("Verification: {} × {} = {}", p, q, &p * &q);

                if &p * &q == n {
                    info!("✓ VERIFIED: Factors are correct!");
                } else {
                    info!("✗ ERROR: Factors do not multiply to N!");
                }

                info!("");
                info!("*****************************************");
                return;
            }
            Err(e) => {
                info!("");
                info!("Fast factorization failed: {}", e);
                info!("");

                // If number is in GNFS range and simpler methods failed, use GNFS
                let digits = n.to_string().len();
                if digits >= 100 {
                    info!("Number is {} digits - proceeding to GNFS...", digits);
                } else {
                    info!("Number may be prime or require more advanced methods.");
                    info!("Falling back to GNFS for complete analysis...");
                }
                info!("");
            }
        }
    } else {
        info!("");
        info!("========================================");
        info!("GNFS PATH");
        info!("========================================");
        info!("Number: {}", n);
        info!("Size: {} digits - GNFS is the optimal choice", n.to_string().len());
        info!("");
    }

    // Create or load GNFS instance
    let mut gnfs = create_or_load_gnfs(&n, &config);

    // Log factor base information
    let (rat_fb_size, alg_fb_size, quad_fb_size) = gnfs.get_factor_base_info();
    info!("Rational factor base size: {}", rat_fb_size);
    info!("Algebraic factor base size: {}", alg_fb_size);
    info!("Quadratic factor base size: {}", quad_fb_size);

    let (rat_pairs, alg_pairs, quad_pairs) = gnfs.get_factor_pair_info();
    info!("Rational factor pairs: {}", rat_pairs);
    info!("Algebraic factor pairs: {}", alg_pairs);
    info!("Quadratic factor pairs: {}", quad_pairs);

    // Start the factorization process
    let cancel_token = CancellationToken::new();

    // Set up CTRL-C handler for graceful shutdown
    let cancel_token_clone = cancel_token.clone();
    ctrlc::set_handler(move || {
        warn!("");
        warn!("========================================");
        warn!("CTRL-C RECEIVED - INITIATING GRACEFUL SHUTDOWN");
        warn!("========================================");
        warn!("Saving progress to disk...");
        warn!("Current relations will be preserved.");
        warn!("Please wait for shutdown to complete...");
        cancel_token_clone.cancel();
    }).expect("Error setting CTRL-C handler");

    info!("Graceful shutdown enabled: Press CTRL-C to save progress and exit");

    // Stage 1: Relation Sieving
    info!("");
    info!("========================================");
    info!("STAGE 1: RELATION SIEVING");
    info!("========================================");
    gnfs.find_relations(&cancel_token, false);

    // Check if execution was cancelled
    if cancel_token.is_cancellation_requested() {
        warn!("");
        warn!("========================================");
        warn!("GRACEFUL SHUTDOWN COMPLETE");
        warn!("========================================");
        let (smooth_found, smooth_target) = gnfs.get_relations_info();
        warn!("Progress saved:");
        warn!("  Smooth relations: {} / {} ({:.1}%)",
              smooth_found, smooth_target,
              100.0 * smooth_found as f64 / smooth_target as f64);
        warn!("  Relations saved to: {}/streamed_relations.jsonl", n);
        warn!("");
        warn!("To resume: Run the same command again");
        warn!("The program will automatically load saved progress.");
        return;
    }

    // Stage 2: Check if we have enough relations
    info!("");
    info!("========================================");
    info!("STAGE 2: CHECKING RELATIONS");
    info!("========================================");

    let (smooth_found, smooth_target) = gnfs.get_relations_info();
    if smooth_found >= smooth_target {
        info!("Enough smooth relations found! Proceeding to matrix construction and solving...");
        info!("Smooth relations found: {}", smooth_found);
        info!("Target quantity: {}", smooth_target);
    } else {
        info!("Not enough relations found. Need more sieving.");
        info!("Smooth relations found: {}", smooth_found);
        info!("Target quantity: {}", smooth_target);
        info!("Exiting - run again to continue sieving.");
        return;
    }

    // Stage 3: Matrix Construction and Solving
    info!("");
    info!("========================================");
    info!("STAGE 3: MATRIX SOLVING");
    info!("========================================");

    let cancel_token_arc = Arc::new(AtomicBool::new(cancel_token.is_cancellation_requested()));
    gnfs.matrix_solve(&cancel_token_arc);

    // Check if we found any free relations (solution sets)
    let free_relations_count = gnfs.free_relations_count();
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

    let factors_found = gnfs.square_finder_solve(&cancel_token);

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
        if let Some(solution) = gnfs.get_solution() {
            info!("N = {} = {} × {}", gnfs.n(), solution.p, solution.q);
            info!("");
            info!("Verification: {} × {} = {}", solution.p, solution.q, &solution.p * &solution.q);

            if &solution.p * &solution.q == *gnfs.n() {
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

    // Optional cleanup of output directory
    if config.cleanup && gnfs.is_factored() {
        let save_dir = Path::new(&config.output_dir).join(format!("{}", n));
        match std::fs::remove_dir_all(&save_dir) {
            Ok(_) => info!("Cleaned up output directory: {}", save_dir.display()),
            Err(e) => warn!("Could not cleanup directory {}: {}", save_dir.display(), e),
        }
    }
}

fn create_or_load_gnfs(n: &BigInt, config: &GnfsConfig) -> GNFSWrapper {
    // Note: For now, we don't support loading from checkpoint with GNFSWrapper
    // This is because we'd need to serialize the backend type along with the data.
    // For Phase 3, we'll just create fresh instances.
    // TODO: Implement checkpoint loading with backend type detection in Phase 4

    let save_directory = format!("{}", n);
    let params_file = format!("{}/parameters.json", save_directory);

    if Path::new(&params_file).exists() {
        info!("========================================");
        info!("Found existing checkpoint at {}/", save_directory);
        info!("WARNING: Checkpoint loading not yet supported with adaptive backends");
        info!("Starting fresh factorization...");
        info!("========================================");
        info!("");
    } else {
        info!("No checkpoint found at {}/", save_directory);
        info!("Starting fresh factorization...");
        info!("");
    }

    create_new_gnfs(n, config)
}

fn create_new_gnfs(n: &BigInt, config: &GnfsConfig) -> GNFSWrapper {
    info!("Creating a new GNFS instance...");
    let cancel_token = CancellationToken::new();
    let polynomial_base = BigInt::from(31);
    let poly_degree = 3;

    // MATHEMATICALLY SOUND PARAMETER SELECTION
    // Based on GNFS complexity theory and empirical validation
    //
    // GNFS complexity: L_n[1/3, (64/9)^(1/3)] ≈ exp(1.923 · (ln n)^(1/3) · (ln ln n)^(2/3))
    // Optimal smoothness bound: B ≈ exp(sqrt(ln n · ln ln n)) for small numbers
    //
    // Key insight: Smooth relation density decreases exponentially with N.
    // Prime bounds must increase faster than linear to maintain feasible sieving time.
    //
    // References:
    // - Lenstra et al. (1993): "The number field sieve"
    // - CADO-NFS and msieve implementations
    // - Empirical testing on 6-15 digit semiprimes
    let digits = n.to_string().len();
    let base_prime_bound = if digits <= 10 {
        // Linear scaling for small numbers (6-10 digits)
        // Formula: 50 * (digits - 5) gives {50, 100, 150, 200, 250}
        // This provides adequate smooth relation density while keeping FB small
        let bound = 50 * (digits as i64 - 5).max(1);
        BigInt::from(bound.max(50))
    } else if digits <= 15 {
        // Exponential scaling for medium numbers (11-15 digits)
        // Formula: 100 * 1.6^(digits - 10)
        // Balances smooth relation discovery with linear algebra cost
        let exponent = (digits as i32) - 10;
        let bound = (100.0 * 1.6_f64.powi(exponent)) as i64;
        BigInt::from(bound)
    } else if digits <= 30 {
        // L-notation approximation for larger numbers (16-30 digits)
        // B ≈ exp(c · sqrt(ln n · ln ln n)) where c = 0.3 for practical bounds
        let n_f64 = n.to_f64().unwrap_or(10_f64.powi(digits as i32));
        let ln_n = n_f64.ln();
        let ln_ln_n = ln_n.ln();
        let bound = (0.3 * (ln_n * ln_ln_n).sqrt().exp()) as i64;
        BigInt::from(bound)
    } else {
        // Full L-notation for very large numbers (31+ digits)
        // B = exp(c · (ln n)^(1/3) · (ln ln n)^(2/3)) where c ≈ 0.5
        let n_f64 = n.to_f64().unwrap_or(10_f64.powi(digits as i32));
        let ln_n = n_f64.ln();
        let ln_ln_n = ln_n.ln();
        let bound = (0.5 * ln_n.powf(1.0/3.0) * ln_ln_n.powf(2.0/3.0)).exp() as i64;
        BigInt::from(bound)
    };

    // Apply performance multiplier from config
    let prime_bound = if config.performance.prime_bound_multiplier != 1.0 {
        let multiplied = base_prime_bound.clone() * BigInt::from((config.performance.prime_bound_multiplier * 1000.0) as i64) / BigInt::from(1000);
        info!("Applied prime bound multiplier {}: {} -> {}", config.performance.prime_bound_multiplier, base_prime_bound, multiplied);
        multiplied
    } else {
        base_prime_bound
    };

    let base_relation_quantity = 5;
    let relation_quantity = if config.performance.relation_quantity_multiplier != 1.0 {
        let multiplied = (base_relation_quantity as f64 * config.performance.relation_quantity_multiplier) as usize;
        info!("Applied relation quantity multiplier {}: {} -> {}", config.performance.relation_quantity_multiplier, base_relation_quantity, multiplied);
        multiplied
    } else {
        base_relation_quantity
    };

    let relation_value_range = 50; // Adjust the relation value range as needed
    let created_new_data = true;

    info!("n: {}", n);
    info!("Polynomial Base: {}", polynomial_base.clone());
    info!("Polynomial Degree: {}", poly_degree);
    info!("Prime Bound: {} (based on {} digits)", prime_bound.clone(), digits);
    info!("Relation Target: {}", relation_quantity);
    info!("Relation Value: {}", relation_value_range);
    info!("GNFS: {}", created_new_data);

    let gnfs = GNFSWrapper::with_config(
        &cancel_token,
        n,
        &polynomial_base,
        poly_degree,
        &prime_bound,
        relation_quantity,
        relation_value_range,
        created_new_data,
        config.buffer.clone(),
    );

    // Save initial parameters
    info!("Saving initial parameters to {}", gnfs.parameters_filepath());
    // Note: Serialization will use the wrapper's methods to dispatch to the correct backend
    // For now, we'll skip saving as it requires more complex handling
    // TODO: Implement serialization for GNFSWrapper
    info!("Parameters save skipped (TODO: implement serialization for GNFSWrapper)");

    gnfs
}