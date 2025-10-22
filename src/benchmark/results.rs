// src/benchmark/results.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::benchmark::system_info::SystemInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub category: String,
    pub mean_time_ns: u64,
    pub std_dev_ns: u64,
    pub iterations: u64,
    pub throughput: Option<Throughput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Throughput {
    pub unit: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorizationBenchmark {
    pub number: String,
    pub digit_count: usize,
    pub factors: Vec<String>,
    pub total_time_ms: u64,
    pub stage_times: StageTimings,
    pub relations_found: usize,
    pub relations_required: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageTimings {
    pub initialization_ms: u64,
    pub sieving_ms: u64,
    pub matrix_construction_ms: Option<u64>,
    pub matrix_solving_ms: Option<u64>,
    pub square_root_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub timestamp: DateTime<Utc>,
    pub system_info: SystemInfo,
    pub micro_benchmarks: Vec<BenchmarkResult>,
    pub factorization_benchmarks: Vec<FactorizationBenchmark>,
}

impl BenchmarkSuite {
    pub fn new() -> Self {
        BenchmarkSuite {
            timestamp: Utc::now(),
            system_info: SystemInfo::collect(),
            micro_benchmarks: Vec::new(),
            factorization_benchmarks: Vec::new(),
        }
    }

    pub fn add_micro_benchmark(&mut self, result: BenchmarkResult) {
        self.micro_benchmarks.push(result);
    }

    pub fn add_factorization_benchmark(&mut self, result: FactorizationBenchmark) {
        self.factorization_benchmarks.push(result);
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let suite = serde_json::from_str(&json)?;
        Ok(suite)
    }

    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(80));
        println!("BENCHMARK SUITE RESULTS");
        println!("{}", "=".repeat(80));
        println!("\nTimestamp: {}", self.timestamp);
        println!("{}", self.system_info.to_string_pretty());

        if !self.micro_benchmarks.is_empty() {
            println!("\n{}", "-".repeat(80));
            println!("MICRO-BENCHMARKS");
            println!("{}", "-".repeat(80));
            println!("{:<40} {:>15} {:>15}", "Benchmark", "Mean Time", "Throughput");
            println!("{}", "-".repeat(80));

            for bench in &self.micro_benchmarks {
                let mean_time = Self::format_duration(bench.mean_time_ns);
                let throughput = bench.throughput.as_ref()
                    .map(|t| format!("{:.2} {}", t.value, t.unit))
                    .unwrap_or_else(|| "-".to_string());
                println!("{:<40} {:>15} {:>15}", bench.name, mean_time, throughput);
            }
        }

        if !self.factorization_benchmarks.is_empty() {
            println!("\n{}", "-".repeat(80));
            println!("END-TO-END FACTORIZATION BENCHMARKS");
            println!("{}", "-".repeat(80));

            for bench in &self.factorization_benchmarks {
                println!("\nNumber:           {} ({} digits)", bench.number, bench.digit_count);
                println!("Factors:          {}", bench.factors.join(" × "));
                println!("Total Time:       {} ms", bench.total_time_ms);
                println!("Relations:        {} / {} required", bench.relations_found, bench.relations_required);
                println!("Stage Breakdown:");
                println!("  Initialization: {} ms", bench.stage_times.initialization_ms);
                println!("  Sieving:        {} ms ({:.1}% of total)",
                    bench.stage_times.sieving_ms,
                    100.0 * bench.stage_times.sieving_ms as f64 / bench.total_time_ms as f64);

                if let Some(matrix_ms) = bench.stage_times.matrix_construction_ms {
                    println!("  Matrix Build:   {} ms", matrix_ms);
                }
                if let Some(solve_ms) = bench.stage_times.matrix_solving_ms {
                    println!("  Matrix Solve:   {} ms", solve_ms);
                }
                if let Some(sqrt_ms) = bench.stage_times.square_root_ms {
                    println!("  Square Root:    {} ms", sqrt_ms);
                }
            }
        }

        println!("\n{}", "=".repeat(80));
    }

    fn format_duration(ns: u64) -> String {
        if ns < 1_000 {
            format!("{} ns", ns)
        } else if ns < 1_000_000 {
            format!("{:.2} µs", ns as f64 / 1_000.0)
        } else if ns < 1_000_000_000 {
            format!("{:.2} ms", ns as f64 / 1_000_000.0)
        } else {
            format!("{:.2} s", ns as f64 / 1_000_000_000.0)
        }
    }
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self::new()
    }
}
