# Stage 4 (Square Root Extraction) Performance Issue

## Problem Identified

After adding progress logging to `square_finder.rs`, we discovered that **Stage 4 is NOT hanging** - it's running but extremely slowly and not finding any irreducible primes.

### Observations

From test run with GNFS_THREADS=2 on N=45113:
- Tested **4,300 primes** in ~15 seconds
- Found **0 irreducible primes**
- Current prime tested: **41,911**
- Starting prime: **424** (2 × sqrt(45113))

### Root Cause

The irreducibility test at `square_finder.rs:264-267` computes:
```rust
let g = Polynomial::parse(&format!("X^{} - X", last_p_i128));
```

This creates a polynomial `X^p - X` where p can be **40,000+**. Computing modular operations on such a high-degree polynomial is:
1. **Extremely slow** (each test takes milliseconds)
2. **Memory intensive**
3. **Possibly incorrect** - the algorithm may not be working as intended

## Why No Irreducible Primes Found?

After testing 4,300 primes without finding a single irreducible prime, there are several possibilities:

1. **Algorithmic Bug**: The irreducibility test implementation may be incorrect
2. **Modular Arithmetic Issue**: The `mod_mod` or `field_gcd` functions may have bugs
3. **Polynomial Issue**: The polynomial f(X) = X³ + 15X² + 29X + 8 may have special properties
4. **Starting Point Too High**: Starting at p=424 may skip smaller irreducible primes

## Expected Behavior (from C# Reference)

In the C# implementation, irreducible primes should be found relatively quickly. For a degree-3 polynomial:
- Need 3 irreducible primes
- Typically found within the first few hundred primes tested
- Should start finding them almost immediately

## Current Performance Impact

- **~100 primes/second** testing rate
- **0% success rate** (0 irreducible primes found)
- At this rate, it would take hours or days to search higher

## Recommended Solutions

### Short-term Fix (Diagnostic)
1. Add logging to show GCD results for first few primes
2. Check if `field_gcd` is always returning something != 1
3. Verify the irreducibility test is correctly implemented

### Medium-term Fix
1. **Optimize polynomial modular exponentiation** - Use fast exponentiation algorithm
2. **Pre-compute X^p mod f** incrementally instead of parsing huge polynomial strings
3. **Use a more efficient irreducibility test**

### Long-term Fix
Look at the C# reference implementation's irreducibility test and ensure exact correspondence.

## Testing Notes

The progress logging successfully reveals:
- The algorithm is not hung - it's actively testing primes
- Every 10 primes, it logs progress
- The issue is NOT parallelization - it's the algorithmic approach

## Files Modified for Diagnosis

- `src/square_root/square_finder.rs:248-265` - Added progress logging
  - Logs search start parameters
  - Logs every 10 primes tested with current prime value
  - Shows how many irreducible primes found (always 0 in our test)
