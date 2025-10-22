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
