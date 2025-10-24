# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust implementation of the General Number Field Sieve (GNFS), the most advanced classical algorithm for factoring large integers. The project is inspired by AdamWhiteHat's GNFS implementation and is intended for educational and research purposes.

## Build and Development Commands

### Building
```bash
cargo build --release
```

### Running Tests
```bash
cargo test
```

### Running the Program
```bash
cargo run --release
```

### Checking Code
```bash
cargo check
cargo clippy
```

## High-Level Architecture

### Core Algorithm Flow

The GNFS factorization process follows this sequence:

1. **Initialization** (`core/gnfs.rs`): Creates GNFS instance with polynomial selection, prime base construction, and factor pair generation
2. **Relation Sieving** (`relation_sieve/`): Searches for smooth relations using the polynomial and factor bases
3. **Matrix Construction** (`Matrix/`): Builds a Gaussian matrix from smooth relations
4. **Square Root Extraction** (`square_root/`): Finds square roots in finite fields to extract factors

### Module Structure

- **`Core/`**: Central GNFS orchestration and data structures
  - `gnfs.rs`: Main GNFS struct coordinating the entire factorization process
  - `factor_base.rs`: Manages rational, algebraic, and quadratic factor bases
  - `cancellation_token.rs`: Handles cancellation of long-running operations
  - `directory_location.rs`: Manages file paths for persisting state
  - `serialization/`: Save/load functionality for GNFS state (partially implemented)

- **`Factor/`**: Factor pair management for the factor bases
  - `factor_pair_collection.rs`: Collections of (p, r) pairs where p is prime
  - Factory methods construct rational, algebraic, and quadratic factor pair collections

- **`Polynomial/`**: Polynomial arithmetic and field operations
  - `polynomial.rs`: Core polynomial representation and operations
  - `field.rs`: Finite field arithmetic
  - `algorithms.rs`: Polynomial-specific algorithms

- **`integer_math/`**: Number-theoretic utilities
  - `prime_factory.rs`: Prime generation and primality testing
  - `fast_prime_sieve.rs`: Efficient prime sieving
  - `legendre.rs`, `quadratic_residue.rs`: Modular arithmetic operations
  - `gcd.rs`: Greatest common divisor

- **`relation_sieve/`**: Relation discovery through sieving
  - `poly_relations_sieve_progress.rs`: Orchestrates the sieving process, tracks progress
  - `relation.rs`: Represents a single relation (a, b) pair
  - `relation_container.rs`: Stores and manages collections of relations

- **`Matrix/`**: Gaussian elimination for linear algebra step
  - `gaussian_matrix.rs`: Matrix representation for solving linear systems
  - `matrix_solve.rs`: Solution algorithms

- **`square_root/`**: Final square root extraction
  - `square_finder.rs`: Algorithms to find square roots in finite fields
  - `finite_field_arithmetic.rs`: Field arithmetic for square root computation

### Key Data Structures

**GNFS struct** (`core/gnfs.rs:20-33`): Central coordinator containing:
- `n`: Number to factor
- `polynomial_degree`, `polynomial_base`: Polynomial parameters
- `current_polynomial`: Active polynomial for sieving
- `prime_factor_base`: Three prime bases (rational, algebraic, quadratic)
- Factor pair collections (rational, algebraic, quadratic)
- `current_relations_progress`: Tracks sieving state
- `save_locations`: Persistence paths

**Factor Bases**: Three distinct bases used in GNFS:
- Rational Factor Base: Primes up to `rational_factor_base_max`
- Algebraic Factor Base: Primes up to `algebraic_factor_base_max` (typically 3x rational)
- Quadratic Factor Base: A smaller set of larger primes

**Relations**: Pairs (a, b) where both rational and algebraic norms factor smoothly over the respective factor bases

### Parallelization

The codebase uses:
- **`rayon`** for parallel iteration (data parallelism)
- **`tokio`** async runtime (currently minimal usage)
- Atomic operations and Arc/Weak references for shared state

### State Persistence

GNFS state is saved to disk in directories named `gnfs_data_{n}`:
- Relations (smooth and rough)
- Factor pair collections
- Sieving progress

Note: Serialization is partially implemented (see TODO comments throughout)

## Development Notes

### Polynomial Degree Selection

Polynomial degree is automatically chosen based on input size (`gnfs.rs:147-160`):
- < 65 digits: degree 3
- < 125 digits: degree 4
- < 225 digits: degree 5
- etc.

### Prime Bound Selection

Prime bounds are chosen heuristically based on the size of N (`gnfs.rs:162-179`). The algebraic factor base is typically 3x the rational base.

### Testing Strategy

Currently the codebase has minimal formal tests. Test by:
1. Running with small composite numbers (e.g., 45113 in `main.rs:38`)
2. Verifying log output shows progress through each stage
3. Checking that factor bases are constructed correctly

### Known TODOs

Multiple TODOs exist throughout the codebase:
- Complete serialization/deserialization implementation
- Implement `PrimeFactory::get_approximate_value_from_index`
- Complete matrix solving and square root extraction
- Add comprehensive unit tests
- Error handling improvements

### Logging

Uses `env_logger` with configurable levels. Set via environment:
```bash
MY_LOG_LEVEL=debug cargo run
```

Default level is `info`. Key stages log their progress.

### Environment Variables

The GNFS implementation supports several environment variables for customization:

#### GNFS_OUTPUT_DIR
Controls where GNFS saves its working data (relations, factor bases, progress).

**Default:** `.` (current directory)

**Examples:**
```bash
# Save to /tmp (useful for benchmarking or temporary runs)
GNFS_OUTPUT_DIR=/tmp ./target/release/gnfs 738883

# Save to specific project directory
GNFS_OUTPUT_DIR=/data/gnfs-workdir ./target/release/gnfs 100085411
```

The output directory will contain a subdirectory named after the number being factored (e.g., `738883/` containing `streamed_relations.jsonl`, `parameters.json`, etc.).

#### GNFS_CLEANUP
Controls whether to delete the output directory after successful factorization.

**Default:** `false` (keep output for inspection)

**Examples:**
```bash
# Clean up after successful factorization (useful for benchmarks)
GNFS_CLEANUP=true ./target/release/gnfs 143

# Keep output directory (default behavior)
./target/release/gnfs 143
```

**Warning:** Only set `GNFS_CLEANUP=true` if you don't need the intermediate data. The cleanup happens regardless of whether factorization succeeded.

#### GNFS_RELATION_BUFFER_SIZE
Controls how many smooth relations are buffered in memory before flushing to disk.

**Default:** `50`

**Trade-offs:**
- **Too small** (e.g., 5): Excessive disk I/O, high system CPU usage (50%+), context switches
- **Too large** (e.g., 500): Higher memory usage, risk of OOM on large numbers
- **Optimal** (50-100): Good balance for most workloads

**Examples:**
```bash
# Reduce disk I/O for fast SSDs (use larger buffer)
GNFS_RELATION_BUFFER_SIZE=100 ./target/release/gnfs 738883

# Reduce memory usage for memory-constrained systems
GNFS_RELATION_BUFFER_SIZE=25 ./target/release/gnfs 100085411

# Use default (recommended)
./target/release/gnfs 738883
```

#### MY_LOG_LEVEL
Controls logging verbosity (from `env_logger`).

**Default:** `info`

**Options:** `error`, `warn`, `info`, `debug`, `trace`

**Example:**
```bash
MY_LOG_LEVEL=debug cargo run --release
```

#### GNFS_THREADS
Controls the number of parallel threads used for sieving (from rayon).

**Default:** Number of logical CPU cores

**Example:**
```bash
# Use 8 threads for sieving
GNFS_THREADS=8 ./target/release/gnfs 100085411
```

### Combined Usage Examples

```bash
# Benchmark run: use /tmp, cleanup after, with timing
time env GNFS_OUTPUT_DIR=/tmp GNFS_CLEANUP=true MY_LOG_LEVEL=info \
  ./target/release/gnfs 738883

# Production run: custom directory, keep output, debug logging
GNFS_OUTPUT_DIR=/data/gnfs GNFS_RELATION_BUFFER_SIZE=100 MY_LOG_LEVEL=debug \
  ./target/release/gnfs 10003430467

# Memory-constrained run: small buffer, fewer threads
GNFS_RELATION_BUFFER_SIZE=25 GNFS_THREADS=4 \
  ./target/release/gnfs 100085411
```

## Performance Benchmarking

The project includes a comprehensive benchmarking suite for tracking performance across changes and identifying optimization opportunities.

### Running Benchmarks

**Basic usage** (benchmarks 7, 9, and 11 digit numbers by default):
```bash
cargo build --release
./target/release/gnfs --bench
```

**Custom digit counts**:
```bash
./target/release/gnfs --bench 6 7 9 11 12
```

**With timeout** (recommended for larger numbers):
```bash
gtimeout 300 ./target/release/gnfs --bench 9 11 12
```

### Benchmark Output

Benchmarks produce two outputs:

1. **Console summary**: Real-time progress and final summary showing:
   - System information (CPU, RAM, OS, Git commit)
   - Initialization time
   - Sieving time (breakdown by stage)
   - Relations found vs required
   - Percentage of time spent in each stage

2. **JSON file**: Timestamped file `benchmark_results_<timestamp>.json` containing:
   - Complete system metadata
   - Detailed timing for each stage
   - Number of relations found
   - Git commit hash for reproducibility

### Example Output

```
================================================================================
GNFS BENCHMARK SUITE
================================================================================

System Information:
  Hostname:     Daniels-MBP.lan
  OS:           Darwin 26.0.1
  CPU:          Apple M3 Pro (12 cores, 12 threads)
  Memory:       18432 MB
  Git:          5e95bca2 (benchmarking-suite)
  Rust:         rustc 1.89.0 (29483883e 2025-08-04)

Number:           738883 (6 digits)
Total Time:       598 ms
Relations:        886 / 1000 required
Stage Breakdown:
  Initialization: 1 ms
  Sieving:        597 ms (99.8% of total)
```

### Comparing Benchmarks

To compare performance before/after changes:

1. **Baseline**: Run benchmarks on main branch
   ```bash
   git checkout main
   cargo build --release
   ./target/release/gnfs --bench 7 9 11
   mv benchmark_results_*.json baseline.json
   ```

2. **After changes**: Run benchmarks on your branch
   ```bash
   git checkout your-branch
   cargo build --release
   ./target/release/gnfs --bench 7 9 11
   mv benchmark_results_*.json after.json
   ```

3. **Compare**: Load both JSON files and compare `total_time_ms` and `sieving_ms` values

### Key Insights from Benchmarks

Based on current benchmarks:

- **Sieving dominates**: 98-99% of factorization time is spent in the sieving stage
- **Initialization is fast**: < 1-2ms regardless of number size
- **Matrix/square root**: Not yet implemented, so not measured

**Optimization priorities** (based on time spent):
1. Sieving optimization (98%+ of time) - GPU/OpenCL, SIMD, or algorithmic improvements
2. Matrix solving (not yet implemented)
3. Polynomial selection (occurs during initialization, already fast)

### Pre-Selected Test Numbers

The benchmark suite uses pre-selected semiprimes for consistent testing:

| Digit Count | Test Number | Description |
|-------------|-------------|-------------|
| 6 | 143 | 11 × 13 |
| 7 | 738883 | Known composite |
| 9 | 100085411 | Tested semiprime |
| 10 | 1000730021 | Tested semiprime |
| 11 | 10003430467 | Tested semiprime |
| 12 | 100002599317 | Tested semiprime |
| 14 | 10000004400000259 | Large semiprime |

## macOS Development Notes

### Command-Line Tools

**Timeout on macOS:**
- Use `gtimeout` instead of `timeout` (GNU coreutils)
- Install via: `brew install coreutils`
- Example: `gtimeout 60 ./target/release/gnfs 47893197`

**Alternative patterns:**
- Background with monitoring: `./command & PID=$! ; sleep 60 && kill $PID 2>/dev/null`
- Built-in timeout on newer macOS: Some systems may have timeout, but gtimeout is more reliable

