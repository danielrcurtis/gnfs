# Discrepancies Found Between C# and Rust Implementations

## Bugs Fixed

### 1. ✅ FIXED: MaxB Increment Value
**File**: `src/relation_sieve/poly_relations_sieve_progress.rs:107`

**Bug**:
```rust
self.max_b += 1000;  // WRONG
```

**Fix**:
```rust
self.max_b += 100;  // Matches C# implementation
```

**C# Reference** (`PolyRelationsSieveProgress.cs:124`):
```csharp
MaxB += 100;
```

**Impact**: This was causing the sieving to skip large ranges of B values, significantly reducing the search space for smooth relations.

---

## Other Potential Discrepancies to Investigate

### 2. Sieving Order: Rational Before Algebraic?
**C# Code** (`Relation.cs:127-135`):
```csharp
public void Sieve(PolyRelationsSieveProgress relationsSieve)
{
    Sieve(relationsSieve._gnfs.PrimeFactorBase.RationalFactorBase, ref RationalQuotient, RationalFactorization);

    if (IsRationalQuotientSmooth) // Only sieve algebraic if rational is smooth
    {
        Sieve(relationsSieve._gnfs.PrimeFactorBase.AlgebraicFactorBase, ref AlgebraicQuotient, AlgebraicFactorization);
    }
}
```

**Rust Code** (`src/relation_sieve/relation.rs:95-110`):
Currently factorizes both simultaneously. Should add the optimization to check rational first.

**Recommendation**: Add early termination - if rational isn't smooth, skip algebraic.

---

### 3. Early Termination in Sieve Method
**C# Code** (`Relation.cs:151-159`):
```csharp
if ((factor * factor) > quotientValue)
{
    if (primeFactors.Contains(quotientValue))
    {
        dictionary.Add(quotientValue);
        quotientValue = 1;
    }
    return;  // Early termination
}
```

**Rust Code**: Implemented in `factor_with_base()` but should verify correctness.

---

### 4. Sieve Range Generation
**C# Code** (`PolyRelationsSieveProgress.cs:141`):
```csharp
foreach (BigInteger a in SieveRange.GetSieveRangeContinuation(A, ValueRange))
```

Need to verify `SieveRange` logic matches between implementations.

---

### 5. Coprimality Check
**C# Code** (`PolyRelationsSieveProgress.cs:149`):
```csharp
if (GCD.AreCoprime(A, B))
```

**Rust Code** (`poly_relations_sieve_progress.rs:133`):
```rust
if GCD::are_coprime(&[self.a.clone(), self.b.clone()])
```

Should verify the GCD implementation is correct and matches C# behavior.

---

### 6. Relation Counter Update
**C# Code** (`PolyRelationsSieveProgress.cs:158-160`):
```csharp
if (smooth)
{
    Serialization.Save.Relations.Smooth.Append(_gnfs, rel);
    _gnfs.CurrentRelationsProgress.Relations.SmoothRelations.Add(rel);
}
```

The counter is updated in the `Append` method, not directly incremented.

**Rust Code** (`poly_relations_sieve_progress.rs:139-140`):
```rust
self.relations.smooth_relations.push(rel);
self.smooth_relations_counter += 1;
```

This looks correct, but verify counter is accurate.

---

### 7. Value Range Increment
**C# Code** (`PolyRelationsSieveProgress.cs:112-115`):
```csharp
if (A >= ValueRange)
{
    ValueRange += 200;
}
```

**Rust Code** (`poly_relations_sieve_progress.rs:88-90`):
```rust
if self.a >= self.value_range {
    self.value_range += BigInt::from(200);
}
```

✅ This matches.

---

### 8. Odd Value Enforcement
**C# Code** (`PolyRelationsSieveProgress.cs:117-118`):
```csharp
ValueRange = (ValueRange % 2 == 0) ? ValueRange + 1 : ValueRange;
A = (A % 2 == 0) ? A + 1 : A;
```

**Rust Code** (`poly_relations_sieve_progress.rs:92-101`):
```rust
self.value_range = if self.value_range.is_even() {
    &self.value_range + 1
} else {
    self.value_range.clone()
};

self.a = if self.a.is_even() {
    &self.a + 1
} else {
    self.a.clone()
};
```

✅ This matches.

---

## Priority Fixes

1. ✅ **MaxB increment** - FIXED (was += 1000, now += 100)
2. **Add rational-first optimization** in sieve method
3. **Verify SieveRange logic** matches C# implementation
4. **Test with fixed MaxB** to see if smooth relations are found

---

## Testing Strategy

Run tests with fixed MaxB:
1. `test_option1_larger_prime_bounds` - Should find relations with prime_bound=500
2. `test_option2_simpler_number` - Should find relations with N=143
3. `test_verify_first_relations` - Manual verification of norm computations

If still no smooth relations found, investigate:
- Polynomial construction (verify coefficients match)
- Factor base construction (verify correct primes included)
- Norm computation (add detailed logging for first 10 relations)
