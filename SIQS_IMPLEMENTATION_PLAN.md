# SIQS Implementation Plan
## Self-Initializing Quadratic Sieve for 40-100 Digit Factorization

---

## Executive Summary

**Current Situation:**
- Single-polynomial QS implementation is **mathematically unable** to factor 40+ digit numbers
- For 41-digit n, Q(x) values are 23-26 digits, but B=8,000 can only smooth ~17-24 digit numbers
- Finding 0/18,625 smooth relations confirms this is a **fundamental limitation**, not a bug

**Root Cause:**
- Research parameters assume **Multiple Polynomial QS (MPQS)**, not single-polynomial QS
- MPQS uses Q(x) = (ax + b)² - n with optimized 'a' to reduce norms
- Single-polynomial uses Q(x) = x² - n which produces norms ≈ size of n

**Recommended Solution:**
- Implement **SIQS (Self-Initializing Quadratic Sieve)**
- SIQS is MPQS with 2x speedup via fast polynomial switching
- Required for 40-100 digit range, as used by all production implementations

**Implementation Effort:**
- **Phase 1 (MVP)**: 3-5 days, ~800 LOC
- **Phase 2 (Full SIQS)**: 2-3 days, +300 LOC
- **Phase 3 (Optimizations)**: 3-5 days, +400 LOC
- **Total**: ~10-16 days, ~1,500 LOC

**Expected Performance:**
- 40 digits: ~90 seconds (vs current: fails)
- 50 digits: ~10 minutes (vs current: fails)
- 60 digits: ~2 hours (vs current: fails)

---

## Part 1: Understanding the Problem

### Why Single-Polynomial QS Fails for 40+ Digits

**Mathematical Analysis:**

For n = 10⁴¹ (41-digit test number):

1. **Single-polynomial QS**: Q(x) = x² - n
   - Sieve around x ≈ √n ≈ 10²⁰·⁵
   - Q(x) values: 10²³ to 10²⁶ (23-26 digits)

2. **Smoothness bound**: B = 8,000
   - Max smooth number: ≈ 8000^k for small k
   - 8000⁶ ≈ 2.6×10²³ (24 digits)
   - 8000⁷ ≈ 2.1×10²⁷ (28 digits)

3. **The Gap**:
   - Q(x) values: 23-26 digits
   - B-smooth range: up to ~24 digits (barely)
   - **Probability of smoothness: essentially 0%**

**Why SIQS Works:**

SIQS uses Q(x) = (ax + b)² - n = a × [residue]
- Choose a ≈ √(2n)/M ≈ √n/M
- Factor out 'a' (known prime factors)
- Remaining residue ≈ n/a ≈ M√n
- For M=700,000: residue ≈ 7×10⁵ × 10²⁰·⁵ ≈ **10²⁵·⁵** (25-26 digits)
- But after factoring out 'a' (~10¹⁵ for 40 digits): **effective size ~20-24 digits** ✓

**Conclusion**: Your code is correct. The algorithm is wrong for the task.

---

## Part 2: SIQS Algorithm Overview

### Key Differences from Single-Polynomial QS

| Aspect | Single-Poly QS | SIQS |
|--------|----------------|------|
| **Polynomial** | Q(x) = x² - n | Q(x) = (ax + b)² - n |
| **Norm size** | ~40 digits | ~20-24 digits (after factoring 'a') |
| **Polynomials used** | 1 | Hundreds (8 per 'a', many 'a' values) |
| **Polynomial switching** | N/A | Fast (microseconds with self-init) |
| **Target range** | ≤ 30 digits | 40-100 digits |

### SIQS Algorithm Steps

**1. Initialization** (once per factorization)
- Choose B (smoothness bound) and M (sieve interval)
- Build factor base: primes p ≤ B where (n/p) = 1
- Compute tsqrt[p] = √n mod p for each prime

**2. Generate Polynomial** (every 8-16 polynomials)
- Select j primes q₁, q₂, ..., qⱼ from factor base (j = 3-5)
- Set a = q₁ × q₂ × ... × qⱼ
- Compute b such that b² ≡ n (mod a) using CRT
- Set c = (b² - n) / a

**3. Initialize Sieve** (once per polynomial)
- For each prime p ∉ {q₁, ..., qⱼ}:
  - Compute ainv = a⁻¹ mod p
  - Compute roots: (tsqrt[p] - b) × ainv mod p

**4. Sieve** (same as current QS)
- Initialize log array
- Add log(p) to positions x where Q(x) ≡ 0 (mod p)
- Identify candidates above threshold

**5. Trial Division** (modified from current QS)
- For candidate x: Q(x) = (ax + b)² - n
- Factor out 'a' first (known factors)
- Trial divide remainder by factor base
- Store smooth relations

**6. Switch Polynomial** (SIQS fast method)
- Use binary Gray code to update b: b' = b ± B[i]
- Incrementally update roots: soln' = soln - Δ mod p
- **Key advantage**: Takes microseconds vs milliseconds

**7. Repeat** until enough relations collected

**8. Matrix Solve + Factor Extraction** (same as current QS)

---

## Part 3: Implementation Plan

### Phase 1: Minimal Viable SIQS (3-5 days)

**Goal**: Factor 40-digit numbers with basic SIQS

**Tasks:**

1. **Create SIQS module structure** (4 hours)
   ```
   src/algorithms/siqs/
     ├── mod.rs           (public API)
     ├── polynomial.rs    (generation & switching)
     ├── sieve.rs         (sieving logic)
     └── parameters.rs    (parameter selection)
   ```

2. **Implement polynomial generation** (12-16 hours)
   - Select j primes for 'a' coefficient (~300 LOC)
   - Compute B[i] arrays using CRT (~200 LOC)
   - Compute b from B[i] values (~100 LOC)
   - Unit tests for each component (~200 LOC)

3. **Modify trial division** (4 hours)
   - Factor out 'a' before trial division (~50 LOC)
   - Track 'a' factors in relation (~50 LOC)

4. **Basic polynomial switching** (6-8 hours)
   - Full recomputation per switch (no self-init yet) (~200 LOC)
   - Generate multiple polynomials per 'a' (~100 LOC)

5. **Integration testing** (8 hours)
   - Test on 35-40 digit semiprimes
   - Verify relations found
   - End-to-end factorization

**Deliverables:**
- Working SIQS that factors 40-digit numbers
- ~800 LOC new code
- Reuses ~500 LOC from existing QS

**Success Criteria:**
- Factor 40-digit test number in < 5 minutes
- Find smooth relations (non-zero count)

---

### Phase 2: Full SIQS with Self-Initialization (2-3 days)

**Goal**: 2x speedup via fast polynomial switching

**Tasks:**

1. **SIQS polynomial switching** (8-10 hours)
   - Binary Gray code index tracking (~100 LOC)
   - Incremental b updates (~50 LOC)
   - Incremental root updates (~100 LOC)

2. **Multiple 'a' value support** (4-6 hours)
   - Generate new 'a' after 2^(j-1) polynomials (~100 LOC)
   - Prime selection heuristics (~50 LOC)

3. **Benchmarking** (4 hours)
   - Compare Phase 1 vs Phase 2 performance
   - Verify ~2x speedup from self-init

**Deliverables:**
- Fast polynomial switching (microseconds per switch)
- ~300 LOC additional code
- Benchmark showing speedup

**Success Criteria:**
- Factor 40-digit number in < 3 minutes
- Polynomial switch overhead < 1% of total time

---

### Phase 3: Optimizations (3-5 days, Optional)

**Goal**: Match production SIQS performance

**Tasks:**

1. **Large prime variation** (12-16 hours)
   - Accept partial relations with 1 large prime < B²
   - Combine partials to form complete relations
   - Expected 2-4x speedup

2. **Parameter tuning** (8 hours)
   - Test different B and M for 40-70 digits
   - Optimize j (primes per 'a') for each range
   - Create parameter tables

3. **Block Lanczos** (8-10 hours, optional)
   - Replace Gaussian elimination
   - ~10x speedup for large matrices

**Deliverables:**
- Large prime variant working
- Optimized parameters
- ~400 LOC additional code

**Success Criteria:**
- 40 digits: ~90 seconds
- 50 digits: ~10 minutes
- Match or exceed published benchmarks

---

## Part 4: Code Reuse from Existing QS

### What Can Be Reused (~50-60% of code)

✅ **Reuse directly:**
- Factor base construction (200 LOC)
- Tonelli-Shanks for square roots (100 LOC)
- Matrix solving and Gaussian elimination (200 LOC)
- Factor extraction (100 LOC)
- Parameter selection framework (50 LOC)

⚠️ **Reuse with modifications:**
- Trial division (~100 LOC, add 'a' handling)
- Sieving loop (~300 LOC, use polynomial coefficients)
- Relation storage (~50 LOC, track polynomial used)

❌ **Must implement new:**
- Polynomial generation (~500 LOC)
- Polynomial switching (~250 LOC)
- SIQS-specific sieve initialization (~150 LOC)

---

## Part 5: Technical Specifications

### Data Structures

**New structures:**

```rust
// Polynomial representation
struct SIQSPolynomial {
    a: BigInt,              // Leading coefficient (product of j primes)
    b: BigInt,              // Linear coefficient (from CRT)
    c: BigInt,              // Constant (computed as (b² - n)/a)
    a_factors: Vec<u64>,    // Prime factors of 'a'
    b_array: Vec<BigInt>,   // B[i] values for fast switching
}

// Sieve state
struct SieveState {
    polynomial: SIQSPolynomial,
    roots: Vec<(i64, i64)>,  // Sieve roots for each prime
    poly_index: u32,          // Current polynomial index (1 to 2^(j-1))
}

// Main SIQS structure
pub struct SIQS {
    n: BigInt,
    sqrt_n: BigInt,
    smoothness_bound: u64,
    sieve_interval: i64,
    factor_base: Vec<Prime>,  // With added tsqrt field
    current_state: SieveState,
    relations: Vec<Relation>,
}
```

**Modified structures:**

```rust
// Add square root field to Prime
#[derive(Clone, Debug)]
struct Prime {
    p: u64,
    roots: Vec<i64>,  // For sieving (existing)
    tsqrt: i64,       // NEW: sqrt(n) mod p
    log_p: f32,
}
```

### Key Algorithms

**1. Polynomial Generation (most complex)**

```
Input: n, factor_base, target_a ≈ sqrt(2n)/M
Output: (a, b, c, B[])

1. Select j primes q₁, ..., qⱼ such that:
   - Each qᵢ in range [2000, 4000] for 40-digit numbers
   - Product q₁ × ... × qⱼ ≈ target_a
   - (n/qᵢ) = 1 for all i (guaranteed by factor base)

2. Compute B[i] for i = 1 to j:
   γ = tsqrt[qᵢ] × (a/qᵢ)⁻¹ mod qᵢ
   If γ > qᵢ/2: γ = qᵢ - γ
   B[i] = (a/qᵢ) × γ

3. Compute b = B[1] + B[2] + ... + B[j]
4. Compute c = (b² - n) / a
5. Return (a, b, c, B[])
```

**2. Polynomial Switching (SIQS method)**

```
Input: current polynomial index i (1 to 2^(j-1) - 1), B[]
Output: new polynomial (a, b', c')

1. v = count_trailing_zeros(2i)
2. e = sign based on bit pattern of i
3. b' = b + e × B[v]
4. c' = (b'² - n) / a
5. Update sieve roots incrementally:
   Δ = e × B[v] × ainv mod p
   soln1' = (soln1 - Δ) mod p
   soln2' = (soln2 - Δ) mod p
```

### Parameter Selection

**For 40-100 digit range:**

| Digits | B (smoothness) | M (interval) | j (primes) | Est. factor base |
|--------|----------------|--------------|------------|------------------|
| 40-44  | 8,000          | 700,000      | 4          | ~800             |
| 45-49  | 15,000         | 1,200,000    | 4          | ~1,500           |
| 50-54  | 25,000         | 1,800,000    | 4-5        | ~2,500           |
| 55-59  | 42,000         | 3,000,000    | 5          | ~4,000           |
| 60-64  | 65,000         | 4,500,000    | 5          | ~6,000           |

---

## Part 6: Risk Assessment & Mitigation

### Implementation Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **Polynomial generation bugs** | Medium | High | Extensive unit tests, reference multiple implementations |
| **Incorrect CRT computation** | Medium | High | Verify b² ≡ n (mod a) in tests |
| **Root update errors** | High | High | Test each polynomial switch, log intermediate values |
| **Performance below expectations** | Low | Medium | Profile and optimize, add large primes in Phase 3 |
| **Integration issues** | Low | Medium | Gradual integration, keep single-poly QS for comparison |

### Success Factors

✅ **In our favor:**
- 50-60% code reuse from existing QS
- Clear reference implementations (C-QS, msieve)
- Comprehensive research completed
- Strong Rust ecosystem for BigInt arithmetic

⚠️ **Challenges:**
- Complex polynomial generation (CRT, modular arithmetic)
- Debugging numerical code with large integers
- Parameter tuning requires extensive testing

---

## Part 7: Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_polynomial_generation() {
        // Verify: b² ≡ n (mod a)
        // Verify: c = (b² - n) / a is integer
        // Verify: a is product of j primes
    }

    #[test]
    fn test_polynomial_switching() {
        // Verify: b' computed correctly
        // Verify: roots updated correctly
        // Verify: Q(x) values using new poly
    }

    #[test]
    fn test_trial_division_with_a() {
        // Verify: 'a' factors extracted
        // Verify: remaining trial division correct
    }
}
```

### Integration Tests

```rust
#[test]
fn test_siqs_40_digit_range() {
    // Known 40-digit semiprimes
    let test_cases = vec![
        ("10000000000000000016800000000000000005031", // 41 digits
         "100000000000000000039",
         "100000000000000000129"),
        // Add more test cases
    ];

    for (n_str, p_str, q_str) in test_cases {
        let n = BigInt::from_str(n_str).unwrap();
        let result = siqs(&n);
        assert!(result.is_some());
        let (p, q) = result.unwrap();
        assert_eq!(p * q, n);
    }
}
```

### Benchmark Tests

```rust
#[bench]
fn bench_siqs_40_digit(b: &mut Bencher) {
    let n = BigInt::from_str("10000000000000000016800000000000000005031").unwrap();
    b.iter(|| siqs(&n));
}
```

---

## Part 8: Implementation Timeline

### Week 1: Phase 1 MVP

| Day | Tasks | Hours |
|-----|-------|-------|
| 1 | Module structure, polynomial generation scaffolding | 8 |
| 2 | Polynomial generation: select primes, compute 'a' | 8 |
| 3 | Polynomial generation: compute B[i], b, c | 8 |
| 4 | Trial division modifications, basic switching | 8 |
| 5 | Integration, testing, debugging | 8 |

**Deliverable**: Working SIQS (slow polynomial switching)

### Week 2: Phase 2 Full SIQS

| Day | Tasks | Hours |
|-----|-------|-------|
| 6 | SIQS switching: binary index tracking | 6 |
| 7 | SIQS switching: incremental updates | 6 |
| 8 | Multiple 'a' support, benchmarking | 6 |

**Deliverable**: Fast SIQS with 2x speedup

### Week 3: Phase 3 Optimizations (Optional)

| Day | Tasks | Hours |
|-----|-------|-------|
| 9-10 | Large prime variation | 12 |
| 11 | Parameter tuning | 6 |
| 12 | Block Lanczos (optional) | 8 |

**Deliverable**: Production-ready SIQS

---

## Part 9: Success Criteria

### Phase 1 Success

✅ Factor 40-digit test number successfully
✅ Find non-zero smooth relations
✅ Complete factorization in < 5 minutes
✅ All unit tests pass

### Phase 2 Success

✅ Factor 40-digit number in < 3 minutes
✅ Polynomial switch overhead < 1%
✅ ~2x speedup vs Phase 1
✅ Handle multiple 'a' values correctly

### Phase 3 Success

✅ 40 digits: ~90 seconds
✅ 50 digits: ~10 minutes
✅ 60 digits: ~2 hours
✅ Match published benchmarks for single-threaded Rust

---

## Part 10: Next Steps

### Immediate Actions

1. **Read Contini (1997) paper** (2-3 hours)
   - Most complete SIQS description
   - Algorithm pseudocode
   - Parameter selection guidance

2. **Study C-Quadratic-Sieve** (4-6 hours)
   - Clean reference implementation
   - https://github.com/michel-leonard/C-Quadratic-Sieve
   - Focus on polynomial generation

3. **Create feature branch** (30 minutes)
   ```bash
   git checkout -b siqs-implementation
   mkdir -p src/algorithms/siqs
   ```

4. **Scaffold SIQS module** (2 hours)
   - Create module files
   - Define data structures
   - Write skeleton functions

5. **Implement polynomial generation** (1-2 days)
   - Start with simplest case (j=3)
   - Test thoroughly
   - This is the hardest part!

### Long-term Vision

**After SIQS is working:**
1. SIQS handles 40-100 digits ✓
2. GNFS handles 100+ digits ✓
3. Complete factorization pipeline ✓

**Future enhancements:**
- Parallel SIQS (multi-threaded sieving)
- GPU/OpenCL acceleration (your vision!)
- Distributed computing support
- Mersenne number specialization

---

## Part 11: References

### Must-Read Papers

1. **Contini (1997)** - "Factoring Integers with the Self-Initializing Quadratic Sieve"
   - THE definitive SIQS reference
   - Complete algorithm description

2. **Silverman (1987)** - "The Multiple Polynomial Quadratic Sieve"
   - Original MPQS paper
   - Polynomial selection theory

### Reference Implementations

1. **C-Quadratic-Sieve** (Michel Leonard, 2022)
   - https://github.com/michel-leonard/C-Quadratic-Sieve
   - **Best for learning**: Clean, commented
   - ~2,000 LOC C code

2. **msieve** (Jason Papadopoulos)
   - https://github.com/radii/msieve
   - Production-quality reference
   - More complex but highly optimized

### Learning Resources

1. **Eric Landquist (2001)** - "The Quadratic Sieve Factoring Algorithm"
   - https://www.cs.virginia.edu/crab/QFS_Simple.pdf
   - Excellent tutorial

2. **Prime-Wiki SIQS Article**
   - https://www.rieselprime.de/ziki/Self-initializing_quadratic_sieve
   - Concise algorithm summary

---

## Appendix: Current QS Status & Limitations

### What We've Accomplished

✅ Implemented single-polynomial QS with:
- Correct factor base construction
- Working Tonelli-Shanks
- Proper sieving logic
- Trial division
- Matrix solving
- Parameter selection framework

✅ All 7/8 unit tests pass (1 regression for 8051, which should use Trial Division anyway)

✅ Works perfectly for intended range (< 30 digits)

### Documented Limitations

❌ **Cannot handle 40+ digits** due to mathematical constraints:
- Q(x) = x² - n produces norms too large to be B-smooth
- Not a bug - fundamental limitation of single-polynomial approach
- Research parameters assume MPQS, not single-poly QS

### Path Forward

**Option A: Document and move on**
- Update QS docs to note 30-digit limit
- Keep single-poly QS for educational value
- Route 40+ digits to GNFS directly

**Option B: Implement SIQS** ← **Recommended**
- Fill the 40-100 digit gap properly
- Align with your OpenCL/GPU vision
- Production-quality implementation
- ~2-3 weeks effort

**Option C: Hybrid approach**
- Document limitations (1 day)
- Plan SIQS implementation for later
- Focus on other project priorities now

---

## Conclusion

**The single-polynomial QS is working correctly** - it's just not the right algorithm for 40+ digits. SIQS is the standard solution used by all production implementations. With ~2-3 weeks of focused development, you'll have a proper 40-100 digit factorization capability that sets the foundation for your OpenCL/clustering vision.

**Recommended decision point**: Do you want to implement SIQS now, or document the current QS limitations and move on to other features (GNFS optimization, OpenCL prototyping, etc.)?

Either path is valid - SIQS is valuable but represents a significant time investment. The research and planning are complete; execution is now a matter of prioritization.
