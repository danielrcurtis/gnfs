// src/benchmark_cli.rs
// CLI benchmark runner - separated to avoid polluting main.rs

use env_logger::Env;
use chrono::Utc;
use crate::benchmark::runner::BenchmarkRunner;

pub fn run_benchmarks(args: &[String]) {
    // Initialize logging for benchmarks
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("\n{}", "=".repeat(80));
    println!("GNFS BENCHMARK SUITE");
    println!("{}", "=".repeat(80));

    // Parse digit counts from command line, or use defaults
    let digit_counts: Vec<usize> = if args.len() > 2 {
        args[2..].iter()
            .filter_map(|s| s.parse::<usize>().ok())
            .collect()
    } else {
        // Default: benchmark 7, 9, and 11 digit numbers
        vec![7, 9, 11]
    };

    println!("\nBenchmarking digit counts: {:?}", digit_counts);

    // Create benchmark runner
    let mut runner = BenchmarkRunner::new();

    // Run factorization benchmarks
    runner.run_factorization_benchmarks(&digit_counts);

    // Print summary
    runner.print_summary();

    // Save results to JSON
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("benchmark_results_{}.json", timestamp);

    match runner.save_results(&filename) {
        Ok(_) => println!("\nResults saved to: {}", filename),
        Err(e) => eprintln!("Error saving results: {}", e),
    }

    println!("\nTo compare with a previous run:");
    println!("  Load and compare JSON files manually, or use a comparison tool");
    println!("\nExample: compare benchmark_results_<timestamp1>.json with benchmark_results_<timestamp2>.json");
}
