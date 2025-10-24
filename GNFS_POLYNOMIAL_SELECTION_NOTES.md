# GNFS Polynomial Selection Enhancement Notes

## Core Objective

The polynomial selection process in GNFS is the most critical phase for overall performance. The quality of selected polynomials largely determines sieving requirements and total runtime.

Given integer `n` to factor, find two coprime, irreducible polynomials:
- `f(x), g(x) ∈ Z[x]`
- With common root modulo `n`: `f(m) ≡ g(m) ≡ 0 (mod n)`

**Typical choices**:
- `deg(g) = 1` and `deg(f)` between 4 and 6
- `Res(f,g)` should equal (or be close to) ±n
- Quality influences how often `F(a,b)` and `G(a,b)` yield smooth values during sieving

## CADO-NFS Approach

Uses combination of:
- Kleinjung's base-m construction
- Murphy's α-score evaluation
- Lattice-based optimization methods

### 1. Polynomial Generation

Start with `m ≈ n^(1/d)`

Express `n` in base `m`:
```
n = a_d·m^d + a_(d-1)·m^(d-1) + ... + a_0
```

Yielding:
- `f(x) = a_d·x^d + ... + a_0`
- `g(x) = x - m`

Iterate over many `m` and degrees `d`, collecting thousands to millions of candidate pairs.

### 2. Norm and Size Optimization

For each `f(x)`, compute skew parameter `s` that minimizes skewed infinity norm:
```
||f||_{s,∞} = max_i |a_i|·s^(i-d/2)
```

Smaller norms → smaller coefficient magnitudes → smoother values when evaluating `F(a,b)`.

### 3. Murphy's α-Score

Evaluate each polynomial using Murphy's α-value:
- Captures how favorable polynomial's root distribution is modulo small primes
- Lower α → more smooth relations on average
- CADO-NFS estimates efficiently through modular sieving of `f(a,b)` for small primes

### 4. Root Optimization

Apply exhaustive or lattice-based refinement:
- Translation, rotation, scaling
- Reduces L2-norm and improves α simultaneously
- Kleinjung and Bai's methods:
  - "Rotation" via `f(x+k)`
  - Selective coefficient reduction

### 5. Ranking and Validation

Best candidates:
1. Ranked by combined size and α-scores (empirical scoring functions)
2. Top ≈10-20 tested with small-scale "test sieving"
3. Confirm real smoothness yield before full computation

## Implementation Guidelines (Rust)

### Key Considerations:
1. **Use modular arithmetic and arbitrary-precision integers**
   - Libraries: `rug` or `num-bigint`

2. **Parallelize α-score evaluation**
   - Job queue model with `rayon`

3. **Efficient computation**
   - Skewed norm and α computation dominate runtime
   - Optimize these heavily

4. **Adjustable parameters**
   - Degree, skew, and search bounds for experimentation

5. **Caching**
   - Serialize best candidates to avoid recomputation

## Summary Table

| Phase | Purpose | Key Technique | Effect |
|-------|---------|---------------|--------|
| Polynomial generation | Produce base candidates | Base-m expansion | Defines degree and structure |
| Size optimization | Minimize coefficient norms | Skew adjustment | Reduces average norms |
| α-evaluation | Estimate smoothness yield | Murphy's α | Predicts sieving effort |
| Root optimization | Fine-tune coefficients | Rotation / lattice reduction | Maximizes smoothness chances |
| Validation | Confirm efficacy | Test sieving | Picks top performers |

## Key Insight

Polynomial selection balances **coefficient size** and **root properties** to accelerate sieving. It's a blend of:
- Number-theoretical insight
- Numerical optimization

Highly parallelizable and compute-intensive - should be modularized in implementation.

## References

- CADO-NFS implementation
- Kleinjung's base-m construction
- Murphy's polynomial selection paper
- Bai's optimization techniques
