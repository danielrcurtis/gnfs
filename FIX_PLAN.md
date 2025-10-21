# GNFS Fix Plan - Based on C# Reference Implementation

## Critical Bugs Found

### 1. **ALGEBRAIC NORM COMPUTATION IS COMPLETELY WRONG** ⚠️

**Current Rust Code** (`src/relation_sieve/relation.rs:58`):
```rust
self.algebraic_norm = f_a.clone();  // Just f(a) - INCORRECT!
```

**Correct C# Implementation** (`GNFSCore/IntegerMath/Normal.cs:41-59`):
```csharp
public static BigInteger Algebraic(BigInteger a, BigInteger b, Polynomial poly)
{
    BigRational ab = BigRational.Negate(a) / b;  // -a/b
    BigRational left = PolynomialEvaluate_BigRational(poly, ab);  // f(-a/b)
    BigInteger right = BigInteger.Pow(BigInteger.Negate(b), poly.Degree);  // (-b)^degree
    BigRational product = right * left;
    return product.WholePart;
}
```

**Formula**: `Algebraic Norm = f(-a/b) × (-b)^degree`

**Why this matters**: The algebraic norm is used to determine if a relation is smooth. With incorrect norms, we can't find any smooth relations.

---

### 2. Rational Norm is Correct ✅

**Current Rust Code**:
```rust
self.rational_norm = self.apply(&gnfs.polynomial_base);  // a + b*m - CORRECT
```

**C# Implementation**: Same formula `a + bm`

---

### 3. Missing Absolute Value Handling

**C# Implementation** (`Relation.cs:78-89`):
```csharp
AlgebraicQuotient = BigInteger.Abs(AlgebraicNorm);
RationalQuotient = BigInteger.Abs(RationalNorm);

if (AlgebraicNorm.Sign == -1)
{
    AlgebraicFactorization.Add(BigInteger.MinusOne);
}

if (RationalNorm.Sign == -1)
{
    RationalFactorization.Add(BigInteger.MinusOne);
}
```

**Rust Code**: Missing this logic - quotients should be absolute values, and -1 should be added to factorization for negative norms.

---

### 4. Sieving Method Can Be Optimized

**C# Implementation** (`Relation.cs:127-135`):
```csharp
public void Sieve(PolyRelationsSieveProgress relationsSieve)
{
    Sieve(relationsSieve._gnfs.PrimeFactorBase.RationalFactorBase, ref RationalQuotient, RationalFactorization);

    if (IsRationalQuotientSmooth) // Only sieve algebraic if rational is smooth first
    {
        Sieve(relationsSieve._gnfs.PrimeFactorBase.AlgebraicFactorBase, ref AlgebraicQuotient, AlgebraicFactorization);
    }
}
```

**Optimization**: Check rational smoothness first before checking algebraic. Also has early termination when `factor² > quotient`.

---

## Fix Implementation Plan

### Step 1: Add Rational Number Support
Since algebraic norm requires rational arithmetic (for `-a/b`), we need:
- Use the `num-rational` crate (already in dependencies as part of `num`)
- Implement polynomial evaluation with rational numbers

### Step 2: Fix Algebraic Norm Computation
File: `src/relation_sieve/relation.rs`

```rust
use num::rational::Ratio;
use num::BigInt;

pub fn compute_algebraic_norm(a: &BigInt, b: &BigInt, poly: &Polynomial) -> BigInt {
    // Calculate -a/b as a rational number
    let neg_a = -a;
    let ab_ratio = Ratio::new(neg_a, b.clone());

    // Evaluate f(-a/b) using rational arithmetic
    let poly_value = poly.evaluate_rational(&ab_ratio);

    // Calculate (-b)^degree
    let neg_b = -b;
    let right = neg_b.pow(poly.degree() as u32);

    // Multiply: f(-a/b) × (-b)^degree
    let product = poly_value * Ratio::from_integer(right);

    // Extract integer part (should have no fractional part)
    assert_eq!(*product.denom(), BigInt::from(1), "Algebraic norm should be an integer");

    product.numer().clone()
}
```

### Step 3: Add Polynomial Evaluation with Rationals
File: `src/Polynomial/polynomial.rs`

```rust
use num::rational::Ratio;

impl Polynomial {
    pub fn evaluate_rational(&self, x: &Ratio<BigInt>) -> Ratio<BigInt> {
        let mut result = Ratio::from_integer(self.terms.last().unwrap().coefficient.clone());

        for term in self.terms.iter().rev().skip(1) {
            result = result * x + Ratio::from_integer(term.coefficient.clone());
        }

        result
    }
}
```

### Step 4: Handle Negative Norms Correctly
File: `src/relation_sieve/relation.rs`

```rust
// After computing norms:
self.algebraic_quotient = self.algebraic_norm.abs();
self.rational_quotient = self.rational_norm.abs();

// Handle negative norms by adding -1 to factorization
if self.algebraic_norm < BigInt::zero() {
    self.algebraic_factorization.add(&BigInt::from(-1));
}

if self.rational_norm < BigInt::zero() {
    self.rational_factorization.add(&BigInt::from(-1));
}
```

### Step 5: Optimize Sieving
File: `src/relation_sieve/relation.rs`

```rust
pub fn sieve(&mut self, gnfs: &GNFS, progress: &mut PolyRelationsSieveProgress) {
    // Sieve rational first
    self.sieve_with_base(
        &gnfs.prime_factor_base.rational_factor_base,
        &mut self.rational_quotient,
        &mut self.rational_factorization
    );

    // Only sieve algebraic if rational is smooth (optimization)
    if self.is_rational_quotient_smooth() {
        self.sieve_with_base(
            &gnfs.prime_factor_base.algebraic_factor_base,
            &mut self.algebraic_quotient,
            &mut self.algebraic_factorization
        );
    }
}

fn sieve_with_base(&self, factor_base: &[BigInt], quotient: &mut BigInt, factorization: &mut CountDictionary) {
    for factor in factor_base {
        if *quotient == BigInt::one() {
            return;
        }

        // Early termination: if factor² > quotient, we can stop
        if factor * factor > *quotient {
            if factor_base.contains(quotient) {
                factorization.add(quotient);
                *quotient = BigInt::one();
            }
            return;
        }

        while quotient % factor == BigInt::zero() {
            factorization.add(factor);
            *quotient /= factor;
        }
    }
}
```

---

## Testing Plan

### Test 1: Verify Algebraic Norm Formula
For N=45113, m=31, polynomial: f(x) = x³ + 15x² + 29x + 8

Test relation (a=1, b=3):
- **Rational norm**: 1 + 3×31 = 94 = 2 × 47
- **Algebraic norm**: f(-1/3) × (-3)³
  - f(-1/3) = (-1/3)³ + 15(-1/3)² + 29(-1/3) + 8
  - f(-1/3) = -1/27 + 15/9 - 29/3 + 8
  - f(-1/3) = -1/27 + 45/27 - 261/27 + 216/27 = -1/27
  - Algebraic norm = (-1/27) × (-27) = 1

Wait, that can't be right. Let me recalculate...

Actually, checking the C# code more carefully:
- It evaluates f(-a/b) where a=1, b=3, so f(-1/3)
- Then multiplies by (-b)^degree = (-3)^3 = -27

### Test 2: Run with Small Number
Test with N=143 (11×13), smaller prime bounds

---

## Priority

**IMMEDIATE**: Fix algebraic norm computation - this is the blocker preventing ANY smooth relations from being found.

Once this is fixed, the sieving should start finding smooth relations and the rest of the workflow can be tested.
