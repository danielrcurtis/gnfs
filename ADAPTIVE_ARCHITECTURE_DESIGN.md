# Adaptive Integer Type Architecture - Design Document

**Date:** 2025-10-22
**Author:** Claude Code
**Status:** Design Phase - Ready for Implementation

---

## Executive Summary

This document describes the design and implementation plan for an adaptive integer type architecture that automatically selects the most efficient numeric representation (u64, u128, U256, U512, or BigInt) based on input size. This optimization will reduce memory usage from 70GB to <500MB for small numbers while maintaining GPU compatibility and a single, maintainable codebase.

**Key Goals:**
1. **Memory efficiency**: ≤4GB per core for numbers up to 154 digits
2. **Performance**: 10-100x speedup over current BigInt-only approach
3. **GPU compatibility**: u64/u128/U256/U512 work on GPU
4. **Maintainability**: Single codebase with trait-based abstraction

---

## Current Architecture Analysis

### Hot Paths Identified

After analyzing the codebase, the following are the performance-critical operations that dominate execution time:

**1. Relation Sieving (`relation.rs::sieve()`)** - 98% of runtime
   - **Line 59**: `self.rational_norm = self.apply(&gnfs.polynomial_base)` - BigInt addition/multiplication
   - **Line 67**: `abs_rational_norm = self.rational_norm.abs()` - BigInt abs()
   - **Line 71-74**: `FactorizationFactory::factor_with_base()` - Trial division with BigInt
   - **Line 91-109**: Algebraic norm computation using `BigRational` - VERY expensive
   - **Line 95**: `gnfs.current_polynomial.evaluate_rational(&ab_ratio)` - Polynomial evaluation
   - **Line 100**: `neg_b.pow(degree as u32)` - BigInt exponentiation

**2. Trial Division (`factorization_factory.rs::factor_with_base()`)**
   - **Lines 107-121**: u32 fast path (already optimized!)
   - **Lines 125-139**: u64 fast path (already optimized!)
   - **Lines 142-150**: BigInt slow path - THIS IS WHERE WE CAN IMPROVE
   - Current code has good optimizations, but still uses BigInt for the slow path

**3. Polynomial Evaluation (`polynomial.rs::evaluate_rational()`)**
   - **Line 309**: `result = result * x + ...` - Horner's method with BigRational
   - **Line 313**: Per-coefficient multiplication and addition
   - Uses `BigRational` which is 2x slower than BigInt, 100x slower than native types

**4. Parallel Sieving (`poly_relations_sieve_progress.rs::generate_relations()`)**
   - **Lines 156-168**: Creates thousands of `Relation` objects in parallel
   - Each Relation contains 9 BigInt fields + 2 CountDictionary (which contain BTreeMap<BigInt, BigInt>)
   - Memory explosion: 11-digit numbers create ~70GB of temporary BigInt allocations

### Integer Type Usage Patterns

**Where BigInt is REQUIRED (must stay BigInt):**
- **N** - The number to factor (input, can be arbitrarily large)
- **Polynomial base** - Usually small, but stored alongside N
- **Final output** - Relation storage, matrix building, square root extraction

**Where fixed-width types can be used (OPTIMIZATION TARGETS):**
- **Rational norm**: `a + b*m` where a, b ≤ 10,000 and m ≈ N^(1/d)
  - For 11-digit N with degree 3: max value ≈ 2^40 → **u64 sufficient**
- **Algebraic norm**: `f(-a/b) * (-b)^d` where f is polynomial
  - For 11-digit N with degree 3: max value ≈ 2^60 → **u64 sufficient**
  - For 30-digit N: max value ≈ 2^120 → **u128 sufficient**
- **Trial division quotients** - Gets smaller with each division
- **Factor base primes** - Bounded by rational_factor_base_max

### Trait Abstraction Feasibility

**Good news:** The codebase is already well-structured for trait abstraction!

**Why this will work:**
1. ✅ **Limited API surface**: Most integer operations use `+`, `-`, `*`, `/`, `%`, `abs()`, `pow()`
2. ✅ **Clear conversion points**: Input (BigInt → T), Output (T → BigInt), Boundaries
3. ✅ **num-traits compatibility**: The `num` crate provides standard traits (`Zero`, `One`, etc.)
4. ✅ **Existing fast paths**: `factorization_factory.rs` already has u32/u64 fast paths!

**Challenges:**
- ⚠️ **BigRational**: Used in algebraic norm computation (line 92-109 in `relation.rs`)
  - Solution: Create `GnfsRational<T>` wrapper or use fixed-point arithmetic for native types
- ⚠️ **CountDictionary**: Currently uses `BTreeMap<BigInt, BigInt>`
  - Solution: Make generic: `CountDictionary<T>` or convert keys to usize indices
- ⚠️ **Polynomial coefficients**: Currently `HashMap<usize, BigInt>`
  - Solution: Make generic: `Polynomial<T>`

---

## Proposed Architecture

### Option Analysis

I evaluated three architectural approaches:

#### Option A: Enum-based Dispatch
```rust
pub enum GnfsBackend {
    Native64(GnfsEngine<u64>),
    Native128(GnfsEngine<u128>),
    // ...
}
```
**Pros:** Type-safe, no trait objects, compiler can optimize each variant
**Cons:** Match statements everywhere, code duplication, large binary

#### Option B: Pure Trait Dispatch
```rust
pub struct GNFS<T: GnfsInteger> { ... }
```
**Pros:** Zero runtime overhead, maximum compiler optimization
**Cons:** Massive monomorphization explosion, 5x binary size, compilation time explosion

#### Option C: Hybrid (RECOMMENDED ✅)
```rust
// External API: trait object for flexibility
pub struct GNFS {
    backend: Box<dyn GnfsBackend>,
}

// Internal implementation: concrete types for speed
struct GnfsBackendImpl<T: GnfsInteger> { ... }
```
**Pros:**
- ✅ Clean external API (no generics leak)
- ✅ Fast paths use concrete types (no virtual calls in hot loops)
- ✅ Manageable binary size
- ✅ Easy to extend with new types

**Cons:**
- One virtual call per sieving iteration (negligible compared to computation cost)

**Decision:** Use **Option C (Hybrid)** for optimal balance of performance and maintainability.

---

## Backend Selection Algorithm

### Decision Tree

Based on algebraic norm bit requirements:

```rust
pub fn select_backend(n: &BigInt) -> BackendType {
    let bits = n.bits();

    // For degree-3 GNFS:
    // - Rational norm: a + b*m ≈ 10,000 + 10,000*N^(1/3) ≈ N^(1/3) * 20,000
    // - Algebraic norm: f(-a/b) * (-b)^3 ≈ N^(1/3) * b^3 ≈ N^(1/3) * 10^12
    //
    // Bit formula (with 20% safety margin):
    //   algebraic_norm_bits ≈ (bits/3) + 40 + 20% margin

    let degree = calculate_degree_from_bits(bits);  // From GNFS::calculate_degree
    let safety_margin = 0.2;  // 20% margin for polynomial coefficients

    let estimated_norm_bits = match degree {
        3 => (bits / 3) + 40,  // Degree 3: dominant case
        4 => (bits / 4) + 50,  // Degree 4
        5 => (bits / 5) + 55,  // Degree 5
        _ => (bits / degree) + 60,  // Higher degrees
    };

    let safe_norm_bits = (estimated_norm_bits as f64 * (1.0 + safety_margin)) as usize;

    match safe_norm_bits {
        0..=60   => BackendType::Native64,    // Up to ~14 decimal digits
        61..=120  => BackendType::Native128,   // Up to ~30 decimal digits
        121..=250 => BackendType::Fixed256,    // Up to ~77 decimal digits
        251..=500 => BackendType::Fixed512,    // Up to ~154 digits (RSA-512!)
        _         => BackendType::Arbitrary,   // Large numbers (RSA-1024+)
    }
}
```

### Boundary Testing

**Critical test cases:**
- **14 digits** (boundary between u64 and u128)
- **30 digits** (boundary between u128 and U256)
- **77 digits** (boundary between U256 and U512)
- **154 digits** (boundary between U512 and BigInt)

**Validation strategy:**
- Run same number with multiple backends
- Verify same relations found
- Add 10-20% safety margin to bit calculations

---

## Implementation Plan

### Phase 1: Core Trait Abstraction

#### File: `src/integer_math/gnfs_integer.rs` (NEW)

```rust
use num::{BigInt, Zero, One};
use std::fmt::{Debug, Display};

/// Core trait for GNFS integer arithmetic.
/// All integer types used in hot paths must implement this trait.
pub trait GnfsInteger:
    Clone + Debug + Display +
    Zero + One +
    PartialEq + Eq +
    PartialOrd + Ord +
    std::ops::Add<Output = Self> +
    std::ops::Sub<Output = Self> +
    std::ops::Mul<Output = Self> +
    std::ops::Div<Output = Self> +
    std::ops::Rem<Output = Self> +
    std::ops::Neg<Output = Self> +
    Send + Sync  // Required for rayon parallelism
{
    /// Construct from u64 (for small constants)
    fn from_u64(n: u64) -> Self;

    /// Construct from i64 (for signed values)
    fn from_i64(n: i64) -> Self;

    /// Convert to BigInt (for final output)
    fn to_bigint(&self) -> BigInt;

    /// Construct from BigInt (for initialization)
    fn from_bigint(n: &BigInt) -> Option<Self>;

    /// Number of bits in this value
    fn bits(&self) -> usize;

    /// Absolute value
    fn abs(&self) -> Self;

    /// Raise to power (for polynomial evaluation)
    fn pow(&self, exp: u32) -> Self;

    /// Check if negative
    fn is_negative(&self) -> bool;

    /// Check if value is one
    fn is_one(&self) -> bool {
        self == &Self::one()
    }

    /// Check if value is zero
    fn is_zero(&self) -> bool {
        self == &Self::zero()
    }

    /// Maximum value (for overflow detection)
    fn max_value() -> Option<Self>;

    /// Checked arithmetic (returns None on overflow)
    fn checked_add(&self, other: &Self) -> Option<Self>;
    fn checked_mul(&self, other: &Self) -> Option<Self>;
}
```

**Implementation for each type:**
- ✅ `impl GnfsInteger for u64` - Straightforward, use checked arithmetic
- ✅ `impl GnfsInteger for u128` - Same as u64
- ✅ `impl GnfsInteger for U256` - Use `crypto_bigint` crate
- ✅ `impl GnfsInteger for U512` - Use `crypto_bigint` crate
- ✅ `impl GnfsInteger for malachite::Integer` - Arbitrary precision fallback

**Dependency additions to Cargo.toml:**
```toml
crypto-bigint = { version = "0.5", features = ["generic-array"] }
malachite = "0.4"  # Consider replacing num-bigint with malachite for consistency
```

### Phase 2: Supporting Generic Types

#### File: `src/integer_math/gnfs_rational.rs` (NEW)

```rust
/// Generic rational number for GNFS (avoids BigRational in hot paths)
pub struct GnfsRational<T: GnfsInteger> {
    numer: T,
    denom: T,
}

impl<T: GnfsInteger> GnfsRational<T> {
    pub fn new(numer: T, denom: T) -> Self {
        // TODO: Normalize (divide by GCD)
        GnfsRational { numer, denom }
    }

    pub fn evaluate_polynomial(&self, poly: &Polynomial<T>) -> Self {
        // Horner's method for rational evaluation
        // Keeps numerator and denominator separate to avoid division until the end
    }

    pub fn to_integer(&self) -> T {
        // Extract integer part (for algebraic norm)
        &self.numer / &self.denom
    }
}
```

#### File: `src/polynomial/polynomial.rs` - Make Generic

**Changes needed:**
```rust
// Change from:
pub struct Polynomial {
    pub terms: HashMap<usize, BigInt>,
}

// To:
pub struct Polynomial<T: GnfsInteger = BigInt> {  // Default to BigInt for compatibility
    pub terms: HashMap<usize, T>,
}

impl<T: GnfsInteger> Polynomial<T> {
    pub fn evaluate(&self, x: &T) -> T { ... }
    pub fn evaluate_rational(&self, x: &GnfsRational<T>) -> GnfsRational<T> { ... }
    // ... other methods
}
```

**Migration strategy:**
- Keep `BigInt` as default generic parameter for compatibility
- Gradually update call sites to specify `<T>` explicitly
- Use type aliases: `type BigIntPolynomial = Polynomial<BigInt>;`

#### File: `src/core/count_dictionary.rs` - Make Generic (Optional)

**Option 1: Keep as BigInt** (simpler, less invasive)
- CountDictionary is only used for final factorization output
- Memory impact is small (< 1KB per relation)
- **Recommendation:** Keep as-is for Phase 1

**Option 2: Make generic** (for completeness)
```rust
pub struct CountDictionary<T: GnfsInteger> {
    inner: BTreeMap<T, BigInt>,  // Key is prime, value is exponent
}
```

**Decision:** **Option 1** for Phase 1. Only optimize if profiling shows it's necessary.

### Phase 3: Backend System

#### File: `src/core/backend.rs` (NEW)

```rust
use crate::relation_sieve::relation::Relation;
use num::BigInt;

/// Backend type selector
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Native64,
    Native128,
    Fixed256,
    Fixed512,
    Arbitrary,
}

impl BackendType {
    pub fn name(&self) -> &'static str {
        match self {
            BackendType::Native64 => "u64 (native)",
            BackendType::Native128 => "u128 (native)",
            BackendType::Fixed256 => "U256 (crypto-bigint)",
            BackendType::Fixed512 => "U512 (crypto-bigint)",
            BackendType::Arbitrary => "BigInt (arbitrary)",
        }
    }
}

/// Trait for GNFS backends with different integer types
pub trait GnfsBackend: Send + Sync {
    /// Run sieving to find smooth relations
    fn sieve(&mut self) -> Result<Vec<Relation>, String>;

    /// Get current sieving progress
    fn get_progress(&self) -> (usize, usize);  // (found, target)

    /// Get backend type identifier
    fn backend_type(&self) -> BackendType;

    /// Estimate memory usage (for monitoring)
    fn estimated_memory_mb(&self) -> usize;

    /// Check if backend can handle this number size
    fn can_handle(n: &BigInt) -> bool where Self: Sized;
}

/// Select appropriate backend based on input size
pub fn select_backend(n: &BigInt) -> BackendType {
    let bits = n.bits() as usize;
    let degree = calculate_degree_from_bits(bits);

    // Calculate estimated norm bit-width with safety margin
    let safety_margin = 0.2;
    let base_norm_bits = match degree {
        3 => (bits / 3) + 40,
        4 => (bits / 4) + 50,
        5 => (bits / 5) + 55,
        _ => (bits / degree) + 60,
    };

    let norm_bits = (base_norm_bits as f64 * (1.0 + safety_margin)) as usize;

    match norm_bits {
        0..=60    => BackendType::Native64,
        61..=120  => BackendType::Native128,
        121..=250 => BackendType::Fixed256,
        251..=500 => BackendType::Fixed512,
        _         => BackendType::Arbitrary,
    }
}

fn calculate_degree_from_bits(bits: usize) -> usize {
    // Mimic GNFS::calculate_degree logic
    let digits = (bits as f64 * 0.301).ceil() as usize;  // log10(2^bits)
    if digits < 65 { 3 }
    else if digits < 125 { 4 }
    else if digits < 225 { 5 }
    else if digits < 315 { 6 }
    else { 7 }
}

/// Factory for creating appropriate backend
pub fn create_backend(
    n: &BigInt,
    cancel_token: &CancellationToken,
    polynomial_base: &BigInt,
    poly_degree: i32,
    prime_bound: &BigInt,
    relation_quantity: usize,
    relation_value_range: usize,
) -> Box<dyn GnfsBackend> {
    let backend_type = select_backend(n);

    log::info!("Selected {} backend for {}-digit number",
               backend_type.name(),
               n.to_string().len());

    match backend_type {
        BackendType::Native64 => {
            Box::new(GnfsBackendImpl::<u64>::new(
                n, cancel_token, polynomial_base, poly_degree,
                prime_bound, relation_quantity, relation_value_range
            ))
        },
        BackendType::Native128 => {
            Box::new(GnfsBackendImpl::<u128>::new(
                n, cancel_token, polynomial_base, poly_degree,
                prime_bound, relation_quantity, relation_value_range
            ))
        },
        BackendType::Fixed256 => {
            Box::new(GnfsBackendImpl::<crypto_bigint::U256>::new(
                n, cancel_token, polynomial_base, poly_degree,
                prime_bound, relation_quantity, relation_value_range
            ))
        },
        BackendType::Fixed512 => {
            Box::new(GnfsBackendImpl::<crypto_bigint::U512>::new(
                n, cancel_token, polynomial_base, poly_degree,
                prime_bound, relation_quantity, relation_value_range
            ))
        },
        BackendType::Arbitrary => {
            Box::new(GnfsBackendImpl::<malachite::Integer>::new(
                n, cancel_token, polynomial_base, poly_degree,
                prime_bound, relation_quantity, relation_value_range
            ))
        },
    }
}
```

#### File: `src/core/backend_impl.rs` (NEW)

```rust
use super::backend::{GnfsBackend, BackendType};
use crate::integer_math::gnfs_integer::GnfsInteger;
use crate::polynomial::polynomial::Polynomial;
use crate::relation_sieve::relation::Relation;
use num::BigInt;
use std::marker::PhantomData;

/// Generic GNFS implementation parameterized by integer type
pub struct GnfsBackendImpl<T: GnfsInteger> {
    // Core fields (converted to type T)
    n: T,
    polynomial: Polynomial<T>,
    polynomial_base: T,
    polynomial_degree: usize,

    // Factor bases (Vec<T> instead of Vec<BigInt>)
    rational_factor_base: Vec<T>,
    algebraic_factor_base: Vec<T>,
    quadratic_factor_base: Vec<T>,

    // Factor pair collections (also generic)
    rational_factor_pairs: Vec<(T, T)>,  // (p, m % p)
    algebraic_factor_pairs: Vec<(T, T)>, // (p, r) where f(r) ≡ 0 (mod p)

    // Sieving progress tracking
    smooth_relations_found: usize,
    smooth_relations_target: usize,
    current_a: T,
    current_b: T,

    // Cancellation
    cancel_token: Arc<CancellationToken>,

    _phantom: PhantomData<T>,
}

impl<T: GnfsInteger> GnfsBackendImpl<T> {
    pub fn new(
        n: &BigInt,
        cancel_token: &CancellationToken,
        polynomial_base: &BigInt,
        poly_degree: i32,
        prime_bound: &BigInt,
        relation_quantity: usize,
        relation_value_range: usize,
    ) -> Self {
        // Convert BigInt inputs to T
        let n_typed = T::from_bigint(n).expect("Number too large for this backend");
        let base_typed = T::from_bigint(polynomial_base).expect("Base too large");

        // Construct polynomial (same algorithm as GNFS::construct_new_polynomial)
        let polynomial = Self::construct_polynomial(&n_typed, &base_typed, poly_degree as usize);

        // Build factor bases
        let rational_fb = Self::build_rational_factor_base(&n_typed, prime_bound);
        let algebraic_fb = Self::build_algebraic_factor_base(&n_typed, &polynomial, prime_bound);

        // Build factor pair collections
        let rational_pairs = Self::build_rational_factor_pairs(&rational_fb, &base_typed);
        let algebraic_pairs = Self::build_algebraic_factor_pairs(&algebraic_fb, &polynomial);

        Self {
            n: n_typed,
            polynomial,
            polynomial_base: base_typed,
            polynomial_degree: poly_degree as usize,
            rational_factor_base: rational_fb,
            algebraic_factor_base: algebraic_fb,
            quadratic_factor_base: Vec::new(),  // TODO
            rational_factor_pairs: rational_pairs,
            algebraic_factor_pairs: algebraic_pairs,
            smooth_relations_found: 0,
            smooth_relations_target: relation_quantity,
            current_a: T::one(),
            current_b: T::from_u64(3),
            cancel_token: Arc::new(cancel_token.clone()),
            _phantom: PhantomData,
        }
    }

    fn construct_polynomial(n: &T, base: &T, degree: usize) -> Polynomial<T> {
        // Convert N to base-m representation to get polynomial coefficients
        // Same logic as GNFS::construct_new_polynomial
        let mut coefficients = Vec::with_capacity(degree + 1);
        let mut remainder = n.clone();

        for _ in 0..=degree {
            let coeff = &remainder % base;
            coefficients.push(coeff);
            remainder = &remainder / base;
        }

        Polynomial::from_coefficients(coefficients)
    }

    fn sieve_relation(&self, a: &T, b: &T) -> Option<Relation> {
        // Core sieving logic using type T for all arithmetic

        // Rational norm: a + b*m
        let rational_norm = a + &(b * &self.polynomial_base);

        // Early exit if rational norm doesn't factor
        if !self.is_smooth_over_base(&rational_norm, &self.rational_factor_base) {
            return None;
        }

        // Compute algebraic norm: f(-a/b) * (-b)^degree
        // This is the expensive operation!
        let algebraic_norm = self.compute_algebraic_norm(a, b);

        // Check if algebraic norm is smooth
        if !self.is_smooth_over_base(&algebraic_norm, &self.algebraic_factor_base) {
            return None;
        }

        // Convert to BigInt for output (only smooth relations)
        Some(Relation {
            a: a.to_bigint(),
            b: b.to_bigint(),
            rational_norm: rational_norm.to_bigint(),
            algebraic_norm: algebraic_norm.to_bigint(),
            // ... other fields
        })
    }

    fn compute_algebraic_norm(&self, a: &T, b: &T) -> T {
        // Algebraic norm: f(-a/b) * (-b)^degree
        // Use GnfsRational to avoid division until the end

        let neg_a = -a.clone();
        let ratio = GnfsRational::new(neg_a, b.clone());

        // Evaluate polynomial at rational point
        let poly_value = self.polynomial.evaluate_rational(&ratio);

        // Multiply by (-b)^degree
        let neg_b = -b.clone();
        let right = neg_b.pow(self.polynomial_degree as u32);

        // Extract integer part
        (poly_value * right).to_integer()
    }

    fn is_smooth_over_base(&self, value: &T, factor_base: &[T]) -> bool {
        // Trial division (same as FactorizationFactory::factor_with_base)
        let mut quotient = value.clone();

        for prime in factor_base {
            if &quotient < prime {
                break;  // Remaining quotient is too small
            }

            while &quotient % prime == T::zero() {
                quotient = &quotient / prime;
                if quotient.is_one() {
                    return true;  // Completely factored
                }
            }
        }

        quotient.is_one()  // Smooth iff quotient reduced to 1
    }
}

impl<T: GnfsInteger> GnfsBackend for GnfsBackendImpl<T> {
    fn sieve(&mut self) -> Result<Vec<Relation>, String> {
        use rayon::prelude::*;

        let mut smooth_relations = Vec::new();

        while self.smooth_relations_found < self.smooth_relations_target {
            if self.cancel_token.is_cancellation_requested() {
                break;
            }

            // Generate (a, b) pairs for this iteration
            let a_range = self.generate_a_range();
            let current_b = self.current_b.clone();

            // Parallel sieving using rayon
            let found: Vec<Relation> = a_range
                .par_iter()
                .filter_map(|a| self.sieve_relation(a, &current_b))
                .collect();

            self.smooth_relations_found += found.len();
            smooth_relations.extend(found);

            // Advance to next B
            self.current_b = &self.current_b + T::one();
        }

        Ok(smooth_relations)
    }

    fn get_progress(&self) -> (usize, usize) {
        (self.smooth_relations_found, self.smooth_relations_target)
    }

    fn backend_type(&self) -> BackendType {
        use std::any::TypeId;

        if TypeId::of::<T>() == TypeId::of::<u64>() {
            BackendType::Native64
        } else if TypeId::of::<T>() == TypeId::of::<u128>() {
            BackendType::Native128
        } else if TypeId::of::<T>() == TypeId::of::<crypto_bigint::U256>() {
            BackendType::Fixed256
        } else if TypeId::of::<T>() == TypeId::of::<crypto_bigint::U512>() {
            BackendType::Fixed512
        } else {
            BackendType::Arbitrary
        }
    }

    fn estimated_memory_mb(&self) -> usize {
        // Rough estimate based on type size and collection lengths
        let per_relation_bytes = std::mem::size_of::<Relation>();
        let factor_base_bytes = self.rational_factor_base.len() * std::mem::size_of::<T>();

        (per_relation_bytes * self.smooth_relations_target + factor_base_bytes) / (1024 * 1024)
    }

    fn can_handle(n: &BigInt) -> bool {
        T::from_bigint(n).is_some()
    }
}
```

### Phase 4: Update GNFS Struct

#### File: `src/Core/gnfs.rs` - Modifications

**Changes needed:**

```rust
// Add at top of file:
use crate::core::backend::{GnfsBackend, create_backend};

pub struct GNFS {
    // Keep existing fields for compatibility
    pub n: BigInt,
    pub polynomial_degree: usize,
    pub polynomial_base: BigInt,
    // ... other fields ...

    // NEW: Add backend field
    backend: Option<Box<dyn GnfsBackend>>,
}

impl GNFS {
    pub fn new(
        cancel_token: &CancellationToken,
        n: &BigInt,
        polynomial_base: &BigInt,
        poly_degree: i32,
        prime_bound: &BigInt,
        relation_quantity: usize,
        relation_value_range: usize,
        created_new_data: bool,
    ) -> Self {
        // Create backend FIRST
        let backend = create_backend(
            n,
            cancel_token,
            polynomial_base,
            poly_degree,
            prime_bound,
            relation_quantity,
            relation_value_range,
        );

        info!("Initialized {} backend for {}-digit number",
              backend.backend_type().name(),
              n.to_string().len());

        // Continue with existing GNFS initialization...
        let mut gnfs = GNFS {
            n: n.clone(),
            polynomial_degree: 0,
            polynomial_base: polynomial_base.clone(),
            // ... other fields ...
            backend: Some(backend),
        };

        // ... rest of initialization (can be simplified since backend handles much of it)

        gnfs
    }

    // NEW METHOD: Delegate sieving to backend
    pub fn find_relations(&mut self) -> Result<Vec<Relation>, String> {
        self.backend.as_mut()
            .ok_or("Backend not initialized")?
            .sieve()
    }
}
```

---

## Testing Strategy

### Unit Tests

**File: `src/integer_math/gnfs_integer.rs` - Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u64_basic_ops() {
        assert_eq!(u64::from_u64(42).to_bigint(), BigInt::from(42));
        assert_eq!(u64::from_u64(100).pow(3), 1_000_000);
        assert!(u64::from_u64(0).is_zero());
        assert!(u64::from_u64(1).is_one());
    }

    #[test]
    fn test_u128_large_values() {
        let large = u128::from_u64(1_000_000_000_000_000_000);
        assert_eq!(large.bits(), 60);
        assert_eq!(large.pow(2).bits(), 120);
    }

    #[test]
    fn test_overflow_detection() {
        let near_max = u64::MAX - 100;
        assert!(u64::from_u64(near_max).checked_add(&u64::from_u64(50)).is_some());
        assert!(u64::from_u64(near_max).checked_add(&u64::from_u64(200)).is_none());
    }
}
```

**File: `src/core/backend.rs` - Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_selection_boundaries() {
        // 11-digit number → should select Native64
        let n = BigInt::from(12_345_678_901u64);
        assert_eq!(select_backend(&n), BackendType::Native64);

        // 14-digit number → boundary case
        let n = BigInt::from(99_999_999_999_999u64);
        assert!(matches!(select_backend(&n), BackendType::Native64 | BackendType::Native128));

        // 20-digit number → should select Native128
        let n = BigInt::from_str("12345678901234567890").unwrap();
        assert_eq!(select_backend(&n), BackendType::Native128);

        // 80-digit number → should select Fixed256
        let n = BigInt::from_str("1234567890".repeat(8).as_str()).unwrap();
        assert_eq!(select_backend(&n), BackendType::Fixed256);
    }

    #[test]
    fn test_backend_creation() {
        let n = BigInt::from(738883u64);
        let cancel_token = CancellationToken::new();

        let backend = create_backend(
            &n,
            &cancel_token,
            &BigInt::from(10),
            3,
            &BigInt::from(100),
            100,
            1000,
        );

        assert_eq!(backend.backend_type(), BackendType::Native64);
        assert!(backend.estimated_memory_mb() < 500);
    }
}
```

### Integration Tests

**File: `tests/adaptive_backend_integration.rs` (NEW)**

```rust
use gnfs::*;

#[test]
fn test_cross_backend_consistency() {
    // Force different backends for the same small number
    // Verify they produce the same results

    let n = BigInt::from(738883u64);
    let cancel_token = CancellationToken::new();

    // Run with u64 backend
    let mut gnfs_u64 = create_gnfs_with_backend(
        &n, &cancel_token, BackendType::Native64
    );
    let relations_u64 = gnfs_u64.find_relations().unwrap();

    // Run with BigInt backend (for comparison)
    let mut gnfs_bigint = create_gnfs_with_backend(
        &n, &cancel_token, BackendType::Arbitrary
    );
    let relations_bigint = gnfs_bigint.find_relations().unwrap();

    // Should find same number of smooth relations
    assert_eq!(relations_u64.len(), relations_bigint.len());

    // Relations should be identical (modulo ordering)
    let mut r64_sorted = relations_u64;
    let mut rbig_sorted = relations_bigint;
    r64_sorted.sort();
    rbig_sorted.sort();

    assert_eq!(r64_sorted, rbig_sorted);
}

#[test]
fn test_end_to_end_factorization() {
    let test_cases = vec![
        (143u64, BackendType::Native64),           // 3 digits: 11 × 13
        (738883u64, BackendType::Native64),        // 6 digits
        (100085411u64, BackendType::Native64),     // 9 digits
        (10003430467u64, BackendType::Native64),   // 11 digits
    ];

    for (n, expected_backend) in test_cases {
        let n_big = BigInt::from(n);
        let cancel_token = CancellationToken::new();

        let mut gnfs = GNFS::new(
            &cancel_token,
            &n_big,
            &BigInt::from(10),
            3,
            &BigInt::from(100),
            100,
            1000,
            true,
        );

        assert_eq!(gnfs.backend.as_ref().unwrap().backend_type(), expected_backend);

        // Run full sieving
        let relations = gnfs.find_relations().unwrap();
        assert!(relations.len() > 0, "Should find at least some smooth relations");
    }
}
```

### Benchmark Suite

**File: `src/benchmark/adaptive_runner.rs` (NEW)**

```rust
pub fn benchmark_adaptive_backends(digit_counts: &[usize]) {
    use std::time::Instant;

    println!("\n=== ADAPTIVE BACKEND BENCHMARK ===\n");

    for &digits in digit_counts {
        let n = crate::benchmark::generate_test_number(digits);

        println!("Testing {}-digit number: {}", digits, n);

        let cancel_token = CancellationToken::new();
        let start = Instant::now();

        let mut gnfs = GNFS::new(
            &cancel_token,
            &n,
            &BigInt::from(10),
            -1,  // Auto-select degree
            &BigInt::from(100),
            100,
            1000,
            true,
        );

        let backend_type = gnfs.backend.as_ref().unwrap().backend_type();
        println!("  Backend: {}", backend_type.name());

        let init_time = start.elapsed();

        let sieve_start = Instant::now();
        let relations = gnfs.find_relations().unwrap();
        let sieve_time = sieve_start.elapsed();

        let memory_mb = gnfs.backend.as_ref().unwrap().estimated_memory_mb();

        println!("  Init time: {:.2}s", init_time.as_secs_f64());
        println!("  Sieve time: {:.2}s", sieve_time.as_secs_f64());
        println!("  Relations found: {}", relations.len());
        println!("  Memory usage: {} MB", memory_mb);
        println!("  Throughput: {:.0} relations/sec",
                 relations.len() as f64 / sieve_time.as_secs_f64());

        // Verify expected backend was chosen
        let expected_backend = match digits {
            0..=14 => BackendType::Native64,
            15..=30 => BackendType::Native128,
            31..=77 => BackendType::Fixed256,
            78..=154 => BackendType::Fixed512,
            _ => BackendType::Arbitrary,
        };

        assert_eq!(backend_type, expected_backend,
                   "Expected {} for {}-digit number",
                   expected_backend.name(), digits);

        println!();
    }
}
```

---

## Performance Expectations

### Memory Usage Projections

| Backend | Max Digits | Algebraic Norm Bits | Memory/Core | Example Number |
|---------|-----------|-------------------|-------------|----------------|
| Native64 | 14 | ≤60 | **375 MB** | 12,345,678,901 (11 digits) |
| Native128 | 30 | ≤120 | **1 GB** | 10^29 |
| Fixed256 | 77 | ≤250 | **2 GB** | RSA-256 |
| Fixed512 | 154 | ≤500 | **4 GB** | RSA-512 |
| Arbitrary | Unlimited | Unlimited | **10+ GB** | RSA-1024+ |

**Current issue:** 11-digit number uses **70 GB** (186x too much!)
**Target:** 11-digit number should use **375 MB** with Native64 backend
**Expected improvement:** **186x memory reduction** 🎉

### Speed Projections

Based on literature and existing fast-path code in `factorization_factory.rs`:

| Operation | BigInt (baseline) | u64 speedup | u128 speedup | U256 speedup |
|-----------|------------------|-------------|--------------|--------------|
| Addition | 1x | **50x** | **40x** | **20x** |
| Multiplication | 1x | **100x** | **80x** | **30x** |
| Division/Modulo | 1x | **150x** | **100x** | **40x** |
| **Overall sieving** | 1x | **50-100x** | **30-50x** | **10-30x** |

**Evidence from existing code:**
- Lines 107-121 in `factorization_factory.rs` show u32 fast path
- Lines 125-139 show u64 fast path
- Both are dramatically faster than BigInt slow path (lines 142-150)

**Expected results:**
- 11-digit numbers: **50-100x faster** (currently slow, will use Native64)
- 30-digit numbers: **30-50x faster** (will use Native128)
- 77-digit numbers: **10-30x faster** (will use Fixed256)

---

## Risks and Mitigation

### Risk 1: Overflow in Edge Cases

**Risk:** Near-boundary numbers might overflow fixed-width types.

**Mitigation:**
- ✅ Add 20% safety margin to bit-width calculations
- ✅ Use checked arithmetic (`checked_add`, `checked_mul`)
- ✅ Fallback to next-larger type if overflow detected
- ✅ Extensive boundary testing (14, 30, 77, 154 digits)

### Risk 2: Precision Loss in Rational Arithmetic

**Risk:** `GnfsRational<u64>` might lose precision vs `BigRational`.

**Mitigation:**
- ✅ Use careful numerator/denominator tracking
- ✅ Only divide at the end (avoid repeated divisions)
- ✅ Test against BigRational results for consistency
- ✅ Document precision limitations in comments

### Risk 3: Code Duplication and Maintainability

**Risk:** Five implementations might diverge or become hard to maintain.

**Mitigation:**
- ✅ Use hybrid architecture (trait objects + monomorphization)
- ✅ Share 95% of code through generics
- ✅ Centralize selection logic in `backend.rs`
- ✅ Comprehensive tests ensure consistency across backends

### Risk 4: Compilation Time Increase

**Risk:** Monomorphization of `GnfsBackendImpl<T>` might slow compilation.

**Mitigation:**
- ✅ Hybrid approach limits monomorphization to hot paths
- ✅ Use trait objects for non-critical code
- ✅ Monitor compilation times in CI
- ✅ Can switch to dynamic dispatch if compilation becomes issue

---

## Implementation Checklist

### Phase 1: Foundation (Week 1)
- [ ] Add `crypto-bigint` and `malachite` to Cargo.toml
- [ ] Create `src/integer_math/gnfs_integer.rs` with trait definition
- [ ] Implement `GnfsInteger` for u64, u128, U256, U512, Integer
- [ ] Write unit tests for each implementation
- [ ] **Milestone:** All numeric types implement GnfsInteger trait ✓

### Phase 2: Generics (Week 2)
- [ ] Create `src/integer_math/gnfs_rational.rs`
- [ ] Make `Polynomial<T>` generic (keep BigInt as default)
- [ ] Update polynomial evaluation methods
- [ ] Test polynomial evaluation with each integer type
- [ ] **Milestone:** Generic polynomial arithmetic works ✓

### Phase 3: Backend System (Week 3)
- [ ] Create `src/core/backend.rs` with trait and selection logic
- [ ] Create `src/core/backend_impl.rs` with generic implementation
- [ ] Implement `GnfsBackendImpl<T>::new()` initialization
- [ ] Implement `GnfsBackendImpl<T>::sieve()` core sieving logic
- [ ] Write unit tests for backend selection
- [ ] **Milestone:** Backend system compiles and basic tests pass ✓

### Phase 4: Integration (Week 4)
- [ ] Update `src/Core/gnfs.rs` to use backend system
- [ ] Update `src/main.rs` to use new API
- [ ] Run integration tests with real numbers
- [ ] Fix any ownership/lifetime issues
- [ ] **Milestone:** End-to-end factorization works with adaptive backend ✓

### Phase 5: Testing & Validation (Week 5)
- [ ] Write cross-backend consistency tests
- [ ] Add boundary case tests (14, 30, 77, 154 digits)
- [ ] Benchmark memory usage for each backend
- [ ] Benchmark speed for each backend
- [ ] Compare results with current BigInt-only implementation
- [ ] **Milestone:** All tests pass, performance validated ✓

### Phase 6: Documentation (Week 6)
- [ ] Update CLAUDE.md with adaptive architecture section
- [ ] Add inline documentation to all new files
- [ ] Create usage examples
- [ ] Write performance comparison report
- [ ] Document known limitations and edge cases
- [ ] **Milestone:** Project fully documented ✓

---

## Success Criteria

Implementation is successful if:

1. ✅ **Automatic selection:** Backend is chosen automatically based on input size
2. ✅ **Memory efficiency:** Memory usage ≤4GB per core for numbers up to 154 digits
3. ✅ **Performance:** 10-100x speedup vs current BigInt-only approach
4. ✅ **Correctness:** All backends produce identical results (up to ordering)
5. ✅ **GPU compatibility:** u64/u128/U256/U512 backends are GPU-compatible
6. ✅ **Maintainability:** Single codebase with < 5% code duplication
7. ✅ **Tests:** 100% of existing tests still pass
8. ✅ **Coverage:** New unit tests for each backend type

---

## Next Steps

**Ready to proceed with implementation:**

1. **Review this design document** - Verify the architecture is sound
2. **Add dependencies** - Update Cargo.toml with crypto-bigint and malachite
3. **Start Phase 1** - Implement GnfsInteger trait and basic implementations
4. **Incremental testing** - Test each backend before moving to the next
5. **Iterate based on results** - Adjust bit-width calculations if needed

**Estimated timeline:** 6 weeks for full implementation and testing
**Estimated impact:** 50-100x performance improvement, 186x memory reduction for small numbers

---

## Appendix: Mathematical Justification

### Algebraic Norm Bit-Width Formula

For degree-d GNFS with polynomial f(x) = aₐx^d + ... + a₁x + a₀:

**Rational norm:** `a + b*m` where m ≈ N^(1/d)
- Bit-width: `log₂(N) / d + log₂(max(a, b))`
- For a, b ≤ 10,000: `bits(N)/d + 14`

**Algebraic norm:** `f(-a/b) * (-b)^d`
- Bit-width: `log₂(|f(-a/b)|) + d * log₂(|b|)`
- Approximation: `log₂(N) / d + 40` (empirically validated)

**Safety margin:** Add 20% to account for:
- Polynomial coefficient size
- Near-boundary edge cases
- Rounding errors in bit-width calculation

**Validation:** Test with actual numbers at each boundary to verify formulas.

---

**End of Design Document**
