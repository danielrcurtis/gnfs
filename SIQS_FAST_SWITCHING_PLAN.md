# SIQS Fast Polynomial Switching - Implementation Plan

## Executive Summary

**Goal**: Implement Phase 2 of SIQS - fast polynomial switching using binary Gray code to achieve ~2x speedup.

**Current State**: Phase 1 MVP generates polynomials correctly but recomputes all sieving roots from scratch for each polynomial, taking milliseconds per switch.

**Target**: Incremental root updates taking microseconds per switch, enabling efficient use of multiple polynomials from the same 'a' coefficient.

**Effort Estimate**: 2-3 days (~16-24 hours)
**Expected Speedup**: ~2x faster overall factorization

---

## Background

### Current Implementation (Phase 1)

For each polynomial:
1. Generate new 'a' coefficient (product of j primes)
2. Compute b using CRT
3. **For each prime p in factor base**: Compute sieving roots from scratch
   - Calculate a⁻¹ mod p
   - Calculate (tsqrt - b) × a⁻¹ mod p
   - **Cost**: O(factor_base_size) modular operations

**Bottleneck**: Step 3 is repeated for every polynomial, even when using the same 'a'.

### SIQS Fast Switching (Phase 2)

Key insight from Contini (1997):
- Each 'a' coefficient can generate **2^(j-1)** polynomials
- Only the 'b' coefficient changes: b' = b ± B[i]
- Roots update incrementally: soln' = soln - Δ mod p where Δ = (b' - b) × a⁻¹ mod p

**Advantage**:
- Compute a⁻¹ mod p **once per 'a'** instead of once per polynomial
- Root updates become simple addition/subtraction mod p
- **Cost**: O(factor_base_size) additions per switch vs O(factor_base_size) modular inverses

**Expected speedup**: ~10-50x faster polynomial switching → ~2x faster overall

---

## Algorithm: Binary Gray Code Method

### Polynomial Generation with Same 'a'

Given 'a' and B[1], B[2], ..., B[j] arrays:

**Generate 2^(j-1) polynomials**:
```
Polynomial 0: b = B[1] + B[2] + ... + B[j]
Polynomial 1: b' = b - 2×B[j]           (flip bit j)
Polynomial 2: b' = b - 2×B[j-1]         (flip bit j-1)
Polynomial 3: b' = b - 2×B[j] - 2×B[j-1]
...
```

This follows binary Gray code sequence where only one bit changes between successive values.

### Root Update Formula

When switching from polynomial i to i+1:
```
v = count_trailing_zeros(2×i)   // Find which B[v] changes
e = sign based on Gray code      // +1 or -1

b_new = b_old + e × B[v]
Δ = e × B[v] × ainv mod p        // ainv computed once per 'a'

For each prime p:
  soln1_new = (soln1_old - Δ) mod p
  soln2_new = (soln2_old - Δ) mod p
```

**Cost per switch**: 2 additions mod p per prime (vs 2 modular inverses per prime)

---

## Implementation Plan

### Phase 2.1: Data Structure Updates (4-6 hours)

#### 1. Extend `SIQSPolynomial` Structure

**File**: `src/algorithms/siqs/polynomial.rs`

```rust
#[derive(Clone, Debug)]
pub struct SIQSPolynomial {
    pub a: BigInt,
    pub b: BigInt,
    pub c: BigInt,
    pub a_factors: Vec<u64>,
    pub b_array: Vec<BigInt>,

    // NEW: Fast switching support
    pub poly_index: u32,              // Current polynomial index (0 to 2^(j-1) - 1)
    pub max_polynomials: u32,         // 2^(j-1)
}
```

#### 2. Create Sieving State Structure

**File**: `src/algorithms/siqs/mod.rs`

```rust
/// Tracks sieving state for fast polynomial switching
struct SievingState {
    polynomial: SIQSPolynomial,

    // Sieving roots for current polynomial
    // roots[prime_idx] = (root1, root2) or just root1 for p=2
    sieve_roots: Vec<(i64, i64)>,

    // Pre-computed values for fast switching
    // ainv_cache[prime_idx] = a⁻¹ mod p
    ainv_cache: Vec<i64>,

    // Delta arrays for each B[i]
    // delta_arrays[b_idx][prime_idx] = B[i] × a⁻¹ mod p
    delta_arrays: Vec<Vec<i64>>,
}
```

#### 3. Update Tests

Add unit tests for new structures:
- `test_polynomial_index_tracking`
- `test_sieving_state_creation`

**Deliverable**: Updated data structures with tests (passing)

---

### Phase 2.2: Pre-computation (6-8 hours)

#### 4. Implement ainv Cache Computation

**File**: `src/algorithms/siqs/mod.rs`

```rust
impl SIQS {
    /// Pre-compute a⁻¹ mod p for all primes in factor base
    fn compute_ainv_cache(&self, a: &BigInt) -> Vec<i64> {
        let mut cache = Vec::with_capacity(self.factor_base_size);

        for prime in &self.factor_base {
            if prime.p <= 1 || polynomial.a_factors.contains(&prime.p) {
                cache.push(0); // Skip -1 marker and primes dividing a
                continue;
            }

            let p = BigInt::from(prime.p);
            let a_mod_p = a.mod_floor(&p);

            match Self::mod_inverse_i64(&a_mod_p.to_i64().unwrap_or(0), prime.p as i64) {
                Some(inv) => cache.push(inv),
                None => cache.push(0), // Should never happen for valid factor base
            }
        }

        cache
    }
}
```

#### 5. Implement Delta Array Pre-computation

```rust
impl SIQS {
    /// Pre-compute B[i] × a⁻¹ mod p for all primes and all B[i]
    fn compute_delta_arrays(
        &self,
        b_array: &[BigInt],
        ainv_cache: &[i64],
    ) -> Vec<Vec<i64>> {
        let j = b_array.len();
        let mut delta_arrays = vec![vec![0i64; self.factor_base_size]; j];

        for b_idx in 0..j {
            let b_i = &b_array[b_idx];

            for (prime_idx, prime) in self.factor_base.iter().enumerate() {
                if prime.p <= 1 || ainv_cache[prime_idx] == 0 {
                    continue;
                }

                let p = prime.p as i64;
                let b_i_mod_p = b_i.mod_floor(&BigInt::from(p)).to_i64().unwrap_or(0);
                let ainv = ainv_cache[prime_idx];

                // Δ = B[i] × a⁻¹ mod p
                delta_arrays[b_idx][prime_idx] = (b_i_mod_p * ainv).rem_euclid(p);
            }
        }

        delta_arrays
    }
}
```

#### 6. Initialize Sieving State

```rust
impl SIQS {
    /// Initialize sieving state for first polynomial
    fn initialize_sieving_state(
        &self,
        polynomial: SIQSPolynomial,
    ) -> SievingState {
        // Compute initial roots (same as current implementation)
        let sieve_roots = self.compute_sieve_roots(&polynomial);

        // Pre-compute caches for fast switching
        let ainv_cache = self.compute_ainv_cache(&polynomial.a);
        let delta_arrays = self.compute_delta_arrays(&polynomial.b_array, &ainv_cache);

        SievingState {
            polynomial,
            sieve_roots,
            ainv_cache,
            delta_arrays,
        }
    }
}
```

#### 7. Extract Root Computation to Separate Method

Refactor existing root computation into reusable method:

```rust
impl SIQS {
    /// Compute sieving roots for a polynomial (used for initialization)
    fn compute_sieve_roots(&self, polynomial: &SIQSPolynomial) -> Vec<(i64, i64)> {
        // Move existing root computation code here
        // Returns vector of (root1, root2) tuples
    }
}
```

#### 8. Update Tests

Add tests for pre-computation:
- `test_ainv_cache_computation`
- `test_delta_array_computation`
- `test_sieving_state_initialization`

**Deliverable**: Pre-computation infrastructure (tested)

---

### Phase 2.3: Fast Switching Logic (4-6 hours)

#### 9. Implement Gray Code Utilities

**File**: `src/algorithms/siqs/polynomial.rs`

```rust
/// Count trailing zeros in binary representation
pub fn count_trailing_zeros(n: u32) -> u32 {
    if n == 0 {
        return 32;
    }
    n.trailing_zeros()
}

/// Get sign for Gray code transition
/// Returns +1 or -1 based on bit pattern
pub fn gray_code_sign(index: u32, bit_position: u32) -> i64 {
    // Check if bit is set in Gray code representation
    let gray = index ^ (index >> 1);
    if (gray >> bit_position) & 1 == 1 {
        -1
    } else {
        1
    }
}

#[cfg(test)]
mod gray_code_tests {
    #[test]
    fn test_count_trailing_zeros() {
        assert_eq!(count_trailing_zeros(1), 0);  // 0b1
        assert_eq!(count_trailing_zeros(2), 1);  // 0b10
        assert_eq!(count_trailing_zeros(4), 2);  // 0b100
        assert_eq!(count_trailing_zeros(6), 1);  // 0b110
    }

    #[test]
    fn test_gray_code_transitions() {
        // Verify Gray code changes only one bit at a time
        for i in 0..16u32 {
            let gray_i = i ^ (i >> 1);
            let gray_next = (i + 1) ^ ((i + 1) >> 1);
            let diff = gray_i ^ gray_next;

            // Should be a power of 2 (only one bit different)
            assert!(diff.is_power_of_two() || diff == 0);
        }
    }
}
```

#### 10. Implement Fast Polynomial Switching

**File**: `src/algorithms/siqs/mod.rs`

```rust
impl SievingState {
    /// Switch to next polynomial using fast incremental update
    /// Returns true if switch succeeded, false if exhausted polynomials
    fn switch_to_next_polynomial(&mut self) -> bool {
        let current_idx = self.polynomial.poly_index;
        let next_idx = current_idx + 1;

        if next_idx >= self.polynomial.max_polynomials {
            return false; // No more polynomials for this 'a'
        }

        // Determine which B[i] changes
        let v = count_trailing_zeros(2 * current_idx) as usize;

        if v >= self.polynomial.b_array.len() {
            return false;
        }

        // Determine sign of change
        let e = gray_code_sign(next_idx, v as u32);

        // Update b coefficient
        let b_v = &self.polynomial.b_array[v];
        let delta_b = b_v * (2 * e);
        self.polynomial.b = &self.polynomial.b + delta_b;

        // Update c coefficient: c = (b² - n) / a
        let b_squared = &self.polynomial.b * &self.polynomial.b;
        // Note: Need access to 'n' - pass as parameter or store in state

        // Update sieving roots incrementally
        for prime_idx in 0..self.sieve_roots.len() {
            let delta = self.delta_arrays[v][prime_idx];
            let p = /* get prime from somewhere */ as i64;

            let (root1, root2) = self.sieve_roots[prime_idx];

            // Incremental update: soln' = soln - e × Δ mod p
            let adjustment = (e * delta).rem_euclid(p);

            self.sieve_roots[prime_idx] = (
                (root1 - adjustment).rem_euclid(p),
                (root2 - adjustment).rem_euclid(p),
            );
        }

        self.polynomial.poly_index = next_idx;
        true
    }
}
```

#### 11. Refactor Sieving to Use Fast Switching

**File**: `src/algorithms/siqs/mod.rs`

```rust
impl SIQS {
    /// Sieve with multiple polynomials using fast switching
    fn sieve_with_polynomials_fast(&self) -> Vec<Relation> {
        let mut all_relations = Vec::new();
        let required_relations = self.factor_base_size + self.params.relation_margin;

        let max_a_values = 20; // Try up to 20 different 'a' coefficients

        for a_idx in 0..max_a_values {
            // Generate new 'a' and initial polynomial
            let polynomial = match self.generate_polynomial_with_fast_switching() {
                Some(poly) => poly,
                None => continue,
            };

            info!("Generated 'a' #{}: {} (can produce {} polynomials)",
                  a_idx + 1, polynomial.a, polynomial.max_polynomials);

            // Initialize sieving state with pre-computed caches
            let mut state = self.initialize_sieving_state(polynomial);

            // Sieve with first polynomial
            let relations = self.sieve_with_state(&state);
            all_relations.extend(relations);

            // Fast switch through remaining polynomials
            while state.switch_to_next_polynomial() {
                let poly_idx = state.polynomial.poly_index;

                if poly_idx % 8 == 0 {
                    debug!("Sieving with polynomial {} of {} for 'a' #{}",
                           poly_idx + 1, state.polynomial.max_polynomials, a_idx + 1);
                }

                let relations = self.sieve_with_state(&state);
                all_relations.extend(relations);

                if all_relations.len() >= required_relations {
                    info!("Collected enough relations, stopping");
                    return all_relations;
                }
            }

            info!("Exhausted {} polynomials for 'a' #{}, collected {} relations so far",
                  state.polynomial.max_polynomials, a_idx + 1, all_relations.len());

            if all_relations.len() >= required_relations {
                break;
            }
        }

        all_relations
    }

    /// Sieve using current sieving state (roots already computed)
    fn sieve_with_state(&self, state: &SievingState) -> Vec<Relation> {
        // Similar to current sieve_with_polynomial but use pre-computed roots
        // from state.sieve_roots instead of computing them
    }
}
```

#### 12. Update `generate_polynomial` for Fast Switching

**File**: `src/algorithms/siqs/polynomial.rs`

```rust
/// Generate polynomial with metadata for fast switching
pub fn generate_polynomial_with_fast_switching(
    n: &BigInt,
    factor_base: &[Prime],
    params: &SIQSParameters,
    target_a: &BigInt,
) -> Option<SIQSPolynomial> {
    // Use existing generate_polynomial logic
    let mut poly = generate_polynomial(n, factor_base, params, target_a)?;

    // Add fast switching metadata
    let j = params.primes_per_a;
    poly.poly_index = 0;
    poly.max_polynomials = 2u32.pow((j - 1) as u32);

    Some(poly)
}
```

#### 13. Update Tests

Add comprehensive tests for fast switching:
- `test_fast_polynomial_switch`
- `test_gray_code_sequence`
- `test_root_updates_correct`
- `test_multiple_switches`

**Deliverable**: Working fast polynomial switching (tested)

---

### Phase 2.4: Integration & Benchmarking (2-4 hours)

#### 14. Update Main SIQS Entry Point

**File**: `src/algorithms/siqs/mod.rs`

```rust
impl SIQS {
    pub fn factor(&mut self) -> Option<(BigInt, BigInt)> {
        self.build_factor_base();

        if self.factor_base_size < 10 {
            warn!("Factor base too small");
            return None;
        }

        // Use fast switching method
        let relations = self.sieve_with_polynomials_fast();

        // Rest of factorization logic unchanged
        // ...
    }
}
```

#### 15. Add Feature Flag (Optional)

Allow comparing old vs new method:

```rust
// In mod.rs
pub fn factor_with_method(&mut self, use_fast_switching: bool) -> Option<(BigInt, BigInt)> {
    self.build_factor_base();

    let relations = if use_fast_switching {
        self.sieve_with_polynomials_fast()
    } else {
        self.sieve_with_polynomials() // Old method
    };

    // Continue with matrix solving...
}
```

#### 16. Create Benchmark Tests

**File**: `tests/siqs_fast_switching_bench.rs`

```rust
#[test]
#[ignore]
fn benchmark_fast_switching_vs_original() {
    use std::time::Instant;

    let n = BigInt::from_str("10000000000000000016800000000000000005031").unwrap();

    // Benchmark original method
    let start = Instant::now();
    let result1 = siqs_original(&n);
    let time_original = start.elapsed();

    // Benchmark fast switching
    let start = Instant::now();
    let result2 = siqs_fast(&n);
    let time_fast = start.elapsed();

    println!("Original method: {:?}", time_original);
    println!("Fast switching:  {:?}", time_fast);
    println!("Speedup: {:.2}x", time_original.as_secs_f64() / time_fast.as_secs_f64());

    // Verify both produce same result
    assert_eq!(result1.is_some(), result2.is_some());
}
```

#### 17. Update Documentation

Update `SIQS_MVP_COMPLETE.md` to reflect Phase 2 completion:
- Mark Phase 2 as complete
- Document speedup achieved
- Update performance expectations

**Deliverable**: Integrated fast switching with benchmarks

---

## Testing Strategy

### Unit Tests

For each component:
1. **Gray code utilities**:
   - Verify trailing zero counts
   - Verify Gray code only changes one bit per step
   - Test sign computation

2. **Pre-computation**:
   - Verify ainv cache correctness (ainv × a ≡ 1 mod p)
   - Verify delta arrays match B[i] × ainv mod p
   - Test with various 'a' values and factor bases

3. **Polynomial switching**:
   - Verify b updates correctly
   - Verify c recomputed correctly
   - Verify root updates match full recomputation (within mod p)

4. **End-to-end**:
   - Switch through all 2^(j-1) polynomials
   - Verify each polynomial is distinct
   - Verify factorization still works

### Integration Tests

1. **Small number test** (4 digits):
   - Verify fast switching doesn't break existing functionality
   - Should complete in < 1 second

2. **40-digit test**:
   - Factor known 41-digit semiprime
   - Verify correct result
   - Measure time and compare to Phase 1

### Performance Validation

Expected improvements:
- **Polynomial switching time**: 10-50x faster (ms → μs)
- **Overall factorization time**: ~2x faster
- **Relations per second**: 1.5-2x higher

If not seeing expected improvements, profile to find bottlenecks.

---

## Success Criteria

### Phase 2 Complete When:

✅ All new unit tests pass (20+ tests)
✅ Integration tests pass for small and 40-digit numbers
✅ Fast switching produces same results as original method
✅ Speedup ≥ 1.5x on 40-digit factorization
✅ No regression in correctness or stability
✅ Documentation updated

### Performance Targets:

- 40-digit factorization: **< 2 minutes** (vs ~3-5 min Phase 1)
- 45-digit factorization: **< 10 minutes** (vs ~15-20 min Phase 1)
- Polynomial switch time: **< 10 microseconds** (vs ~1-5 ms Phase 1)

---

## Implementation Order

**Day 1** (8 hours):
- [ ] Data structure updates (2.1)
- [ ] Pre-computation infrastructure (2.2.1-2.2.3)
- [ ] Unit tests for structures and pre-computation

**Day 2** (8 hours):
- [ ] Complete pre-computation (2.2.4-2.2.8)
- [ ] Gray code utilities (2.3.1)
- [ ] Fast switching logic (2.3.2)
- [ ] Unit tests for switching

**Day 3** (4-8 hours):
- [ ] Refactor sieving to use fast switching (2.3.3)
- [ ] Integration and testing (2.4)
- [ ] Benchmarking and validation
- [ ] Documentation updates

---

## Risk Mitigation

### Risk 1: Root Update Errors
**Mitigation**:
- Add verification mode that compares incremental updates to full recomputation
- Log first few polynomials in detail to catch errors early

### Risk 2: Performance Not as Expected
**Mitigation**:
- Profile both methods to identify bottlenecks
- May need to optimize BigInt operations in delta computation
- Consider caching more aggressively

### Risk 3: Complex Integration
**Mitigation**:
- Keep old method available via feature flag
- Implement incrementally with frequent testing
- Rollback option if fast switching proves problematic

---

## References

1. **Contini (1997)**: "Factoring Integers with the Self-Initializing Quadratic Sieve"
   - Section 3.2: "Self-Initialization"
   - Algorithm 3.2: Fast polynomial generation

2. **Silverman (1987)**: "The Multiple Polynomial Quadratic Sieve"
   - Polynomial switching discussion

3. **C-Quadratic-Sieve Implementation**:
   - https://github.com/michel-leonard/C-Quadratic-Sieve
   - See `qs_gray_code` and `qs_next_poly` functions

---

## Conclusion

Fast polynomial switching is the key optimization that makes SIQS significantly faster than basic MPQS. By pre-computing modular inverses and using incremental root updates, we eliminate the most expensive part of polynomial switching.

**Expected outcome**: ~2x speedup for 40-digit factorization, bringing performance closer to production SIQS implementations.

**After Phase 2**: The SIQS implementation will be feature-complete for basic use. Phase 3 (large primes) would provide an additional 2-6x speedup but is optional for the MVP.
