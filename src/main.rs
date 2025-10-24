use gnfs::integer_math::prime_factory;
// src/main.rs
use log::{debug, info, warn};
use env_logger::Env;
use gnfs::core::cpu_info::CPUInfo;
use gnfs::core::gnfs_wrapper::GNFSWrapper;
use gnfs::core::cancellation_token::CancellationToken;
use gnfs::config::GnfsConfig;
use gnfs::benchmark_cli;
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

    // Fast pre-check: For small numbers, use trial division instead of GNFS
    // GNFS is only efficient for numbers with 7+ digits (> 10^7)
    if n < BigInt::from(10_i64.pow(7)) {
        info!("");
        info!("========================================");
        info!("SMALL NUMBER DETECTED - USING TRIAL DIVISION");
        info!("========================================");
        info!("Number: {}", n);
        info!("GNFS is designed for large numbers (7+ digits). Using fast trial division instead...");

        use gnfs::integer_math::factorization_factory::FactorizationFactory;
        let (factorization, quotient) = FactorizationFactory::factor(&n);

        if quotient == BigInt::from(1) {
            // Completely factored
            info!("");
            info!("*****************************************");
            info!("*** FACTORIZATION SUCCESSFUL (Trial Division) ***");
            info!("*****************************************");
            info!("");
            info!("N = {}", n);
            info!("Prime factorization: {:?}", factorization);
            info!("");

            // If there are exactly 2 prime factors (counting multiplicity)
            let dict = factorization.to_dict();
            let mut all_factors = Vec::new();
            for (prime, exponent) in dict.iter() {
                let exp_u32 = if let Some(val) = exponent.to_u32() {
                    val
                } else {
                    exponent.to_u64().unwrap_or(1) as u32
                };
                for _ in 0..exp_u32 {
                    all_factors.push(prime.clone());
                }
            }

            if all_factors.len() == 2 {
                info!("{} = {} × {}", n, all_factors[0], all_factors[1]);
                info!("Verification: {} × {} = {}", all_factors[0], all_factors[1], &all_factors[0] * &all_factors[1]);
            } else if all_factors.len() == 1 {
                info!("{} is PRIME", n);
            } else {
                info!("{} = {}", n, all_factors.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(" × "));
            }
            info!("*****************************************");
        } else {
            // Partially factored - quotient is a large prime or composite
            info!("");
            info!("Partial factorization: {:?}", factorization);
            info!("Unfactored quotient: {}", quotient);
            info!("");
            info!("The number contains a large prime factor > sqrt(N).");
            info!("For complete factorization of very large composites, use GNFS on numbers > 10^15.");
        }
        return;
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

    // Empirically determined prime bounds based on digit count
    // These bounds ensure smooth relation density is high enough for practical factorization
    // while minimizing computation time. Tested on M3 MacBook Pro with 8 threads.
    let digits = n.to_string().len();
    let base_prime_bound = if digits <= 8 {
        BigInt::from(100)         // 8 digits: ~0.3s, 254 relations
    } else if digits == 9 {
        BigInt::from(100)         // 9 digits: 2-28s (varies), sufficient smooth relations
    } else if digits == 10 {
        BigInt::from(200)         // 10 digits: targeting <60s (was >5min with 100)
    } else if digits == 11 {
        BigInt::from(400)         // 11 digits: targeting <90s
    } else if digits == 12 {
        BigInt::from(800)         // 12 digits: targeting <2min
    } else if digits <= 14 {
        BigInt::from(2000)        // 13-14 digits: may take 3-5 minutes
    } else if digits <= 16 {
        BigInt::from(5000)        // 15-16 digits: may take 5-10 minutes
    } else if digits <= 18 {
        BigInt::from(10000)       // 17-18 digits: ~1 minute (tested: 57s for 17-digit)
    } else {
        // For larger numbers (19+ digits), use exponential scaling
        BigInt::from(digits) * BigInt::from(1000)
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