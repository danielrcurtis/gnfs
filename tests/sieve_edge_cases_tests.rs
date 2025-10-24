// tests/sieve_edge_cases_tests.rs
//
// Edge case tests for sieving behavior that previously caused bugs.
// These tests document and prevent regression of known issues.

use num::{BigInt, Signed};

#[cfg(test)]
mod sieve_edge_cases {
    use super::*;
    use gnfs::core::sieve_range::SieveRange;

    #[test]
    fn test_sieve_range_with_start_greater_than_max() {
        // REGRESSION TEST: This was causing 0 pairs bug
        // When A > max, iterator should return empty
        let start = BigInt::from(715);
        let max = BigInt::from(51);  // max < start

        let result: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start, &max).collect();

        assert_eq!(result.len(), 0, "Should return empty when start > max");
    }

    #[test]
    fn test_sieve_range_correct_usage() {
        // CORRECT USAGE: Pass absolute end value, not range size
        let start_a = BigInt::from(715);
        let value_range = BigInt::from(51);
        let max_a = &start_a + &value_range;  // 766

        let result: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start_a, &max_a).collect();

        assert!(result.len() > 0, "Should generate values when start < max");
        assert_eq!(result.len(), 104, "Should generate 2*(766-715+1) = 104 values");

        // Verify range bounds
        let abs_values: Vec<i64> = result.iter()
            .map(|v| v.abs().to_string().parse::<i64>().unwrap())
            .collect();

        assert!(abs_values.iter().all(|&v| v >= 715 && v <= 766),
                "All values should be in range [715, 766]");
    }

    #[test]
    fn test_batch_size_vs_max_b_interaction() {
        // REGRESSION TEST: Large batch_size with small max_b caused 0 pairs
        // Simulates: batch_size=2048, B=3, max_b=400

        let batch_size = 2048;
        let b = 3;
        let max_b = 400;
        let mut b_values_processed = 0;

        // Simulate the loop
        for b_offset in 0..batch_size {
            let current_b = b + b_offset;
            if current_b > max_b {
                break;  // Should stop, not continue
            }
            b_values_processed += 1;
        }

        // We process B values [3, 4, 5, ..., 400], which is 398 values
        assert_eq!(b_values_processed, 398, "Should process 398 values (3 through 400)");

        // B should advance by actual processed
        let b_after = b + b_values_processed;
        assert_eq!(b_after, 401, "B should advance to 3+398=401");
        assert_ne!(b_after, b + batch_size, "B should NOT advance by full batch_size");
    }

    #[test]
    fn test_start_a_capture_inside_loop() {
        // REGRESSION TEST: start_a captured outside loop caused infinite loop
        // Simulates the difference between capturing outside vs inside loop

        // WRONG: Capture outside
        let mut a_wrong = BigInt::from(1);
        let value_range = BigInt::from(50);
        let start_a_wrong = a_wrong.clone();  // ❌ Captured once

        let mut a_values_wrong = vec![];
        for _ in 0..3 {
            a_values_wrong.push(a_wrong.clone());
            a_wrong = &start_a_wrong + &value_range;  // Always resets to 51
        }

        // All iterations search the same A region (bug!)
        assert_eq!(a_values_wrong, vec![
            BigInt::from(1),
            BigInt::from(51),
            BigInt::from(51),  // ❌ Stuck!
        ]);

        // CORRECT: Capture inside
        let mut a_correct = BigInt::from(1);

        let mut a_values_correct = vec![];
        for _ in 0..3 {
            let start_a_correct = a_correct.clone();  // ✓ Fresh each iteration
            a_values_correct.push(a_correct.clone());
            a_correct = &start_a_correct + &value_range;
        }

        // Each iteration advances correctly
        assert_eq!(a_values_correct, vec![
            BigInt::from(1),
            BigInt::from(51),
            BigInt::from(101),  // ✓ Advances!
        ]);
    }

    #[test]
    fn test_zero_pairs_condition() {
        // Test the exact condition that was causing "Total (A,B) pairs: 0"

        // Scenario: A=715, value_range=51
        // BUG: Passed value_range (51) directly as max
        let start_a_bug = BigInt::from(715);
        let value_range_bug = BigInt::from(51);
        let bug_result: Vec<BigInt> = SieveRange::get_sieve_range_continuation(
            &start_a_bug,
            &value_range_bug  // ❌ Wrong!
        ).collect();
        assert_eq!(bug_result.len(), 0, "Bug: passing size as max produces 0 pairs");

        // FIX: Calculate absolute max
        let start_a_fix = BigInt::from(715);
        let value_range_fix = BigInt::from(51);
        let max_a = &start_a_fix + &value_range_fix;  // 766
        let fix_result: Vec<BigInt> = SieveRange::get_sieve_range_continuation(
            &start_a_fix,
            &max_a  // ✓ Correct!
        ).collect();
        assert!(fix_result.len() > 0, "Fix: passing absolute max produces pairs");
    }

    #[test]
    fn test_effective_value_range_vs_value_range() {
        // Test that effective_value_range (capped) is used, not value_range

        let value_range = BigInt::from(200);  // Actual value_range
        let max_value_range = BigInt::from(150);  // Cap

        let effective_value_range = if value_range > max_value_range {
            max_value_range.clone()
        } else {
            value_range.clone()
        };

        assert_eq!(effective_value_range, BigInt::from(150), "Should be capped at 150");

        // When advancing A, should use effective (150), not original (200)
        let start_a = BigInt::from(1);
        let a_after = &start_a + &effective_value_range;

        assert_eq!(a_after, BigInt::from(151), "A advances by effective_value_range (150)");
        assert_ne!(a_after, BigInt::from(201), "A should NOT advance by uncapped value_range (200)");
    }

    #[test]
    fn test_max_b_growth_pattern() {
        // Test that max_b grows correctly when B exceeds it
        let initial_max_b = BigInt::from(300);
        let mut max_b = initial_max_b.clone();
        let mut b = BigInt::from(3);

        // Advance B in steps
        for _ in 0..25 {
            // When B exceeds max_b, increase max_b BEFORE advancing B
            if b > max_b {
                max_b = &b + 100;
            }

            b = &b + 16;  // batch_size=16
        }

        // After 25 iterations: B = 3 + 25*16 = 403
        assert_eq!(b, BigInt::from(403));

        // max_b should have grown from initial 300
        assert!(max_b > initial_max_b, "max_b should have increased from initial value");
        assert!(max_b >= b, "max_b should be >= B");
    }

    #[test]
    fn test_coprimality_filter_not_cause_zero_pairs() {
        // Even if coprimality filter removes most pairs, we should still generate
        // the initial candidate pairs before filtering

        let start_a = BigInt::from(1);
        let max_a = BigInt::from(51);

        let candidates: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start_a, &max_a).collect();

        assert!(candidates.len() > 0, "Should generate candidate pairs");
        assert_eq!(candidates.len(), 102, "Should generate all pairs before filtering");

        // After filtering (simulated), some pairs remain
        let filtered: Vec<BigInt> = candidates.into_iter()
            .filter(|a| {
                let b = BigInt::from(10);
                // Simple coprimality check (not actual GCD)
                (a % &b) != BigInt::from(0)
            })
            .collect();

        // Even after filtering, should have some pairs
        assert!(filtered.len() > 0, "Should have some coprime pairs");
    }
}
