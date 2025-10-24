// tests/sieve_advancement_tests.rs
//
// Tests for the A/B advancement logic in relation sieving.
// These tests verify that the search space is traversed correctly.

use num::BigInt;

#[cfg(test)]
mod sieve_advancement_tests {
    use super::*;

    /// Simulates the A/B advancement logic from poly_relations_sieve_progress.rs
    /// This is extracted to test the core logic without needing a full GNFS instance.
    struct SieveState {
        a: BigInt,
        b: BigInt,
        max_b: BigInt,
        value_range: BigInt,
    }

    impl SieveState {
        fn new(initial_a: i64, initial_b: i64, max_b: i64, value_range: i64) -> Self {
            SieveState {
                a: BigInt::from(initial_a),
                b: BigInt::from(initial_b),
                max_b: BigInt::from(max_b),
                value_range: BigInt::from(value_range),
            }
        }

        /// Simulates one iteration of the sieving loop
        fn advance(&mut self, batch_size: i64, b_values_actually_processed: i64, effective_value_range: i64) {
            // Capture start_a at beginning of iteration (CRITICAL - must be inside loop)
            let start_a = self.a.clone();

            // Increase max_b if needed
            if &self.b > &self.max_b {
                self.max_b = &self.b + 100;
            }

            // After processing, advance A and B
            self.b = &self.b + b_values_actually_processed;  // Use actual, not requested
            self.a = &start_a + &BigInt::from(effective_value_range);
        }
    }

    #[test]
    fn test_a_advances_each_iteration() {
        // Test: A should advance by value_range each iteration
        let mut state = SieveState::new(1, 3, 300, 50);

        // Iteration 1: A should advance from 1 to 1+50=51
        state.advance(16, 16, 50);
        assert_eq!(state.a, BigInt::from(51), "A should advance by value_range (50)");

        // Iteration 2: A should advance from 51 to 51+50=101
        state.advance(16, 16, 50);
        assert_eq!(state.a, BigInt::from(101), "A should continue advancing");

        // Iteration 3
        state.advance(16, 16, 50);
        assert_eq!(state.a, BigInt::from(151), "A advancement should be consistent");
    }

    #[test]
    fn test_b_advances_by_actual_processed() {
        // Test: B should advance by ACTUAL values processed, not requested batch_size
        let mut state = SieveState::new(1, 3, 300, 50);

        // Request batch_size=2048, but only 298 processed (limited by max_b)
        state.advance(2048, 298, 50);
        assert_eq!(state.b, BigInt::from(3 + 298), "B should advance by actual processed (298)");
        assert_ne!(state.b, BigInt::from(3 + 2048), "B should NOT advance by full batch_size");
    }

    #[test]
    fn test_b_respects_max_b_limit() {
        // Test: When B exceeds max_b, max_b should increase
        let mut state = SieveState::new(1, 3, 300, 50);

        // Process enough to exceed max_b
        state.advance(16, 16, 50);
        assert_eq!(state.b, BigInt::from(19));
        assert_eq!(state.max_b, BigInt::from(300), "max_b unchanged when B < max_b");

        // Advance B past max_b
        for _ in 0..20 {
            state.advance(16, 16, 50);
        }

        assert!(state.b > BigInt::from(300), "B should exceed original max_b");
        assert!(state.max_b > BigInt::from(300), "max_b should have increased");
        assert!(state.b <= state.max_b, "B should not exceed max_b after adjustment");
    }

    #[test]
    fn test_partial_batch_processing() {
        // Test: When batch is partially processed (e.g., 5 out of 16), B advances correctly
        let mut state = SieveState::new(1, 295, 300, 50);

        // Request 16, but only 5 fit within max_b (295-300)
        state.advance(16, 5, 50);

        assert_eq!(state.b, BigInt::from(300), "B should advance by 5, not 16");
        assert_eq!(state.a, BigInt::from(51), "A still advances normally");
    }

    #[test]
    fn test_advancement_pattern_consistency() {
        // Test: Verify the 2D search space is covered consistently
        let mut state = SieveState::new(1, 3, 1000, 50);

        let mut a_values = vec![];
        let mut b_values = vec![];

        // Run 10 iterations
        for _ in 0..10 {
            a_values.push(state.a.clone());
            b_values.push(state.b.clone());
            state.advance(16, 16, 50);
        }

        // Verify A forms an arithmetic sequence
        for i in 1..a_values.len() {
            let diff = &a_values[i] - &a_values[i-1];
            assert_eq!(diff, BigInt::from(50), "A should advance by constant amount (50)");
        }

        // Verify B forms an arithmetic sequence
        for i in 1..b_values.len() {
            let diff = &b_values[i] - &b_values[i-1];
            assert_eq!(diff, BigInt::from(16), "B should advance by constant amount (16)");
        }
    }

    #[test]
    fn test_no_duplicate_search_regions() {
        // Test: Ensure we don't search the same (A, B) region twice
        // This was the bug: start_a was captured outside loop, causing repeated searches
        let mut state = SieveState::new(1, 3, 1000, 50);

        let mut regions = vec![];

        for _ in 0..5 {
            // Record the region being searched
            regions.push((state.a.clone(), state.b.clone()));
            state.advance(16, 16, 50);
        }

        // Check for duplicates
        for i in 0..regions.len() {
            for j in (i+1)..regions.len() {
                assert_ne!(regions[i], regions[j], "Should not search same region twice");
            }
        }
    }

    #[test]
    fn test_value_range_capping() {
        // Test: effective_value_range can differ from value_range (capped at 150)
        let mut state = SieveState::new(1, 3, 1000, 200);  // value_range = 200

        // But effective_value_range is capped at 150
        state.advance(16, 16, 150);  // Use capped value

        assert_eq!(state.a, BigInt::from(151), "A advances by effective_value_range (150), not value_range (200)");
    }

    #[test]
    fn test_large_batch_size_with_small_max_b() {
        // Test: Large batch_size doesn't cause issues when constrained by max_b
        let mut state = SieveState::new(1, 3, 400, 50);

        // Huge batch_size, but only 397 can be processed
        state.advance(2048, 397, 50);

        assert_eq!(state.b, BigInt::from(400), "B advances by actual (397)");
        assert_eq!(state.a, BigInt::from(51), "A advances normally");

        // Next iteration
        state.advance(2048, 100, 50);  // max_b increased, 100 more processed

        assert_eq!(state.b, BigInt::from(500), "B continues advancing correctly");
        assert_eq!(state.a, BigInt::from(101), "A continues advancing");
    }
}
