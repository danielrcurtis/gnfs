// src/benchmark/runner.rs

use num::BigInt;
use std::time::Instant;
use log::info;
use crate::core::gnfs_wrapper::GNFSWrapper;
use crate::core::cancellation_token::CancellationToken;
use crate::benchmark::results::{BenchmarkSuite, FactorizationBenchmark, StageTimings};

pub struct BenchmarkRunner {
    suite: BenchmarkSuite,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        BenchmarkRunner {
            suite: BenchmarkSuite::new(),
        }
    }

    /// Run factorization benchmarks for numbers of specified digit counts
    pub fn run_factorization_benchmarks(&mut self, digit_counts: &[usize]) {
        println!("\n{}", "=".repeat(80));
        println!("Running End-to-End Factorization Benchmarks");
        println!("{}", "=".repeat(80));

        for &digits in digit_counts {
            println!("\n{}", "-".repeat(80));
            println!("Benchmarking {}-digit factorization", digits);
            println!("{}", "-".repeat(80));

            let test_number = Self::get_test_number_for_digits(digits);
            println!("Test number: {}", test_number);

            let result = self.benchmark_single_factorization(&test_number);
            self.suite.add_factorization_benchmark(result);
        }
    }

    /// Benchmark a single factorization
    pub fn benchmark_single_factorization(&self, n: &BigInt) -> FactorizationBenchmark {
        let start_total = Instant::now();

        // Stage 1: Initialization
        let start_init = Instant::now();

        // Initialize GNFS with appropriate parameters (matching main.rs logic)
        let cancel_token = CancellationToken::new();
        let polynomial_base = BigInt::from(31);
        let poly_degree = 3;

        // Determine prime bound based on digit count (matching main.rs logic)
        // Updated with Fix 2: exponential scaling for 10+ digits
        let digits = n.to_string().len();
        let prime_bound = if digits <= 8 {
            BigInt::from(100)         // 8 digits: ~0.3s, 254 relations
        } else if digits == 9 {
            BigInt::from(100)         // 9 digits: 2-28s (varies), sufficient smooth relations
        } else if digits == 10 {
            BigInt::from(1000)        // 10 digits: increased from 200, then 500 (Fix 2 - prevent exhaustion)
        } else if digits == 11 {
            BigInt::from(1000)        // 11 digits: increased from 400 (exponential scaling)
        } else if digits == 12 {
            BigInt::from(2000)        // 12 digits: increased from 800 (exponential scaling)
        } else if digits <= 14 {
            BigInt::from(5000)        // 13-14 digits: increased from 2000
        } else if digits <= 16 {
            BigInt::from(10000)       // 15-16 digits: increased from 5000
        } else if digits <= 18 {
            BigInt::from(20000)       // 17-18 digits: increased from 10000
        } else {
            // For larger numbers (19+ digits), use exponential scaling
            // Formula: base * (1.5 ^ (digits - 18))
            let base = 20000_i64;
            let exponent = digits - 18;
            let multiplier = (1.5_f64.powi(exponent as i32) * 1000.0) as i64;
            BigInt::from(base) * BigInt::from(multiplier) / BigInt::from(1000)
        };

        // For benchmarks, we want to test actual sieving performance, not just initialization
        // Use parameters that provide meaningful work while avoiding memory issues
        // NOTE: Even with disk streaming, large numbers use significant memory due to
        // temporary BigInt allocations during sieving. Keep targets very conservative.
        let (relation_quantity, relation_value_range) = if digits <= 9 {
            (1000, 200)  // Small numbers: high target, moderate range
        } else if digits <= 11 {
            (50, 50)     // Medium numbers: VERY low target to avoid memory issues
        } else {
            (25, 50)     // Large numbers: minimal targets for benchmarking only
        };
        let created_new_data = true;

        let mut gnfs = GNFSWrapper::new(
            &cancel_token,
            n,
            &polynomial_base,
            poly_degree,
            &prime_bound,
            relation_quantity,
            relation_value_range,
            created_new_data,
        );
        let init_time = start_init.elapsed();

        // Log which backend was selected
        info!("Backend selected for benchmark: {}", gnfs.backend_name());

        let (rat_fb_size, alg_fb_size, _quad_fb_size) = gnfs.get_factor_base_info();
        println!("  Initialization: {:?}", init_time);
        println!("  Backend: {}", gnfs.backend_name());
        println!("  Polynomial degree: {}", gnfs.polynomial_degree());
        println!("  Rational factor base: {} primes", rat_fb_size);
        println!("  Algebraic factor base: {} primes", alg_fb_size);

        // Stage 2: Sieving
        let start_sieve = Instant::now();
        let sieve_cancel_token = CancellationToken::new();

        // Use the wrapper's find_relations method
        gnfs.find_relations(&sieve_cancel_token, true);

        let sieve_time = start_sieve.elapsed();

        let (relations_found, relations_required) = gnfs.get_relations_info();

        println!("  Sieving: {:?}", sieve_time);
        println!("  Relations found: {} / {} required", relations_found, relations_required);

        // TODO: Add matrix construction, solving, and square root stages when implemented
        // For now, we only benchmark sieving since that's the complete part

        let total_time = start_total.elapsed();

        // Get factors (if factorization completed)
        let factors = Self::extract_factors(n);

        FactorizationBenchmark {
            number: n.to_string(),
            digit_count: n.to_string().len(),
            factors: factors.iter().map(|f| f.to_string()).collect(),
            total_time_ms: total_time.as_millis() as u64,
            stage_times: StageTimings {
                initialization_ms: init_time.as_millis() as u64,
                sieving_ms: sieve_time.as_millis() as u64,
                matrix_construction_ms: None,
                matrix_solving_ms: None,
                square_root_ms: None,
            },
            relations_found,
            relations_required,
        }
    }

    /// Get test numbers for different digit counts
    fn get_test_number_for_digits(digits: usize) -> BigInt {
        // Pre-selected semiprimes of various sizes for consistent benchmarking
        match digits {
            6 => BigInt::from(143_u64), // 11 × 13
            7 => BigInt::from(738_883_u64), // Known composite
            9 => BigInt::from(100_085_411_u64), // Known from your tests
            10 => BigInt::from(1_000_730_021_u64),
            11 => BigInt::from(10_003_430_467_u64),
            12 => BigInt::from(100_002_599_317_u64),
            14 => BigInt::from(10_000_004_400_000_259_u64),
            _ => {
                // For other digit counts, use 10^(digits-1) + small offset as an approximation
                // This gives a number of roughly the right size
                BigInt::from(10_u64).pow(digits as u32 - 1) + BigInt::from(143_u64)
            }
        }
    }

    /// Extract factors from a number (if known)
    fn extract_factors(n: &BigInt) -> Vec<BigInt> {
        // For now, just return the number itself
        // In a complete implementation, this would return the actual factors found by GNFS
        vec![n.clone()]
    }

    /// Save results to JSON file
    pub fn save_results(&self, path: &str) -> std::io::Result<()> {
        self.suite.save_to_file(path)
    }

    /// Print summary to console
    pub fn print_summary(&self) {
        self.suite.print_summary();
    }

    /// Get the benchmark suite
    pub fn get_suite(&self) -> &BenchmarkSuite {
        &self.suite
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Compare two benchmark suites
pub fn compare_benchmarks(baseline_path: &str, current_path: &str) -> std::io::Result<()> {
    let baseline = BenchmarkSuite::load_from_file(baseline_path)?;
    let current = BenchmarkSuite::load_from_file(current_path)?;

    println!("\n{}", "=".repeat(80));
    println!("BENCHMARK COMPARISON");
    println!("{}", "=".repeat(80));
    println!("\nBaseline: {} ({})", baseline.timestamp, baseline.system_info.git_commit.chars().take(8).collect::<String>());
    println!("Current:  {} ({})", current.timestamp, current.system_info.git_commit.chars().take(8).collect::<String>());

    println!("\n{}", "-".repeat(80));
    println!("END-TO-END FACTORIZATION COMPARISON");
    println!("{}", "-".repeat(80));
    println!("{:<15} {:>15} {:>15} {:>15}", "Digits", "Baseline (ms)", "Current (ms)", "Speedup");
    println!("{}", "-".repeat(80));

    for current_bench in &current.factorization_benchmarks {
        if let Some(baseline_bench) = baseline.factorization_benchmarks.iter()
            .find(|b| b.digit_count == current_bench.digit_count) {

            let speedup = baseline_bench.total_time_ms as f64 / current_bench.total_time_ms as f64;
            let speedup_str = if speedup > 1.0 {
                format!("{:.2}x faster", speedup)
            } else {
                format!("{:.2}x slower", 1.0 / speedup)
            };

            println!("{:<15} {:>15} {:>15} {:>15}",
                current_bench.digit_count,
                baseline_bench.total_time_ms,
                current_bench.total_time_ms,
                speedup_str
            );
        }
    }

    println!("{}", "=".repeat(80));
    Ok(())
}
