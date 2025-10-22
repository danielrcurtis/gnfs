# Square Root Performance Instrumentation

## Overview

Added detailed timing instrumentation to the `square_root()` function in `/Users/danielcurtis/source/gnfs/src/square_root/finite_field_arithmetic.rs` to identify performance bottlenecks in Stage 4 of GNFS factorization.

## Problem Context

Previous profiling showed that Stage 4 (square root extraction) was taking ~82 seconds per irreducible prime. After optimizing `Legendre::symbol_search()` (which gave a 24% speedup from 107s → 82s), we needed to identify the next major bottleneck.

Based on code analysis, the suspected bottleneck was `Polynomial::exponentiate_mod()` at line 31 (now line 56) of the square_root function.

## Instrumentation Added

### 1. Function Entry Logging

```rust
let function_start = Instant::now();
info!("square_root() ENTRY: p={}, degree={}, m={}", p, degree, m);
info!("  start_polynomial degree: {}, f degree: {}", start_polynomial.degree(), f.degree());
```

**Purpose**: Log the input parameters to understand what values are being processed.

### 2. q = p.pow(degree) Timing (Line 18-22)

```rust
let q_start = Instant::now();
let q = p.pow(degree as u32);
let q_elapsed = q_start.elapsed();
info!("  q = p.pow(degree) took: {:.3?}", q_elapsed);
info!("  q value: {}", q);
```

**Purpose**: Time the computation of q = p^degree (field size).

### 3. Parameter Computation Logging (Line 31, 39)

```rust
info!("  r={}, s={}", r, s);
info!("  half_s={}", half_s);
```

**Purpose**: Log intermediate values for debugging and understanding algorithm state.

### 4. Legendre::symbol_search() Timing (Line 42-45)

```rust
let legendre_start = Instant::now();
let quadratic_non_residue = Legendre::symbol_search(&(m + 1), &q, &BigInt::from(-1));
let legendre_elapsed = legendre_start.elapsed();
info!("  Legendre::symbol_search() took: {:.3?}", legendre_elapsed);
```

**Purpose**: Measure time spent finding a quadratic non-residue. This was previously optimized but we're still tracking it.

### 5. theta.modpow() Timing (Line 49-52)

```rust
let minus_one_start = Instant::now();
let minus_one = theta.modpow(&((&q - 1) / 2), p);
let minus_one_elapsed = minus_one_start.elapsed();
info!("  theta.modpow() (minus_one) took: {:.3?}", minus_one_elapsed);
```

**Purpose**: Time the modular exponentiation to compute minus_one.

### 6. **Polynomial::exponentiate_mod() Timing (Line 55-58) - SUSPECTED BOTTLENECK**

```rust
let exp_mod_start = Instant::now();
let mut omega_poly = Polynomial::exponentiate_mod(start_polynomial, &half_s, f, p);
let exp_mod_elapsed = exp_mod_start.elapsed();
info!("  Polynomial::exponentiate_mod() took: {:.3?} *** MAJOR OPERATION ***", exp_mod_elapsed);
```

**Purpose**: **This is the suspected bottleneck**. Polynomial exponentiation with potentially huge exponents (half_s can be very large for large primes). This operation performs binary exponentiation which requires O(log(half_s)) polynomial multiplications, each of which is expensive.

### 7. Loop Instrumentation (Line 63-103)

The loop that iterates up to `r` times has comprehensive per-iteration timing:

```rust
let loop_start = Instant::now();
let mut total_zeta_time = std::time::Duration::ZERO;
let mut total_lambda_time = std::time::Duration::ZERO;
let mut total_multiply_time = std::time::Duration::ZERO;

loop {
    i += 1;
    let iteration_start = Instant::now();

    // Line 73-77: theta.modpow() for zeta
    let zeta_start = Instant::now();
    let zeta = theta.modpow(&(&i * &s), p);
    let zeta_elapsed = zeta_start.elapsed();
    total_zeta_time += zeta_elapsed;

    // Line 80-83: lambda update
    let lambda_start = Instant::now();
    lambda = (&lambda * &zeta.pow((2u32.pow((r - i) as u32)) as u32)).mod_floor(p);
    let lambda_elapsed = lambda_start.elapsed();
    total_lambda_time += lambda_elapsed;

    // Line 86-89: Polynomial::multiply()
    let multiply_start = Instant::now();
    omega_poly = Polynomial::multiply(&omega_poly, &Polynomial::from_term(zeta.pow(2u32.pow((r - i - 1) as u32) as u32), 0));
    let multiply_elapsed = multiply_start.elapsed();
    total_multiply_time += multiply_elapsed;

    let iteration_elapsed = iteration_start.elapsed();
    info!("    Loop iteration {}: total={:.3?}, zeta={:.3?}, lambda={:.3?}, multiply={:.3?}",
          i, iteration_elapsed, zeta_elapsed, lambda_elapsed, multiply_elapsed);

    if lambda == BigInt::one() || i > r {
        break;
    }
}

let loop_elapsed = loop_start.elapsed();
info!("  Loop completed: {} iterations, total_time={:.3?}", i, loop_elapsed);
info!("    Loop breakdown: zeta_total={:.3?}, lambda_total={:.3?}, multiply_total={:.3?}",
      total_zeta_time, total_lambda_time, total_multiply_time);
```

**Purpose**:
- Track how many loop iterations occur
- Measure time for each operation within the loop:
  - `zeta` computation (modpow)
  - `lambda` update (exponentiation and modular multiplication)
  - `omega_poly` update (polynomial multiplication)
- Aggregate timing to see which operation dominates within the loop

### 8. Function Exit Logging (Line 105-106)

```rust
let function_elapsed = function_start.elapsed();
info!("square_root() EXIT: total time={:.3?}", function_elapsed);
```

**Purpose**: Report total function execution time.

## Expected Output

When the program reaches Stage 4 and calls `square_root()` for an irreducible prime, you will see output like:

```
[INFO] square_root() ENTRY: p=433, degree=3, m=31
[INFO]   start_polynomial degree: 1, f degree: 3
[INFO]   q = p.pow(degree) took: 15.234µs
[INFO]   q value: 81182897
[INFO]   r=2, s=20295724
[INFO]   half_s=10147862
[INFO]   Legendre::symbol_search() took: 1.234ms
[INFO]   theta.modpow() (minus_one) took: 234.567µs
[INFO]   Polynomial::exponentiate_mod() took: 79.456s *** MAJOR OPERATION ***
[INFO]     Loop iteration 1: total=1.234ms, zeta=234µs, lambda=456µs, multiply=544µs
[INFO]     Loop iteration 2: total=1.123ms, zeta=212µs, lambda=445µs, multiply=466µs
[INFO]   Loop completed: 2 iterations, total_time=2.357ms
[INFO]     Loop breakdown: zeta_total=446µs, lambda_total=901µs, multiply_total=1.010ms
[INFO] square_root() EXIT: total time=79.460s
```

## Analysis Strategy

Once you have the output, compare the timings:

1. **If `Polynomial::exponentiate_mod()` takes 70-80+ seconds**:
   - The bottleneck is confirmed
   - Next step: Optimize polynomial exponentiation (investigate binary exponentiation implementation, consider caching, or use different algorithm)

2. **If the loop takes significant time**:
   - Check if loop iterations are high (r should typically be small, often 1-5)
   - Identify which operation within the loop dominates (zeta, lambda, or multiply)
   - Focus optimization on that operation

3. **If Legendre::symbol_search() is still slow**:
   - The previous optimization may not have been effective for all cases
   - May need additional optimization

4. **If theta.modpow() is slow**:
   - BigInt modular exponentiation may need optimization
   - Consider using a different library or implementation

## Files Modified

- `/Users/danielcurtis/source/gnfs/src/square_root/finite_field_arithmetic.rs`
  - Added imports: `use std::time::Instant;` and `use log::info;`
  - Added comprehensive timing instrumentation throughout `square_root()` function

## Testing Instructions

To see the instrumentation output:

```bash
# Clean any old state
rm -rf 143

# Run with 4 threads
env GNFS_THREADS=4 ./target/release/gnfs 143
```

Wait for the program to reach Stage 4 (square root extraction). The timing information will appear in the logs when `square_root()` is called for each irreducible prime.

## Next Steps After Analysis

Based on the timing results:

1. **Identify the dominant operation** (expected: `Polynomial::exponentiate_mod()`)
2. **Profile that specific function** - add similar instrumentation to understand its internals
3. **Implement optimizations**:
   - For polynomial exponentiation: Consider Montgomery multiplication, precomputation, or algorithmic improvements
   - For loop operations: Consider parallelization or algorithmic changes
   - For BigInt operations: Consider alternative libraries (e.g., GMP via `rug` crate)

## Code Location

All instrumentation is in:
- **File**: `/Users/danielcurtis/source/gnfs/src/square_root/finite_field_arithmetic.rs`
- **Function**: `square_root()` (lines 12-109)
- **Calling Context**: `/Users/danielcurtis/source/gnfs/src/square_root/square_finder.rs` line 327

The function is called during Stage 4 when testing irreducible primes to compute square roots in finite fields for the Chinese Remainder Theorem reconstruction.
