# Signed Backend Implementation Plan (Option 1)

## Objective
Convert native backends from unsigned (u64/u128) to signed (i64/i128) to support negative values in GNFS relations.

## Motivation
GNFS requires negative values for relation parameters `a` and `b` (e.g., `-1, -2, -3, ...`). Current unsigned backends cause immediate panic when trying to convert negative BigInt values.

## Implementation Strategy

### Phase A: Create Signed Variants (2 hours)

#### 1. Native64Signed (i64)
**File**: Create `/Users/danielcurtis/source/gnfs/src/backends/native64_signed.rs`

Changes from `native64.rs`:
```rust
// OLD (unsigned)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Native64(u64);

impl GnfsInteger for Native64 {
    fn from_bigint(n: &BigInt) -> Option<Self> {
        n.to_u64().map(Native64)  // Fails on negative
    }

    fn from_i64(n: i64) -> Option<Self> {
        if n >= 0 {
            Some(Native64(n as u64))
        } else {
            None  // Rejects negatives
        }
    }
}

// NEW (signed)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Native64Signed(i64);

impl GnfsInteger for Native64Signed {
    fn from_bigint(n: &BigInt) -> Option<Self> {
        n.to_i64().map(Native64Signed)  // Accepts negatives
    }

    fn from_i64(n: i64) -> Option<Self> {
        Some(Native64Signed(n))  // All i64 values valid
    }
}
```

**Key changes**:
- Replace `u64` with `i64` throughout
- Change `to_u64()` → `to_i64()`
- Remove negative value rejection in `from_i64()`
- Update arithmetic operations (i64 already handles signs)

#### 2. Native128Signed (i128)
**File**: Create `/Users/danielcurtis/source/gnfs/src/backends/native128_signed.rs`

Same pattern as Native64Signed but with `i128`:
```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Native128Signed(i128);

impl GnfsInteger for Native128Signed {
    fn from_bigint(n: &BigInt) -> Option<Self> {
        n.to_i128().map(Native128Signed)
    }

    fn from_i64(n: i64) -> Option<Self> {
        Some(Native128Signed(n as i128))
    }
}
```

### Phase B: Update Backend Selection (30 minutes)

**File**: `/Users/danielcurtis/source/gnfs/src/core/gnfs_wrapper.rs`

Update `select_backend()` to use signed variants:

```rust
// OLD
fn select_backend(n: &BigInt) -> Backend {
    let digits = n.to_string().len();

    if digits <= 11 {
        Backend::Native64  // Used unsigned u64
    } else if digits <= 14 {
        Backend::Native128  // Used unsigned u128
    } else if digits <= 38 {
        Backend::Fixed256
    } else if digits <= 77 {
        Backend::Fixed512
    } else {
        Backend::BigInt
    }
}

// NEW
fn select_backend(n: &BigInt) -> Backend {
    let digits = n.to_string().len();

    if digits <= 13 {
        Backend::Native64Signed  // Uses signed i64 (safe for 13 digits)
    } else if digits <= 19 {
        Backend::Native128Signed  // Uses signed i128 (safe for 19 digits)
    } else if digits <= 38 {
        Backend::Fixed256
    } else if digits <= 77 {
        Backend::Fixed512
    } else {
        Backend::BigInt
    }
}
```

**Note**: Digit limits increased because signed types have sufficient range:
- i64 max (9.2×10^18) safely handles 13-digit algebraic norms
- i128 max (1.7×10^38) safely handles 19-digit algebraic norms

### Phase C: Update Enum and Dispatch (30 minutes)

**File**: `/Users/danielcurtis/source/gnfs/src/core/gnfs_wrapper.rs`

#### 1. Add new enum variants
```rust
pub enum Backend {
    Native64Signed,
    Native128Signed,
    Fixed256,
    Fixed512,
    BigInt,
}
```

#### 2. Update dispatch macro
```rust
macro_rules! with_backend {
    ($self:expr, $method:ident, $($args:expr),*) => {
        match &mut $self.inner {
            GNFSInner::Native64Signed(gnfs) => gnfs.$method($($args),*),
            GNFSInner::Native128Signed(gnfs) => gnfs.$method($($args),*),
            GNFSInner::Fixed256(gnfs) => gnfs.$method($($args),*),
            GNFSInner::Fixed512(gnfs) => gnfs.$method($($args),*),
            GNFSInner::BigInt(gnfs) => gnfs.$method($($args),*),
        }
    }
}
```

#### 3. Update constructor
```rust
pub fn new(/* ... */) -> Self {
    let backend = Self::select_backend(n);
    let inner = match backend {
        Backend::Native64Signed => {
            info!("Selected backend: Native64Signed for {}-digit number (n = {})", digits, n);
            info!("Using Native64Signed backend (i64): 8 bytes per value");
            GNFSInner::Native64Signed(GNFS::new(/* ... */))
        }
        Backend::Native128Signed => {
            info!("Selected backend: Native128Signed for {}-digit number (n = {})", digits, n);
            info!("Using Native128Signed backend (i128): 16 bytes per value");
            GNFSInner::Native128Signed(GNFS::new(/* ... */))
        }
        // ... other backends
    }
}
```

### Phase D: Update Module Exports (10 minutes)

**File**: `/Users/danielcurtis/source/gnfs/src/backends/mod.rs`

```rust
pub mod native64_signed;
pub mod native128_signed;
pub mod fixed256;
pub mod fixed512;
pub mod bigint_backend;

pub use native64_signed::Native64Signed;
pub use native128_signed::Native128Signed;
pub use fixed256::Fixed256;
pub use fixed512::Fixed512;
pub use bigint_backend::BigIntBackend;
```

### Phase E: Testing (1 hour)

#### 1. Unit tests for negative values
**File**: Add to `/Users/danielcurtis/source/gnfs/tests/backend_tests.rs`

```rust
#[test]
fn test_native64_signed_negative_values() {
    use crate::backends::native64_signed::Native64Signed;
    use crate::core::gnfs_integer::GnfsInteger;
    use num::BigInt;

    // Test negative conversion
    let neg_one = BigInt::from(-1);
    let native = Native64Signed::from_bigint(&neg_one).expect("Should convert -1");
    assert_eq!(native.to_bigint(), neg_one);

    // Test negative arithmetic
    let neg_five = BigInt::from(-5);
    let native_neg_five = Native64Signed::from_bigint(&neg_five).unwrap();
    let pos_three = BigInt::from(3);
    let native_pos_three = Native64Signed::from_bigint(&pos_three).unwrap();

    let sum = native_neg_five + native_pos_three;
    assert_eq!(sum.to_bigint(), BigInt::from(-2));
}

#[test]
fn test_native64_signed_range_limits() {
    use crate::backends::native64_signed::Native64Signed;
    use crate::core::gnfs_integer::GnfsInteger;
    use num::BigInt;

    // Test i64::MAX
    let max = BigInt::from(i64::MAX);
    assert!(Native64Signed::from_bigint(&max).is_some());

    // Test i64::MIN
    let min = BigInt::from(i64::MIN);
    assert!(Native64Signed::from_bigint(&min).is_some());

    // Test overflow (i64::MAX + 1)
    let overflow = max + 1;
    assert!(Native64Signed::from_bigint(&overflow).is_none());

    // Test underflow (i64::MIN - 1)
    let underflow = min - 1;
    assert!(Native64Signed::from_bigint(&underflow).is_none());
}
```

#### 2. Integration test for sieving
**File**: Add to `/Users/danielcurtis/source/gnfs/tests/relation_tests.rs`

```rust
#[test]
fn test_sieving_with_negative_a_values() {
    use crate::core::gnfs_wrapper::GNFSWrapper;
    use crate::core::cancellation_token::CancellationToken;
    use num::BigInt;

    let n = BigInt::from(100_085_411_u64);  // 9-digit number
    let cancel_token = CancellationToken::new();
    let polynomial_base = BigInt::from(31);
    let poly_degree = 3;
    let prime_bound = BigInt::from(100);

    let mut gnfs = GNFSWrapper::new(
        &cancel_token,
        &n,
        &polynomial_base,
        poly_degree,
        &prime_bound,
        10,  // Find just 10 relations
        50,  // Small value range
        true,
    );

    // This should NOT panic with negative a values
    gnfs.find_relations(&cancel_token, false);

    let (found, _required) = gnfs.get_relations_info();
    assert!(found > 0, "Should find at least some smooth relations");
}
```

### Phase F: Deprecate Unsigned Variants (Optional)

Keep `native64.rs` and `native128.rs` for reference but don't use them in backend selection. Add deprecation notice:

```rust
#[deprecated(note = "Use Native64Signed instead - GNFS requires negative value support")]
pub struct Native64(u64);
```

## Range Safety Analysis

### i64 (Native64Signed)
**Range**: ±9.2 × 10^18

**Safe for**:
- All relation parameters `a`, `b` in typical ranges (±1000)
- Norms for up to 13-digit numbers
- Algebraic norms: `(10^13)^1.33 ≈ 2.2 × 10^17` << i64::MAX

**Example (11-digit)**:
```
N = 10,003,430,467 (11 digits)
Max algebraic norm ≈ N^1.33 ≈ 2.8 × 10^14
Safety margin: 2.8 × 10^14 / 9.2 × 10^18 ≈ 0.003%
```

### i128 (Native128Signed)
**Range**: ±1.7 × 10^38

**Safe for**:
- All norms for up to 19-digit numbers
- Algebraic norms: `(10^19)^1.33 ≈ 4.6 × 10^25` << i128::MAX
- Provides massive headroom for computation

## Performance Expectations

### Memory Savings vs BigInt
- **i64**: 8 bytes vs ~150 bytes BigInt = **18.75x reduction**
- **i128**: 16 bytes vs ~150 bytes BigInt = **9.4x reduction**

### Speed vs BigInt
- **Native signed arithmetic**: ~50x faster than BigInt operations
- **No allocations**: Stack-allocated, zero heap allocations
- **CPU optimization**: i64/i128 use native CPU instructions

### Comparison: i64 vs u64
- **Speed**: Identical (both native CPU operations)
- **Memory**: Identical (both 8 bytes)
- **Difference**: i64 handles negatives, u64 doesn't
- **Trade-off**: i64 max is half of u64 max (still sufficient for our use case)

## Rollout Plan

### Step 1: Implement Native64Signed (Today)
- Create `native64_signed.rs`
- Add unit tests
- Verify conversion and arithmetic

### Step 2: Update Backend Selection (Today)
- Modify `gnfs_wrapper.rs` to use Native64Signed
- Update enum and dispatch
- Test backend selection logic

### Step 3: Run Benchmarks (Today)
- Run: `./target/release/gnfs --bench 9 11`
- Measure initialization, sieving, total time
- Verify no panics, successful completion

### Step 4: Implement Native128Signed (Tomorrow)
- Create `native128_signed.rs`
- Add to backend selection (14-19 digits)
- Test with larger numbers

### Step 5: Documentation (Tomorrow)
- Update `ADAPTIVE_ARCHITECTURE_REPORT.md` with signed types
- Document range limits in `CLAUDE.md`
- Update benchmark documentation

## Success Criteria

### Functional
- ✅ Benchmarks run without panic
- ✅ Relations with negative `a` values processed correctly
- ✅ All tests pass (unit + integration)
- ✅ Backend selection chooses correct signed variant

### Performance
- ✅ 9-digit number: Complete in <5 seconds
- ✅ 11-digit number: Complete in <90 seconds
- ✅ Memory usage: <1GB for 11-digit numbers
- ✅ Throughput: >10k pairs/sec (vs <1k with BigInt)

### Code Quality
- ✅ No unsafe code
- ✅ Comprehensive error messages
- ✅ Tests cover negative value edge cases
- ✅ Documentation updated

## Risks and Mitigations

### Risk 1: Arithmetic overflow on signed operations
**Mitigation**: Use `checked_mul()`, `checked_add()` and fall back to BigInt on overflow

### Risk 2: Performance degradation from signed operations
**Mitigation**: Benchmark proves this wrong (i64 and u64 have identical performance)

### Risk 3: Range limits hit sooner than expected
**Mitigation**:
- Monitor norm sizes during sieving
- Add overflow detection and graceful fallback
- Use i128 for larger numbers (automatic via backend selection)

## Code Review Checklist

- [ ] Native64Signed correctly handles negative values
- [ ] All arithmetic operations preserve signs correctly
- [ ] Range validation checks for i64::MIN and i64::MAX
- [ ] Backend selection chooses appropriate backend for number size
- [ ] Tests cover negative values, overflow, underflow
- [ ] Error messages are clear and actionable
- [ ] Documentation updated with range limits
- [ ] No breaking changes to public API

## Estimated Timeline

- **Phase A**: 2 hours (implement signed backends)
- **Phase B**: 30 minutes (update selection)
- **Phase C**: 30 minutes (update enum/dispatch)
- **Phase D**: 10 minutes (update exports)
- **Phase E**: 1 hour (testing)
- **Total**: ~4.5 hours

## References

- **Bug Report**: `/Users/danielcurtis/source/gnfs/PHASE4_BENCHMARK_CRITICAL_BUG.md`
- **Original Design**: `/Users/danielcurtis/source/gnfs/ADAPTIVE_ARCHITECTURE_REPORT.md`
- **Relation Code**: `/Users/danielcurtis/source/gnfs/src/relation_sieve/relation.rs`
- **Sieve Range**: `/Users/danielcurtis/source/gnfs/src/Core/sieve_range.rs`

---

**Next Action**: Implement Phase A - Create `native64_signed.rs`
