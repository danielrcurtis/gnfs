// tests/sieve_range_tests.rs

use gnfs::core::sieve_range::SieveRange;
use num::BigInt;

#[cfg(test)]
mod sieve_range_tests {
    use super::*;

    #[test]
    fn test_get_sieve_range_from_one() {
        // Test: Generate range from 1 to 5
        // Expected: 1, -1, 2, -2, 3, -3, 4, -4, 5, -5
        let max = BigInt::from(5);
        let result: Vec<BigInt> = SieveRange::get_sieve_range(&max).collect();

        let expected: Vec<BigInt> = vec![1, -1, 2, -2, 3, -3, 4, -4, 5, -5]
            .into_iter()
            .map(BigInt::from)
            .collect();

        assert_eq!(result, expected, "get_sieve_range should produce alternating +/- values from 1 to max");
        assert_eq!(result.len(), 10, "Should produce 2*max values");
    }

    #[test]
    fn test_get_sieve_range_continuation_from_positive() {
        // Test: Continue from 3 to 5
        // Expected: 3, -3, 4, -4, 5, -5
        let start = BigInt::from(3);
        let max = BigInt::from(5);
        let result: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start, &max).collect();

        let expected: Vec<BigInt> = vec![3, -3, 4, -4, 5, -5]
            .into_iter()
            .map(BigInt::from)
            .collect();

        assert_eq!(result, expected, "Should continue from start value to max");
        assert_eq!(result.len(), 6, "Should produce 2*(max-start+1) values");
    }

    #[test]
    fn test_get_sieve_range_continuation_from_negative() {
        // Test: Continue from -3 to 5
        // Start with negative value means we're on the "negative" phase
        let start = BigInt::from(-3);
        let max = BigInt::from(5);
        let result: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start, &max).collect();

        // Since start is -3 (negative), counter starts at abs(-3)=3, flip_flop=false
        // Should produce: -3, 4, -4, 5, -5
        let expected: Vec<BigInt> = vec![-3, 4, -4, 5, -5]
            .into_iter()
            .map(BigInt::from)
            .collect();

        assert_eq!(result, expected, "Should handle negative start values correctly");
    }

    #[test]
    fn test_get_sieve_range_continuation_start_equals_max() {
        // Test: Start equals max
        // Expected: start, -start
        let start = BigInt::from(5);
        let max = BigInt::from(5);
        let result: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start, &max).collect();

        let expected: Vec<BigInt> = vec![5, -5]
            .into_iter()
            .map(BigInt::from)
            .collect();

        assert_eq!(result, expected, "Should handle start == max");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_get_sieve_range_continuation_start_exceeds_max() {
        // Test: Start > max
        // Expected: Empty iterator
        let start = BigInt::from(10);
        let max = BigInt::from(5);
        let result: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start, &max).collect();

        assert_eq!(result.len(), 0, "Should return empty when start > max");
    }

    #[test]
    fn test_get_sieve_range_continuation_large_range() {
        // Test: Generate a larger range and verify count
        let start = BigInt::from(100);
        let max = BigInt::from(150);
        let result: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start, &max).collect();

        // Should generate 51 numbers (100-150 inclusive) * 2 (positive and negative)
        assert_eq!(result.len(), 102, "Should generate correct count for large range");

        // Verify first few and last few
        assert_eq!(result[0], BigInt::from(100));
        assert_eq!(result[1], BigInt::from(-100));
        assert_eq!(result[2], BigInt::from(101));
        assert_eq!(result[3], BigInt::from(-101));

        assert_eq!(result[result.len() - 2], BigInt::from(150));
        assert_eq!(result[result.len() - 1], BigInt::from(-150));
    }

    #[test]
    fn test_sieve_range_no_duplicates() {
        // Verify no duplicate values are generated
        let max = BigInt::from(10);
        let result: Vec<BigInt> = SieveRange::get_sieve_range(&max).collect();

        let mut sorted = result.clone();
        sorted.sort();
        sorted.dedup();

        assert_eq!(result.len(), sorted.len(), "Should not generate duplicates");
    }

    #[test]
    fn test_sieve_range_covers_full_range() {
        // Verify all values from 1 to max (positive and negative) are generated
        let max = BigInt::from(7);
        let result: Vec<BigInt> = SieveRange::get_sieve_range(&max).collect();

        // Check all positive values exist
        for i in 1..=7 {
            assert!(result.contains(&BigInt::from(i)), "Should contain positive {}", i);
            assert!(result.contains(&BigInt::from(-i)), "Should contain negative -{}", i);
        }
    }
}
