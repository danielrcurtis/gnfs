# GNFS Stage 4 Square Root Extraction: Parallelization and GPU Acceleration Implementation Plan

**Document Version:** 1.0
**Target Component:** `src/square_root/square_finder.rs`
**Goal:** Reduce Stage 4 execution time from 30+ minutes to under 1 minute through parallelization and GPU acceleration

---

## Executive Summary

This document outlines a four-phase implementation plan to dramatically accelerate the GNFS Stage 4 (square root extraction) through CPU parallelization and GPU acceleration. The bottleneck currently resides in `calculate_algebraic_side()` (lines 187-382), which sequentially tests thousands of primes for irreducibility.

**Expected Performance Gains:**
- **Phase 1 (CPU Parallelization):** 30 minutes → 3-5 minutes (5-10x speedup)
- **Phase 3 (GPU Acceleration):** 3-5 minutes → 30-60 seconds (20-100x speedup for large inputs)
- **Total Improvement:** 30-60x speedup for typical composite numbers

**Recommended Starting Point:** Phase 1 (CPU Parallelization with Rayon)

---

## Current Bottleneck Analysis

### Problem Location
File: `src/square_root/square_finder.rs`, function `calculate_algebraic_side()`

### Current Sequential Algorithm (Lines 253-371)

```rust
loop {
    // Get next prime for testing
    let next_p_i128 = PrimeFactory::get_next_prime_from_i128(last_p_i128 + 1).to_i128().unwrap();
    let p = next_p_i128.to_bigint().unwrap();

    // Test irreducibility using the test prime
    let g = Polynomial::parse(&format!("X^{} - X", test_p));
    let h = finite_field_arithmetic::mod_mod(&g, f, &test_p);
    let gcd = Polynomial::field_gcd(&h, f, &test_p);
    let is_irreducible = gcd.cmp(&Polynomial::one()) == Ordering::Equal;

    if is_irreducible {
        // Compute square root (expensive operation)
        let chosen_poly = finite_field_arithmetic::square_root(&self.s, f, &p, ...);
        let eval = chosen_poly.evaluate(&self.gnfs.polynomial_base);
        let x = eval.mod_floor(&p);

        primes.push(p.clone());
        values.push(x.clone());
    }

    // Need degree number of irreducible primes with product > N
    if primes.len() == degree as usize && prime_product > N { ... }
}
```

### Performance Characteristics

**Operations per prime test:**
1. **Prime generation:** O(1) - Fast
2. **Polynomial construction:** O(d) where d = degree - Fast
3. **Polynomial modular exponentiation:** O(d² log p) - **Expensive**
4. **GCD computation:** O(d² log p) - **Expensive**
5. **Square root (if irreducible):** O(d³ log² p) - **Very Expensive**

**Typical workload:**
- For a 20-digit composite: ~1,000-5,000 prime tests
- For a 50-digit composite: ~10,000-50,000 prime tests
- Each test: 10-100ms (varies with polynomial degree and prime size)

**Critical Observation:** Prime tests are completely independent and can be parallelized.

---

## PHASE 1: CPU Parallelization with Rayon

**Timeline:** 1-2 weeks
**Expected Speedup:** 5-10x (30 minutes → 3-5 minutes)
**Complexity:** Low
**Risk:** Low

### Overview

Leverage Rust's Rayon library (already in `Cargo.toml`) to batch-test multiple primes in parallel using all available CPU cores.

### Implementation Strategy

#### 1.1 Batch Prime Generation

**Create:** `src/square_root/batch_prime_generator.rs`

```rust
use num::BigInt;
use crate::integer_math::prime_factory::PrimeFactory;

pub struct BatchPrimeGenerator {
    current_start: i128,
}

impl BatchPrimeGenerator {
    pub fn new(start: i128) -> Self {
        BatchPrimeGenerator { current_start: start }
    }

    /// Generate the next batch of primes
    /// Returns: Vec of (index, prime) tuples for tracking
    pub fn next_batch(&mut self, batch_size: usize) -> Vec<BigInt> {
        let mut primes = Vec::with_capacity(batch_size);
        let mut current = self.current_start;

        for _ in 0..batch_size {
            current = PrimeFactory::get_next_prime_from_i128(current + 1).to_i128().unwrap();
            primes.push(current.to_bigint().unwrap());
        }

        self.current_start = current;
        primes
    }
}
```

#### 1.2 Parallel Irreducibility Testing

**Modify:** `src/square_root/square_finder.rs`

```rust
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

impl SquareFinder {
    pub fn calculate_algebraic_side(&mut self, cancel_token: &CancellationToken) -> (BigInt, BigInt) {
        // ... existing setup code (lines 188-250) ...

        let batch_size = std::env::var("GNFS_STAGE4_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(num_cpus::get() * 4); // Default: 4 batches per core

        info!("Using parallel batch processing with batch_size = {}", batch_size);

        let mut batch_generator = BatchPrimeGenerator::new(last_p_i128);

        // Shared state for collecting results
        let primes = Arc::new(Mutex::new(Vec::new()));
        let values = Arc::new(Mutex::new(Vec::new()));
        let primes_tested = Arc::new(AtomicUsize::new(0));

        loop {
            if cancel_token.is_cancellation_requested() {
                return (BigInt::one(), BigInt::one());
            }

            // Generate batch of primes to test
            let prime_batch = batch_generator.next_batch(batch_size);

            // Test primes in parallel
            let results: Vec<Option<(BigInt, BigInt)>> = prime_batch
                .par_iter()
                .map(|p| {
                    self.test_prime_irreducibility(p, &f, degree)
                })
                .collect();

            // Process results sequentially (maintains order)
            let mut primes_lock = primes.lock().unwrap();
            let mut values_lock = values.lock().unwrap();

            for result in results {
                if let Some((prime, value)) = result {
                    // Remove oldest if at capacity
                    if primes_lock.len() == degree as usize {
                        primes_lock.remove(0);
                        values_lock.remove(0);
                    }

                    primes_lock.push(prime);
                    values_lock.push(value);

                    info!("Found irreducible prime! Total: {}", primes_lock.len());
                }
            }

            let current_count = primes_tested.fetch_add(batch_size, Ordering::SeqCst);
            if current_count % 100 == 0 {
                info!("Tested {} primes, found {} irreducible",
                      current_count, primes_lock.len());
            }

            // Check termination condition
            if primes_lock.len() == degree as usize {
                let prime_product: BigInt = primes_lock.iter().product();

                if &prime_product >= &self.n {
                    // Found enough primes, try to extract factors
                    let common_modulus = algorithms::chinese_remainder_theorem(
                        &primes_lock, &values_lock
                    );

                    // ... existing CRT and factor extraction code (lines 319-359) ...
                } else {
                    // Clear and continue
                    primes_lock.clear();
                    values_lock.clear();
                }
            }
        }
    }

    /// Test if prime p is irreducible for polynomial f
    /// Returns: Some((prime, square_root_value)) if irreducible, None otherwise
    fn test_prime_irreducibility(
        &self,
        p: &BigInt,
        f: &Polynomial,
        degree: usize
    ) -> Option<(BigInt, BigInt)> {
        // Test: gcd(X^p - X, f) = 1 (mod p)
        let g = Polynomial::parse(&format!("X^{} - X", p));
        let h = finite_field_arithmetic::mod_mod(&g, f, p);
        let gcd = Polynomial::field_gcd(&h, f, p);

        if gcd.cmp(&Polynomial::one()) == Ordering::Equal {
            // Compute square root
            let chosen_poly = finite_field_arithmetic::square_root(
                &self.s, f, p, degree.try_into().unwrap(),
                &self.gnfs.polynomial_base
            );
            let eval = chosen_poly.evaluate(&self.gnfs.polynomial_base);
            let x = eval.mod_floor(p);

            Some((p.clone(), x))
        } else {
            None
        }
    }
}
```

#### 1.3 Configuration

**Environment Variables:**
```bash
# Number of primes to test in each parallel batch
export GNFS_STAGE4_BATCH_SIZE=32

# Optional: Limit CPU cores used
export RAYON_NUM_THREADS=8
```

**Cargo.toml additions:**
```toml
[dependencies]
rayon = "1.11.0"           # Already present
num_cpus = "1.17"          # Already present
```

### Testing Strategy

1. **Unit Tests:** Test `test_prime_irreducibility()` with known primes
2. **Integration Tests:** Compare results against sequential implementation
3. **Benchmarks:** Measure speedup with different batch sizes
4. **Edge Cases:** Test cancellation, small/large batch sizes

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_batch_prime_generation() {
        let mut gen = BatchPrimeGenerator::new(1000);
        let batch = gen.next_batch(10);
        assert_eq!(batch.len(), 10);
        // Verify all are prime and ascending
    }

    #[test]
    fn test_irreducibility_parallel_matches_sequential() {
        // Compare results from both implementations
    }
}
```

### Performance Estimates

**Hardware:** 8-core CPU (16 threads with hyperthreading)
- Sequential: 1 test at a time
- Parallel: 16 tests simultaneously
- Theoretical speedup: 16x
- Practical speedup: 8-10x (accounting for overhead)

**Bottlenecks remaining after Phase 1:**
- Polynomial modular exponentiation (still CPU-bound)
- GCD computation (still CPU-bound)
- Memory bandwidth for large polynomials

---

## PHASE 2: GPU Foundation with OpenCL

**Timeline:** 2-3 weeks
**Expected Speedup:** None directly (infrastructure only)
**Complexity:** Medium
**Risk:** Medium

### Overview

Establish GPU computation infrastructure using OpenCL (cross-platform GPU support) with CPU fallback for systems without GPU support.

### Implementation Strategy

#### 2.1 Add OpenCL Dependencies

**Update:** `Cargo.toml`

```toml
[dependencies]
ocl = "0.19"               # OpenCL bindings for Rust
ocl-core = "0.11"

[features]
default = ["opencl"]
opencl = ["ocl", "ocl-core"]
no-gpu = []                # CPU-only build
```

#### 2.2 Backend Trait Abstraction

**Create:** `src/square_root/backends/mod.rs`

```rust
use num::BigInt;
use crate::polynomial::polynomial::Polynomial;

pub mod cpu_backend;

#[cfg(feature = "opencl")]
pub mod opencl_backend;

/// Trait for square root computation backends
pub trait SquareRootBackend: Send + Sync {
    /// Test if a prime is irreducible for the given polynomial
    fn test_irreducibility_batch(
        &self,
        primes: &[BigInt],
        polynomial: &Polynomial,
        degree: usize,
        s: &Polynomial,
        polynomial_base: &BigInt,
    ) -> Vec<Option<(BigInt, BigInt)>>;

    /// Backend name for logging
    fn name(&self) -> &str;

    /// Check if this backend is available on the current system
    fn is_available(&self) -> bool;
}
```

#### 2.3 CPU Backend Implementation

**Create:** `src/square_root/backends/cpu_backend.rs`

```rust
use super::SquareRootBackend;
use rayon::prelude::*;

pub struct CpuBackend;

impl CpuBackend {
    pub fn new() -> Self {
        CpuBackend
    }
}

impl SquareRootBackend for CpuBackend {
    fn test_irreducibility_batch(
        &self,
        primes: &[BigInt],
        polynomial: &Polynomial,
        degree: usize,
        s: &Polynomial,
        polynomial_base: &BigInt,
    ) -> Vec<Option<(BigInt, BigInt)>> {
        primes.par_iter()
            .map(|p| test_single_prime(p, polynomial, degree, s, polynomial_base))
            .collect()
    }

    fn name(&self) -> &str {
        "CPU (Rayon parallel)"
    }

    fn is_available(&self) -> bool {
        true // Always available
    }
}

// Move Phase 1 test_prime_irreducibility logic here
fn test_single_prime(...) -> Option<(BigInt, BigInt)> {
    // Implementation from Phase 1
}
```

#### 2.4 OpenCL Backend Stub

**Create:** `src/square_root/backends/opencl_backend.rs`

```rust
use super::SquareRootBackend;
use ocl::{Platform, Device, Context, Queue, Program};

pub struct OpenClBackend {
    context: Context,
    queue: Queue,
    program: Program,
}

impl OpenClBackend {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Detect GPU
        let platform = Platform::default();
        let device = Device::first(platform)?;

        info!("OpenCL Backend: Detected {} on {}",
              device.name()?, platform.name()?);

        let context = Context::builder()
            .platform(platform)
            .devices(device.clone())
            .build()?;

        let queue = Queue::new(&context, device, None)?;

        // Load kernel source (Phase 3)
        let program = Program::builder()
            .devices(device)
            .src(include_str!("kernels/polynomial_ops.cl"))
            .build(&context)?;

        Ok(OpenClBackend { context, queue, program })
    }
}

impl SquareRootBackend for OpenClBackend {
    fn test_irreducibility_batch(
        &self,
        primes: &[BigInt],
        polynomial: &Polynomial,
        degree: usize,
        s: &Polynomial,
        polynomial_base: &BigInt,
    ) -> Vec<Option<(BigInt, BigInt)>> {
        // Phase 3: GPU implementation
        // For Phase 2: Just fall back to CPU
        unimplemented!("GPU kernels implemented in Phase 3")
    }

    fn name(&self) -> &str {
        "GPU (OpenCL)"
    }

    fn is_available(&self) -> bool {
        OpenClBackend::new().is_ok()
    }
}
```

#### 2.5 Backend Selection Logic

**Create:** `src/square_root/backend_selector.rs`

```rust
use super::backends::*;
use std::sync::Arc;

pub fn select_backend() -> Arc<dyn SquareRootBackend> {
    // Check environment variable preference
    let prefer_cpu = std::env::var("GNFS_USE_CPU_ONLY")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    if prefer_cpu {
        info!("CPU-only mode requested via GNFS_USE_CPU_ONLY");
        return Arc::new(cpu_backend::CpuBackend::new());
    }

    // Try GPU first
    #[cfg(feature = "opencl")]
    {
        match opencl_backend::OpenClBackend::new() {
            Ok(backend) => {
                info!("Using GPU backend: OpenCL");
                return Arc::new(backend);
            }
            Err(e) => {
                warn!("GPU backend unavailable: {}", e);
                warn!("Falling back to CPU backend");
            }
        }
    }

    // Fallback to CPU
    info!("Using CPU backend with Rayon parallelization");
    Arc::new(cpu_backend::CpuBackend::new())
}
```

#### 2.6 Integration with SquareFinder

**Modify:** `src/square_root/square_finder.rs`

```rust
use crate::square_root::backend_selector;
use crate::square_root::backends::SquareRootBackend;

impl SquareFinder {
    pub fn calculate_algebraic_side(&mut self, cancel_token: &CancellationToken) -> (BigInt, BigInt) {
        // Select backend once
        let backend = backend_selector::select_backend();
        info!("Using backend: {}", backend.name());

        // ... existing setup ...

        loop {
            let prime_batch = batch_generator.next_batch(batch_size);

            // Use backend for testing
            let results = backend.test_irreducibility_batch(
                &prime_batch,
                &self.monic_polynomial,
                degree,
                &self.s,
                &self.gnfs.polynomial_base,
            );

            // ... rest of implementation ...
        }
    }
}
```

### Configuration

**Environment Variables:**
```bash
# Disable GPU and force CPU-only mode
export GNFS_USE_CPU_ONLY=1

# Specify OpenCL device (0=first GPU, 1=second GPU, etc.)
export GNFS_OPENCL_DEVICE=0

# OpenCL work group size (tuned in Phase 3)
export GNFS_OPENCL_WORKGROUP_SIZE=64
```

### Testing Strategy

1. **Backend Detection:** Verify correct backend selection
2. **Fallback Logic:** Ensure CPU fallback works when GPU unavailable
3. **Equivalence:** CPU and GPU backends produce identical results
4. **Error Handling:** Graceful degradation on OpenCL errors

---

## PHASE 3: GPU Polynomial Operations

**Timeline:** 3-4 weeks
**Expected Speedup:** 20-100x over Phase 1 (for large numbers)
**Complexity:** High
**Risk:** Medium-High

### Overview

Implement GPU kernels for the expensive polynomial operations: modular exponentiation, GCD, and square root extraction.

### 3.1 OpenCL Kernel: Polynomial Modular Exponentiation

**Create:** `src/square_root/backends/kernels/polynomial_ops.cl`

```c
// Polynomial representation: coefficients stored as arrays
// p[i] = coefficient of x^i

/**
 * Modular exponentiation: base^exp mod (modulus_poly, prime)
 *
 * Uses square-and-multiply algorithm
 * Each work item processes one prime from the batch
 */
__kernel void poly_mod_exp(
    __global const long *base_coeffs,      // Input polynomial coefficients
    const int base_degree,
    __global const long *mod_poly_coeffs,  // Modulus polynomial
    const int mod_degree,
    __global const long *primes,           // Batch of primes to test
    __global const long *exponents,        // Exponents (typically primes themselves)
    __global long *result_coeffs,          // Output: one polynomial per prime
    const int batch_size
) {
    int gid = get_global_id(0);
    if (gid >= batch_size) return;

    long prime = primes[gid];
    long exp = exponents[gid];

    // Result starts as 1 (polynomial constant term = 1)
    __private long result[MAX_POLY_DEGREE];
    for (int i = 0; i < MAX_POLY_DEGREE; i++) result[i] = 0;
    result[0] = 1;

    // Base polynomial (copy to private memory)
    __private long base[MAX_POLY_DEGREE];
    for (int i = 0; i <= base_degree; i++) {
        base[i] = base_coeffs[i] % prime;
    }

    // Square-and-multiply
    while (exp > 0) {
        if (exp & 1) {
            poly_multiply_mod(result, base, mod_poly_coeffs, mod_degree, prime);
        }
        poly_square_mod(base, mod_poly_coeffs, mod_degree, prime);
        exp >>= 1;
    }

    // Write result
    int offset = gid * (mod_degree + 1);
    for (int i = 0; i <= mod_degree; i++) {
        result_coeffs[offset + i] = result[i];
    }
}

/**
 * Helper: Polynomial multiplication mod (modulus_poly, prime)
 */
void poly_multiply_mod(
    __private long *a,
    __private const long *b,
    __global const long *modulus,
    const int mod_degree,
    const long prime
) {
    __private long temp[MAX_POLY_DEGREE * 2];
    for (int i = 0; i < MAX_POLY_DEGREE * 2; i++) temp[i] = 0;

    // Multiply: c[i+j] += a[i] * b[j]
    for (int i = 0; i < mod_degree; i++) {
        for (int j = 0; j < mod_degree; j++) {
            long product = (a[i] * b[j]) % prime;
            temp[i + j] = (temp[i + j] + product) % prime;
        }
    }

    // Reduce mod modulus polynomial
    poly_reduce_mod(temp, modulus, mod_degree, prime);

    // Copy back to a
    for (int i = 0; i < mod_degree; i++) {
        a[i] = temp[i];
    }
}

/**
 * Helper: Polynomial reduction mod (modulus_poly, prime)
 */
void poly_reduce_mod(
    __private long *poly,
    __global const long *modulus,
    const int mod_degree,
    const long prime
) {
    // Division algorithm: repeatedly subtract multiples of modulus
    // Starting from highest degree term
    for (int i = MAX_POLY_DEGREE - 1; i >= mod_degree; i--) {
        if (poly[i] == 0) continue;

        long coeff = poly[i];
        long lead = modulus[mod_degree];
        long lead_inv = mod_inverse(lead, prime);
        long factor = (coeff * lead_inv) % prime;

        // Subtract factor * modulus from poly
        for (int j = 0; j <= mod_degree; j++) {
            long sub = (factor * modulus[j]) % prime;
            poly[i - mod_degree + j] = (poly[i - mod_degree + j] - sub + prime) % prime;
        }
        poly[i] = 0;
    }
}

/**
 * Helper: Modular inverse using Fermat's little theorem
 * a^(-1) ≡ a^(p-2) (mod p) for prime p
 */
long mod_inverse(long a, long prime) {
    // Use binary exponentiation for a^(prime-2) mod prime
    long result = 1;
    long base = a % prime;
    long exp = prime - 2;

    while (exp > 0) {
        if (exp & 1) {
            result = (result * base) % prime;
        }
        base = (base * base) % prime;
        exp >>= 1;
    }

    return result;
}

/**
 * GCD kernel: Compute gcd(poly_a, poly_b) in finite field
 * Uses Euclidean algorithm
 */
__kernel void poly_gcd(
    __global const long *poly_a_coeffs,
    __global const long *poly_b_coeffs,
    const int degree_a,
    const int degree_b,
    __global const long *primes,
    __global long *result_coeffs,       // Output GCD polynomials
    __global int *result_degrees,       // Output degrees
    const int batch_size
) {
    int gid = get_global_id(0);
    if (gid >= batch_size) return;

    long prime = primes[gid];

    // Copy to private memory
    __private long a[MAX_POLY_DEGREE];
    __private long b[MAX_POLY_DEGREE];

    int deg_a = degree_a;
    int deg_b = degree_b;

    for (int i = 0; i <= deg_a; i++) {
        a[i] = poly_a_coeffs[gid * MAX_POLY_DEGREE + i] % prime;
    }
    for (int i = 0; i <= deg_b; i++) {
        b[i] = poly_b_coeffs[gid * MAX_POLY_DEGREE + i] % prime;
    }

    // Euclidean algorithm
    while (deg_b > 0) {
        poly_divide_mod(a, b, &deg_a, &deg_b, prime);

        // Swap a and b
        __private long temp[MAX_POLY_DEGREE];
        for (int i = 0; i < MAX_POLY_DEGREE; i++) {
            temp[i] = a[i];
            a[i] = b[i];
            b[i] = temp[i];
        }
        int temp_deg = deg_a;
        deg_a = deg_b;
        deg_b = temp_deg;
    }

    // Normalize to monic polynomial
    if (deg_a > 0 && a[deg_a] != 0) {
        long lead_inv = mod_inverse(a[deg_a], prime);
        for (int i = 0; i <= deg_a; i++) {
            a[i] = (a[i] * lead_inv) % prime;
        }
    }

    // Write result
    int offset = gid * MAX_POLY_DEGREE;
    for (int i = 0; i <= deg_a; i++) {
        result_coeffs[offset + i] = a[i];
    }
    result_degrees[gid] = deg_a;
}
```

### 3.2 Rust-OpenCL Integration

**Update:** `src/square_root/backends/opencl_backend.rs`

```rust
use ocl::{Buffer, Kernel, flags};

impl OpenClBackend {
    fn test_irreducibility_batch(
        &self,
        primes: &[BigInt],
        polynomial: &Polynomial,
        degree: usize,
        s: &Polynomial,
        polynomial_base: &BigInt,
    ) -> Vec<Option<(BigInt, BigInt)>> {
        let batch_size = primes.len();

        // Convert BigInt to i64 (or handle arbitrary precision)
        let primes_i64: Vec<i64> = primes.iter()
            .map(|p| p.to_i64().expect("Prime too large for GPU"))
            .collect();

        // Prepare polynomial coefficients
        let poly_coeffs = self.polynomial_to_array(polynomial, degree);

        // Create OpenCL buffers
        let primes_buf = Buffer::builder()
            .queue(self.queue.clone())
            .flags(flags::MEM_READ_ONLY)
            .len(batch_size)
            .copy_host_slice(&primes_i64)
            .build()?;

        let poly_buf = Buffer::builder()
            .queue(self.queue.clone())
            .flags(flags::MEM_READ_ONLY)
            .len(poly_coeffs.len())
            .copy_host_slice(&poly_coeffs)
            .build()?;

        let result_buf = Buffer::builder()
            .queue(self.queue.clone())
            .flags(flags::MEM_WRITE_ONLY)
            .len(batch_size * (degree + 1))
            .build()?;

        // Execute kernel
        let kernel = Kernel::builder()
            .program(&self.program)
            .name("poly_mod_exp")
            .queue(self.queue.clone())
            .global_work_size(batch_size)
            .arg(&poly_buf)
            .arg(&(degree as i32))
            .arg(&primes_buf)
            .arg(&result_buf)
            .build()?;

        unsafe { kernel.enq()?; }

        // Read results
        let mut results = vec![0i64; batch_size * (degree + 1)];
        result_buf.read(&mut results).enq()?;

        // Process results and extract irreducible primes
        self.process_gpu_results(&results, primes, degree, s, polynomial_base)
    }

    fn polynomial_to_array(&self, poly: &Polynomial, max_degree: usize) -> Vec<i64> {
        let mut coeffs = vec![0i64; max_degree + 1];
        for i in 0..=max_degree {
            coeffs[i] = poly[i].to_i64().unwrap_or(0);
        }
        coeffs
    }

    fn process_gpu_results(
        &self,
        gpu_output: &[i64],
        primes: &[BigInt],
        degree: usize,
        s: &Polynomial,
        polynomial_base: &BigInt,
    ) -> Vec<Option<(BigInt, BigInt)>> {
        primes.iter().enumerate().map(|(idx, p)| {
            // Extract GCD result for this prime
            let offset = idx * (degree + 1);
            let gcd_coeffs = &gpu_output[offset..offset + degree + 1];

            // Check if GCD = 1 (irreducible)
            let is_irreducible = gcd_coeffs[0] == 1 &&
                                 gcd_coeffs[1..].iter().all(|&c| c == 0);

            if is_irreducible {
                // Compute square root (can also be GPU-accelerated)
                let value = self.compute_square_root(s, p, degree, polynomial_base);
                Some((p.clone(), value))
            } else {
                None
            }
        }).collect()
    }
}
```

### 3.3 Memory Transfer Optimization

**Problem:** CPU ↔ GPU memory transfers are expensive

**Solutions:**
1. **Batch larger groups** - Amortize transfer overhead over 1000+ primes
2. **Pinned memory** - Use page-locked memory for faster transfers
3. **Asynchronous transfers** - Overlap computation and communication

```rust
impl OpenClBackend {
    pub fn test_irreducibility_streaming(
        &self,
        prime_stream: &mut BatchPrimeGenerator,
        total_primes_needed: usize,
    ) -> Vec<(BigInt, BigInt)> {
        const GPU_BATCH_SIZE: usize = 2048;

        let mut results = Vec::new();
        let mut primes_processed = 0;

        // Use double buffering for overlap
        let mut buffer_a = self.allocate_buffer(GPU_BATCH_SIZE);
        let mut buffer_b = self.allocate_buffer(GPU_BATCH_SIZE);

        let mut current_buffer = &mut buffer_a;
        let mut next_buffer = &mut buffer_b;

        while primes_processed < total_primes_needed {
            // Start transfer to GPU while processing previous results
            let next_batch = prime_stream.next_batch(GPU_BATCH_SIZE);
            self.async_transfer_to_gpu(&next_batch, next_buffer);

            // Process current batch on GPU
            let batch_results = self.execute_kernels(current_buffer)?;
            results.extend(batch_results.into_iter().flatten());

            // Swap buffers
            std::mem::swap(&mut current_buffer, &mut next_buffer);
            primes_processed += GPU_BATCH_SIZE;
        }

        results
    }
}
```

### 3.4 Handling Large BigInt Values

**Challenge:** GPU kernels use 64-bit integers; GNFS uses arbitrary-precision BigInt

**Solution:** Multi-precision arithmetic on GPU or hybrid approach

```rust
// Option 1: Limit GPU to primes < 2^63
impl OpenClBackend {
    fn can_use_gpu(&self, prime: &BigInt) -> bool {
        prime.bits() <= 63
    }

    fn test_irreducibility_batch(&self, primes: &[BigInt], ...) {
        // Split into GPU-compatible and CPU-only
        let (gpu_primes, cpu_primes): (Vec<_>, Vec<_>) = primes.iter()
            .partition(|p| self.can_use_gpu(p));

        // Process GPU batch
        let gpu_results = self.gpu_kernel_dispatch(&gpu_primes, ...);

        // Process CPU batch with Rayon
        let cpu_results = self.cpu_fallback.test_irreducibility_batch(&cpu_primes, ...);

        // Merge results
        merge_results(gpu_results, cpu_results)
    }
}

// Option 2: Multi-precision GPU library (e.g., CGBN - CUDA only)
// Skip for OpenCL implementation; recommend CUDA in Phase 4
```

### Performance Estimates

**Hardware:** NVIDIA RTX 3080 (8704 CUDA cores)

Theoretical concurrent operations:
- CPU (16 threads): 16 primes simultaneously
- GPU (8704 cores): 1024+ primes simultaneously (limited by memory)

**Expected speedup over Phase 1:**
- Small numbers (< 30 digits): 2-5x (overhead dominates)
- Medium numbers (30-50 digits): 10-20x
- Large numbers (> 50 digits): 20-100x

---

## PHASE 4: Advanced Optimizations

**Timeline:** 2-3 weeks
**Expected Speedup:** 1.5-3x over Phase 3
**Complexity:** High
**Risk:** Low (optional enhancements)

### 4.1 Kernel Parameter Tuning

**Work Group Size Optimization:**

```rust
impl OpenClBackend {
    fn auto_tune_workgroup_size(&mut self) -> usize {
        let device = self.queue.device();
        let max_workgroup = device.max_wg_size()?;

        // Test different sizes
        let candidates = vec![32, 64, 128, 256, 512];
        let test_batch_size = 10000;

        let mut best_size = 64;
        let mut best_time = std::time::Duration::MAX;

        for &size in &candidates {
            if size > max_workgroup { break; }

            let start = std::time::Instant::now();
            self.test_with_workgroup_size(size, test_batch_size);
            let elapsed = start.elapsed();

            info!("Workgroup size {}: {:?}", size, elapsed);

            if elapsed < best_time {
                best_time = elapsed;
                best_size = size;
            }
        }

        info!("Optimal workgroup size: {}", best_size);
        self.workgroup_size = best_size;
        best_size
    }
}
```

### 4.2 Multi-GPU Support

```rust
pub struct MultiGpuBackend {
    gpus: Vec<OpenClBackend>,
}

impl MultiGpuBackend {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let platform = Platform::default();
        let devices = Device::list_all(platform)?;

        let gpus: Vec<_> = devices.into_iter()
            .filter_map(|dev| OpenClBackend::with_device(dev).ok())
            .collect();

        info!("Detected {} GPUs", gpus.len());
        Ok(MultiGpuBackend { gpus })
    }
}

impl SquareRootBackend for MultiGpuBackend {
    fn test_irreducibility_batch(&self, primes: &[BigInt], ...) -> Vec<Option<...>> {
        let chunk_size = primes.len() / self.gpus.len();

        // Distribute work across GPUs using Rayon
        let results: Vec<_> = primes.par_chunks(chunk_size)
            .enumerate()
            .flat_map(|(gpu_id, chunk)| {
                self.gpus[gpu_id].test_irreducibility_batch(chunk, ...)
            })
            .collect();

        results
    }
}
```

### 4.3 Hybrid CPU+GPU Scheduling

**Adaptive work distribution based on problem size:**

```rust
pub struct HybridBackend {
    cpu: Arc<CpuBackend>,
    gpu: Arc<dyn SquareRootBackend>,
    scheduler: AdaptiveScheduler,
}

impl HybridBackend {
    fn test_irreducibility_batch(&self, primes: &[BigInt], ...) {
        let (cpu_ratio, gpu_ratio) = self.scheduler.get_ratios();

        let split_point = (primes.len() as f32 * cpu_ratio) as usize;
        let (cpu_primes, gpu_primes) = primes.split_at(split_point);

        // Execute in parallel
        let (cpu_results, gpu_results) = rayon::join(
            || self.cpu.test_irreducibility_batch(cpu_primes, ...),
            || self.gpu.test_irreducibility_batch(gpu_primes, ...)
        );

        // Update scheduler based on performance
        self.scheduler.update(cpu_results.1, gpu_results.1);

        merge_results(cpu_results.0, gpu_results.0)
    }
}

struct AdaptiveScheduler {
    cpu_performance: f32,  // primes per second
    gpu_performance: f32,
}

impl AdaptiveScheduler {
    fn get_ratios(&self) -> (f32, f32) {
        let total = self.cpu_performance + self.gpu_performance;
        (self.cpu_performance / total, self.gpu_performance / total)
    }
}
```

### 4.4 Precomputation and Caching

```rust
pub struct PrecomputeCache {
    // Cache polynomial operations that repeat across primes
    derivative_squared: Polynomial,
    monic_polynomial: Polynomial,

    // Cache prime test results (for retry scenarios)
    tested_primes: HashMap<BigInt, bool>,
}

impl PrecomputeCache {
    pub fn new(polynomial: &Polynomial) -> Self {
        let derivative = polynomial.get_derivative_polynomial();
        let derivative_squared = Polynomial::square(&derivative);
        let monic_polynomial = Polynomial::make_monic(polynomial, &BigInt::zero());

        PrecomputeCache {
            derivative_squared,
            monic_polynomial,
            tested_primes: HashMap::new(),
        }
    }
}
```

---

## Configuration Reference

### Environment Variables

```bash
# Phase 1: CPU Parallelization
export GNFS_STAGE4_BATCH_SIZE=64        # Primes per batch (default: 4 * num_cpus)
export RAYON_NUM_THREADS=16             # CPU threads (default: all cores)

# Phase 2: Backend Selection
export GNFS_USE_CPU_ONLY=0              # Force CPU backend (default: 0 = auto-detect GPU)
export GNFS_OPENCL_DEVICE=0             # GPU device index (default: 0)

# Phase 3: GPU Parameters
export GNFS_GPU_BATCH_SIZE=2048         # Primes per GPU batch (default: 2048)
export GNFS_OPENCL_WORKGROUP_SIZE=256   # OpenCL work group size (default: auto-tune)

# Phase 4: Advanced
export GNFS_USE_MULTI_GPU=1             # Enable multi-GPU (default: 0)
export GNFS_HYBRID_CPU_RATIO=0.1        # CPU work ratio in hybrid mode (default: 0.1)
```

### Runtime Logging

```bash
# Enable detailed performance logging
export RUST_LOG=gnfs::square_root=debug

# Example output:
# [INFO] Using backend: GPU (OpenCL)
# [INFO] OpenCL device: NVIDIA GeForce RTX 3080 (8704 cores)
# [DEBUG] GPU batch size: 2048 primes
# [DEBUG] Tested 10000 primes in 1.2s (8333 primes/sec)
# [INFO] Found 5 irreducible primes
```

---

## Testing and Validation Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_gpu_equivalence() {
        let cpu_backend = CpuBackend::new();
        let gpu_backend = OpenClBackend::new().unwrap();

        let test_primes = generate_test_primes(100);
        let test_poly = Polynomial::parse("X^3 + 2X^2 - 3X + 1");

        let cpu_results = cpu_backend.test_irreducibility_batch(
            &test_primes, &test_poly, 3, &s, &m
        );
        let gpu_results = gpu_backend.test_irreducibility_batch(
            &test_primes, &test_poly, 3, &s, &m
        );

        assert_eq!(cpu_results, gpu_results);
    }

    #[test]
    fn test_batch_prime_generation_deterministic() {
        let mut gen1 = BatchPrimeGenerator::new(1000);
        let mut gen2 = BatchPrimeGenerator::new(1000);

        assert_eq!(gen1.next_batch(100), gen2.next_batch(100));
    }

    #[test]
    fn test_irreducibility_known_primes() {
        // Test against known irreducible primes for specific polynomials
        let f = Polynomial::parse("X^3 + X + 1");
        // Known: p=2 is irreducible for this polynomial
        let result = test_single_prime(&BigInt::from(2), &f, ...);
        assert!(result.is_some());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_square_root_extraction() {
    // End-to-end test with known factorization
    let cancel_token = CancellationToken::new();
    let n = BigInt::from(45113); // = 167 * 270
    let m = BigInt::from(31);

    let mut gnfs = GNFS::new(&cancel_token, &n, &m, 3, &BigInt::from(100), 20, 1000, true);
    let result = SquareFinder::solve(&cancel_token, &mut gnfs);

    assert!(result);
    assert_eq!(gnfs.factorization.unwrap(), Solution::new(&BigInt::from(167), &BigInt::from(270)));
}
```

### Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_backends(c: &mut Criterion) {
    let mut group = c.benchmark_group("irreducibility_test");

    let test_primes = generate_test_primes(1000);
    let test_poly = Polynomial::parse("X^5 + 3X^3 - 2X + 1");

    group.bench_function("cpu_sequential", |b| {
        b.iter(|| {
            test_primes.iter().map(|p| test_single_prime(black_box(p), &test_poly, ...))
        })
    });

    group.bench_function("cpu_parallel", |b| {
        let backend = CpuBackend::new();
        b.iter(|| backend.test_irreducibility_batch(black_box(&test_primes), &test_poly, ...))
    });

    group.bench_function("gpu_opencl", |b| {
        let backend = OpenClBackend::new().unwrap();
        b.iter(|| backend.test_irreducibility_batch(black_box(&test_primes), &test_poly, ...))
    });

    group.finish();
}

criterion_group!(benches, benchmark_backends);
criterion_main!(benches);
```

---

## Risk Mitigation

### Phase 1 Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Race conditions in shared state | High | Use `Arc<Mutex<>>` for all shared collections |
| Incorrect parallelization of dependent operations | High | Ensure prime tests are truly independent; validate with tests |
| Performance worse than sequential | Medium | Benchmark different batch sizes; add adaptive sizing |

### Phase 2 Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| OpenCL not available on target system | Medium | Implement robust CPU fallback |
| Backend trait abstraction overhead | Low | Use `Arc<dyn Trait>` for zero-cost dispatch |

### Phase 3 Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| GPU kernel bugs difficult to debug | High | Extensive testing; compare against CPU reference |
| BigInt overflow in GPU kernels | High | Limit GPU to 63-bit primes; use CPU for larger |
| Memory transfer bottleneck | Medium | Implement streaming with double buffering |
| OpenCL portability issues | Medium | Test on multiple vendors (NVIDIA, AMD, Intel) |

### Phase 4 Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Multi-GPU synchronization overhead | Low | Use separate queues per GPU; no synchronization needed |
| Auto-tuning adds startup latency | Low | Cache tuning results; allow manual override |

---

## Performance Estimates Summary

**Test Case:** Factor a 20-digit composite number (polynomial degree 3)

| Phase | Backend | Primes Tested | Time | Speedup |
|-------|---------|---------------|------|---------|
| Baseline | Sequential CPU | 5,000 | 30 min | 1x |
| Phase 1 | Rayon (16 threads) | 5,000 | 3-5 min | 6-10x |
| Phase 3 | GPU (RTX 3080) | 5,000 | 30-60 sec | 30-60x |
| Phase 4 | Multi-GPU (2x RTX 3080) | 5,000 | 20-40 sec | 45-90x |

**Test Case:** Factor a 50-digit composite number (polynomial degree 5)

| Phase | Backend | Primes Tested | Time | Speedup |
|-------|---------|---------------|------|---------|
| Baseline | Sequential CPU | 50,000 | ~8 hours | 1x |
| Phase 1 | Rayon (16 threads) | 50,000 | ~1 hour | 8x |
| Phase 3 | GPU (RTX 3080) | 50,000 | ~5-10 min | 50-100x |

---

## Recommended Implementation Path

### Minimum Viable Product (MVP): Phase 1 Only
- **Timeline:** 1-2 weeks
- **Effort:** Low
- **Benefit:** 5-10x speedup, immediate impact
- **Risk:** Low
- **Recommendation:** Start here for quick wins

### Standard Implementation: Phases 1-3
- **Timeline:** 6-9 weeks
- **Effort:** Medium-High
- **Benefit:** 20-60x speedup
- **Risk:** Medium
- **Recommendation:** Best balance of effort and performance

### Full Implementation: Phases 1-4
- **Timeline:** 8-12 weeks
- **Effort:** High
- **Benefit:** 30-100x speedup, production-ready
- **Risk:** Medium
- **Recommendation:** For production use or research applications

---

## Next Steps

### Immediate Actions

1. **Week 1: Phase 1 Implementation**
   - Create `BatchPrimeGenerator` struct
   - Refactor `calculate_algebraic_side()` for Rayon parallelization
   - Add environment variable configuration
   - Write unit tests

2. **Week 2: Phase 1 Validation**
   - Integration testing with full GNFS pipeline
   - Benchmark on multiple input sizes
   - Optimize batch size parameter
   - Document performance gains

3. **Week 3-4: Phase 2 Foundation** (Optional)
   - Add OpenCL dependencies
   - Create backend trait and CPU implementation
   - Implement backend selection logic

### Long-term Roadmap

- **Q1:** Phase 1 complete and validated
- **Q2:** Phase 2-3 implementation (GPU acceleration)
- **Q3:** Phase 4 optimizations (multi-GPU, hybrid scheduling)
- **Q4:** Production hardening, comprehensive benchmarking

---

## Appendix A: Code Architecture Diagram

```
square_root/
├── square_finder.rs           # Main interface (modified for Phase 1+)
├── batch_prime_generator.rs   # Phase 1: Batch prime generation
├── backend_selector.rs         # Phase 2: Backend selection logic
└── backends/
    ├── mod.rs                  # Backend trait definition
    ├── cpu_backend.rs          # Phase 1: Rayon-parallel CPU backend
    ├── opencl_backend.rs       # Phase 2-3: GPU backend
    └── kernels/
        └── polynomial_ops.cl   # Phase 3: OpenCL kernels

Core changes to existing files:
- square_finder.rs: Replace sequential loop with backend dispatch
- finite_field_arithmetic.rs: Extract reusable components
- polynomial.rs: Ensure thread-safe operations
```

---

## Appendix B: Alternative Technologies

### CUDA vs OpenCL

**CUDA Advantages:**
- Better performance (5-15% faster)
- More mature ecosystem
- Better debugging tools (cuda-gdb, Nsight)
- Libraries: CGBN for multi-precision arithmetic

**OpenCL Advantages:**
- Cross-platform (NVIDIA, AMD, Intel, Apple)
- Works on CPU as fallback
- No vendor lock-in

**Recommendation:** Start with OpenCL (Phase 2-3), optionally add CUDA backend in Phase 4 for NVIDIA users

### Rust GPU Libraries Comparison

| Library | Pros | Cons | Verdict |
|---------|------|------|---------|
| `ocl` | Mature, well-documented | OpenCL only | **Recommended** |
| `vulkano` | Modern Vulkan API | Compute shaders complex | Future consideration |
| `wgpu` | WebGPU standard, cross-platform | Immature for compute | Phase 4+ |
| `rust-cuda` | Native CUDA | NVIDIA only | Phase 4 optional |

---

## Appendix C: References and Resources

### GNFS Algorithm
- Pomerance, C. (1996). "A Tale of Two Sieves"
- Buhler, J.P., et al. (1993). "Factoring Integers with the Number Field Sieve"

### GPU Compute
- OpenCL 2.0 Specification: https://www.khronos.org/opencl/
- Rust `ocl` crate documentation: https://docs.rs/ocl/
- "CUDA by Example" (Sanders & Kandrot) - applicable to OpenCL

### Polynomial Arithmetic on GPU
- Bajard, J.C., et al. (2009). "Polynomial Multiplication over Finite Fields on GPUs"
- Bernstein, D.J. (2007). "Fast multiplication and its applications"

### Existing Implementations
- AdamWhiteHat GNFS (C#): https://github.com/AdamWhiteHat/GNFS
- CADO-NFS (C++): https://gitlab.inria.fr/cado-nfs/cado-nfs

---

## Document Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-21 | Claude Code | Initial comprehensive implementation plan |

---

**End of Document**
