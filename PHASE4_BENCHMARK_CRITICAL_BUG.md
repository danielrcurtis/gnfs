# Phase 4 Benchmark Results - CRITICAL BUG DISCOVERED

## Executive Summary

**STATUS: BLOCKING BUG - Phase 4 Implementation Cannot Function**

The Phase 4 adaptive architecture implementation has a **critical design flaw** that prevents it from running:
- Native backends (Native64, Native128) use unsigned types (u64, u128)
- GNFS algorithm requires **negative values** for relation parameters `a` and `b`
- Conversion from negative BigInt to unsigned native types **fails**, causing panic on every relation

**Result**: Benchmarks cannot run. All 100+ parallel threads panic immediately with "Failed to convert a to native type".

## System Information
- **Date**: 2025-10-23
- **Hostname**: Daniels-MBP.lan
- **OS**: Darwin 25.0.0 (macOS)
- **CPU**: Apple M3 Pro (12 cores)
- **Memory**: 18432 MB
- **Rust**: rustc 1.89.0
- **Git commit**: a1863ab (main)

## Benchmark Attempt

### Command
```bash
MY_LOG_LEVEL=info ./target/release/gnfs --bench 9 11
```

### Target Numbers
- 9-digit: 100085411
- 11-digit: 10003430467

### Result
**FAILED** - Immediate panic in all threads

### Error Log
```
thread '<unnamed>' panicked at src/relation_sieve/relation.rs:29:34:
Failed to convert a to native type
```

This error occurred in **all parallel threads** (100+ concurrent panics), indicating a systematic failure in the core architecture.

## Root Cause Analysis

### 1. The Mathematical Requirement

GNFS sieving requires exploring both positive and negative values of `a`:

**File**: `/Users/danielcurtis/source/gnfs/src/Core/sieve_range.rs` (lines 15-39)

```rust
pub fn get_sieve_range_continuation(
    current_value: &BigInt,
    maximum_range: &BigInt,
) -> impl Iterator<Item = BigInt> {
    let max = maximum_range.clone();
    let mut counter = current_value.abs();
    let mut flip_flop = current_value.sign() != Sign::Minus;

    std::iter::from_fn(move || {
        if counter <= max {
            let result = if flip_flop {
                Some(counter.clone())
            } else {
                Some(-&counter)  // ← NEGATIVE VALUES GENERATED HERE
            };
            if !flip_flop {
                counter += 1;
            }
            flip_flop = !flip_flop;
            result
        } else {
            None
        }
    })
}
```

**Generated sequence**: `{1, -1, 2, -2, 3, -3, 4, -4, ...}`

This is **mathematically required** - GNFS must search both positive and negative `a` values to find smooth relations.

### 2. The Conversion Failure

**File**: `/Users/danielcurtis/source/gnfs/src/relation_sieve/relation.rs` (lines 26-40)

```rust
impl<T: GnfsInteger> Relation<T> {
    pub fn new(_gnfs: &GNFS<T>, a: &BigInt, b: &BigInt) -> Self {
        Relation {
            a: T::from_bigint(a).expect("Failed to convert a to native type"),  // ← PANIC HERE
            b: T::from_bigint(b).expect("Failed to convert b to native type"),
            // ...
        }
    }
}
```

When `a` is negative (e.g., `-1`, `-2`, `-3`), the conversion attempts to convert to native type.

### 3. The Native64 Implementation

**File**: `/Users/danielcurtis/source/gnfs/src/backends/native64.rs` (lines 28-30)

```rust
impl GnfsInteger for Native64 {
    fn from_bigint(n: &BigInt) -> Option<Self> {
        n.to_u64().map(Native64)  // ← Returns None for negative values
    }
```

**The problem**: `BigInt::to_u64()` returns `None` for negative numbers because `u64` is **unsigned**.

When the `.expect()` unwraps a `None`, it panics with "Failed to convert a to native type".

### 4. Scope of Negative Values in GNFS

Analysis of the codebase reveals negative values are used in multiple places:

#### a. Relation parameters (a, b)
- **File**: `relation.rs`, `poly_relations_sieve_progress.rs`
- **Range**: `a ∈ [-value_range, +value_range]`, typically `[-200, 200]`
- **Frequency**: Every single relation tested

#### b. Norms (rational_norm, algebraic_norm)
- **File**: `relation.rs` (lines 80-82, 150-152)
- Norms can be negative and require special handling:
```rust
if rational_norm_for_comparison < BigInt::zero() {
    self.rational_factorization.add(&BigInt::from(-1));
}
```
- Absolute values taken for factorization (lines 85, 154)

#### c. Polynomial evaluation
- **File**: `relation.rs` (lines 118-120)
```rust
let a_bigint = self.a.to_bigint();
let neg_a = -(&a_bigint);  // Negative values used in computation
let ab_ratio = BigRational::new(neg_a, b_bigint.clone());
```

## Why This Wasn't Caught Earlier

1. **No integration tests**: Phase 4 had no end-to-end tests calling `find_relations()`
2. **Build succeeded**: The generic architecture compiles cleanly - the error is runtime
3. **Unit tests incomplete**: Existing tests use small positive-only values
4. **Design review gap**: The signed/unsigned nature of GNFS values wasn't analyzed

## Impact Assessment

### What Works
- ✅ Backend selection (Native64 correctly chosen for 9 and 11 digit numbers)
- ✅ Initialization (polynomial selection, factor base construction)
- ✅ Logging (backend selection messages appear correctly)
- ✅ Type system (generics compile and dispatch correctly)

### What Fails
- ❌ **All sieving** (100% failure rate on every relation)
- ❌ **All benchmarks** (cannot complete any factorization)
- ❌ **All real usage** (same code path as benchmarks)

### Performance Impact
**Cannot be measured** - system fails before any work completes.

## Solution Options

### Option 1: Use Signed Native Types (RECOMMENDED)

**Change**: Replace `u64`/`u128` with `i64`/`i128` in native backends

**Pros**:
- Handles negative values correctly
- Still provides significant performance benefit (native arithmetic)
- Memory savings: `i64` (8 bytes) vs BigInt (~150 bytes) = 18.75x reduction
- Performance: Native signed arithmetic still ~50x faster than BigInt

**Cons**:
- Halves the positive range (i64 max = 9.2×10^18 vs u64 max = 1.8×10^19)
- For 11-digit numbers: max algebraic norm ≈ 2.8×10^17 (well within i64 range)

**Implementation**:
1. Create `Native64Signed` using `i64` instead of `u64`
2. Update `from_bigint()` to use `to_i64()` instead of `to_u64()`
3. Adjust arithmetic operations (already signed-aware in i64)
4. Update backend selection to choose signed variants

**Estimated effort**: 2-3 hours

### Option 2: Translate Value Ranges

**Change**: Add offset to make all values positive during computation

**Pros**:
- Can keep unsigned types
- Mathematically sound (translation preserves relationships)

**Cons**:
- Complex: Must track offset through all operations
- Error-prone: Easy to forget translation in one place
- Norm calculations still produce negative values (would need separate handling)
- Doesn't solve norm sign issue

**Not recommended**: Too complex and doesn't fully solve the problem.

### Option 3: Hybrid Approach

**Change**: Keep BigInt for `a`/`b`, use native types only for norms/quotients

**Pros**:
- Avoids signed/unsigned issue for relation parameters
- Still accelerates norm computations (the expensive part)

**Cons**:
- Partial optimization only
- Still need to handle negative norms
- More complex implementation (two type systems)
- Less memory savings (a/b still use BigInt)

**Not recommended**: Complexity outweighs benefits.

### Option 4: Fallback to BigInt Backend

**Change**: Disable Native64/Native128 backends, use only BigIntBackend

**Pros**:
- Works correctly (proven by previous sessions)
- No code changes needed

**Cons**:
- **Abandons all Phase 4 work** (~40 hours of development)
- No performance improvement
- No memory savings
- Returns to 60GB memory spikes for 11-digit numbers

**Last resort only**: If Option 1 proves difficult.

## Recommended Action Plan

### Immediate (Today)
1. **Implement Option 1**: Convert native backends to signed types
2. **Test conversion**: Verify i64 can handle expected value ranges
3. **Run benchmarks**: Measure actual performance with signed types

### Short Term (This Week)
1. **Add integration tests**: Test sieving with negative values
2. **Verify all backends**: Ensure Native128, Fixed256, Fixed512 also handle negatives
3. **Document range limits**: Clarify max safe values for each backend

### Long Term (Next Sprint)
1. **Add validation**: Check value ranges during backend selection
2. **Improve error messages**: Convert panics to Results with clear messages
3. **Performance analysis**: Compare i64 vs u64 performance on actual GNFS workload

## Estimated Performance with Signed Types

### Memory (vs BigInt)
- **Per Relation**: 6 fields × (8 bytes i64 vs ~150 bytes BigInt) = **18.75x reduction**
- **11-digit factorization**: 1M relations × 900 bytes saved = ~850MB savings (vs current ~1GB+)

### Speed (vs BigInt)
- **Native i64 arithmetic**: Still ~50x faster than BigInt operations
- **No heap allocations**: i64 is stack-allocated (zero allocations per operation)
- **SIMD potential**: i64 operations can use CPU vector instructions

### Expected Benchmarks (After Fix)
- **9-digit (100085411)**: Target <1 second (vs 2-28s with BigInt)
- **11-digit (10003430467)**: Target <60 seconds (vs 90s+ with BigInt)

**Note**: These are projections - actual benchmarks will be run after implementing Option 1.

## Range Validation

### i64 Capacity Analysis

For **11-digit numbers** (max = 99,999,999,999):

**Algebraic norm calculation**:
```
norm ≈ N^(1 + 1/degree) ≈ (10^11)^(1 + 1/3) ≈ (10^11)^1.33 ≈ 2.8 × 10^14
```

**i64 range**:
```
i64::MIN = -9,223,372,036,854,775,808 ≈ -9.2 × 10^18
i64::MAX = +9,223,372,036,854,775,807 ≈ +9.2 × 10^18
```

**Safety margin**:
```
2.8 × 10^14 / 9.2 × 10^18 ≈ 0.003%
```

The largest expected algebraic norm is **3000x smaller** than i64::MAX - plenty of headroom.

### For larger numbers

**14-digit numbers** (max = 99,999,999,999,999):
```
norm ≈ (10^14)^1.33 ≈ 4.6 × 10^18
```

This is **approaching i64::MAX** (9.2×10^18) - would need careful monitoring or switch to i128/Fixed256.

**Conclusion**: i64 is safe for up to ~13 digits, i128 needed for 14+ digits.

## Lessons Learned

### What Went Wrong
1. **Insufficient testing**: No integration tests for the hot path
2. **Unsigned assumption**: Incorrectly assumed all GNFS values are positive
3. **Range analysis missed**: Didn't consider sign requirements during design
4. **Conversion unchecked**: Used `.expect()` instead of `.ok()` + fallback

### Improvements for Future Phases
1. **Write integration tests first**: Test actual sieving before optimizing
2. **Analyze value domains**: Document sign/range requirements for all types
3. **Graceful degradation**: Use Result types, fall back to BigInt on overflow
4. **Incremental rollout**: Test each backend individually before full integration

## Next Steps

1. **Create `native64_signed.rs`**: Copy `native64.rs`, replace u64 with i64
2. **Update `GnfsInteger` impl**: Change `to_u64()` → `to_i64()`
3. **Test range validation**: Verify 11-digit numbers fit in i64
4. **Run benchmarks**: Measure actual performance vs theoretical
5. **Update documentation**: Clarify signed vs unsigned for each backend

## References

### Key Files
- `/Users/danielcurtis/source/gnfs/src/backends/native64.rs` - Native64 implementation
- `/Users/danielcurtis/source/gnfs/src/relation_sieve/relation.rs` - Relation struct (line 29 panic)
- `/Users/danielcurtis/source/gnfs/src/Core/sieve_range.rs` - Negative value generation
- `/Users/danielcurtis/source/gnfs/src/core/gnfs_wrapper.rs` - Backend selection

### Related Documents
- `ADAPTIVE_ARCHITECTURE_REPORT.md` - Phase 4 design and rationale
- `CLAUDE.md` - Project overview and benchmarking guide

---

**Status**: Awaiting fix (Option 1 implementation recommended)

**Blocking**: All Phase 4 performance measurements

**Priority**: CRITICAL - blocks all benchmarking and production use
