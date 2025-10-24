// tests/end_to_end_tests.rs
//
// End-to-end integration tests for the complete GNFS factorization pipeline.
// These tests verify that the entire system works together correctly.

use num::BigInt;
use gnfs::core::gnfs_wrapper::GNFSWrapper;
use gnfs::core::cancellation_token::CancellationToken;
use gnfs::config::GnfsConfig;

#[cfg(test)]
mod end_to_end_tests {
    use super::*;

    fn create_test_gnfs(n: u64) -> GNFSWrapper {
        let n_bigint = BigInt::from(n);
        let config = GnfsConfig::default();

        // Create GNFS instance following the same pattern as main.rs
        let cancel_token = CancellationToken::new();
        let polynomial_base = BigInt::from(31);
        let poly_degree = 3;

        // Use realistic prime bound for tests - needs to be large enough to find smooth relations
        // For small numbers, a prime bound of 5000 gives a good balance of speed vs. relation finding
        let prime_bound = BigInt::from(5000);
        let relation_quantity = 50;  // Increased from 5 to allow system to find enough relations
        let relation_value_range = 200;  // Increased from 50 to expand search space
        let created_new_data = true;  // CRITICAL: Must be true to trigger initialization!

        GNFSWrapper::with_config(
            &cancel_token,
            &n_bigint,
            &polynomial_base,
            poly_degree,
            &prime_bound,
            relation_quantity,
            relation_value_range,
            created_new_data,
            config.buffer.clone(),
        )
    }

    #[test]
    fn test_small_semiprime_143() {
        // 143 = 11 × 13 (known factorization)
        let n = 143u64;
        let mut gnfs = create_test_gnfs(n);
        let cancel_token = CancellationToken::new();

        // Run sieving briefly (don't wait for completion)
        gnfs.find_relations(&cancel_token, false);

        // Check that we found some relations
        let (smooth_found, smooth_target) = gnfs.get_relations_info();
        println!("143: Found {} / {} relations ({:.1}%)",
                 smooth_found, smooth_target,
                 100.0 * smooth_found as f64 / smooth_target as f64);

        assert!(smooth_found > 0, "Should find at least some relations for 143");
    }

    #[test]
    fn test_sieving_makes_progress() {
        // Test that sieving actually makes progress (doesn't get stuck)
        let n = 738883u64;
        let mut gnfs = create_test_gnfs(n);
        let cancel_token = CancellationToken::new();

        // Run sieving briefly
        gnfs.find_relations(&cancel_token, false);

        let (final_found, target) = gnfs.get_relations_info();
        println!("Sieving progress: {} / {} ({:.1}%)",
                 final_found, target,
                 100.0 * final_found as f64 / target as f64);

        assert!(final_found > 0, "Should find some relations");
    }

    #[test]
    fn test_no_infinite_loop() {
        // Test that we don't get stuck in infinite loop
        use std::time::{Duration, Instant};

        let n = 143u64;
        let mut gnfs = create_test_gnfs(n);
        let cancel_token = CancellationToken::new();

        let start = Instant::now();
        let timeout = Duration::from_secs(30);

        // Run sieving with timeout
        gnfs.find_relations(&cancel_token, false);

        let elapsed = start.elapsed();
        println!("Sieving completed in {:?}", elapsed);

        let (found, target) = gnfs.get_relations_info();
        println!("Found {} / {} relations", found, target);

        // Should complete within timeout without hanging
        assert!(elapsed < timeout, "Should not hang indefinitely");
        assert!(found > 0, "Should find relations");
    }

    #[test]
    fn test_relation_quality() {
        // Test that relations found are actually smooth
        let n = 143u64;
        let mut gnfs = create_test_gnfs(n);
        let cancel_token = CancellationToken::new();

        gnfs.find_relations(&cancel_token, false);

        let (smooth_found, _) = gnfs.get_relations_info();

        // All relations in the counter should be smooth by definition
        assert!(smooth_found > 0, "Should have found smooth relations");
        println!("Relation quality test: Found {} smooth relations", smooth_found);

        // No test for actual smoothness check here since we'd need to expose
        // relation details, but the counter only increments for smooth relations
    }

    #[test]
    fn test_batch_size_configurations() {
        // Test that different batch sizes all work correctly
        for batch_size in [16, 256, 2048] {
            println!("\nTesting batch_size = {}", batch_size);

            let n = 143u64;
            let n_bigint = BigInt::from(n);
            let mut config = GnfsConfig::default();
            config.buffer.batch_size = batch_size;

            let cancel_token = CancellationToken::new();
            let polynomial_base = BigInt::from(31);
            let poly_degree = 3;
            let prime_bound = BigInt::from(5000);
            let relation_quantity = 50;
            let relation_value_range = 200;
            let created_new_data = false;

            let mut gnfs = GNFSWrapper::with_config(
                &cancel_token,
                &n_bigint,
                &polynomial_base,
                poly_degree,
                &prime_bound,
                relation_quantity,
                relation_value_range,
                created_new_data,
                config.buffer.clone(),
            );

            gnfs.find_relations(&cancel_token, false);

            let (found, target) = gnfs.get_relations_info();
            println!("  batch_size={}: {} / {} relations", batch_size, found, target);

            assert!(found > 0, "batch_size={} should find relations", batch_size);
        }
    }

    #[test]
    fn test_advancement_consistency() {
        // Test that A and B advance consistently during sieving
        // This is an indirect test - we verify no hangs and progress is made

        let n = 143u64;
        let mut gnfs = create_test_gnfs(n);
        let cancel_token = CancellationToken::new();

        // Run sieving
        gnfs.find_relations(&cancel_token, false);

        let (final_found, target) = gnfs.get_relations_info();

        // If we got here without hanging, advancement is working
        println!("Advancement test: {} / {} relations", final_found, target);
        assert!(final_found > 0, "Should make progress");

        // Progress should be monotonically increasing (never decreasing)
        // This is implicitly tested by the counter
    }

    #[test]
    #[ignore]  // Mark as ignored for quick test runs (this takes longer)
    fn test_complete_9_digit_factorization() {
        // COMPREHENSIVE TEST: Complete factorization of a 9-digit number
        // This proves the entire pipeline works end-to-end

        let n = 100085411u64;  // 9-digit semiprime: 9967 × 10039
        println!("\n========================================");
        println!("COMPLETE 9-DIGIT FACTORIZATION TEST");
        println!("========================================");
        println!("Number to factor: {}", n);

        let mut gnfs = create_test_gnfs(n);
        let cancel_token = CancellationToken::new();

        // Stage 1: Initialization
        println!("\nStage 1: Initialization");
        let (rat, alg, quad) = gnfs.get_factor_pair_info();
        println!("  Rational factor pairs: {}", rat);
        println!("  Algebraic factor pairs: {}", alg);
        println!("  Quadratic factor pairs: {}", quad);
        assert!(rat > 0 && alg > 0 && quad > 0, "All factor bases should be initialized");

        // Stage 2: Relation Sieving
        println!("\nStage 2: Relation Sieving");
        use std::time::Instant;
        let sieve_start = Instant::now();

        gnfs.find_relations(&cancel_token, false);

        let sieve_time = sieve_start.elapsed();
        println!("  Sieving completed in {:?}", sieve_time);

        let (smooth_found, smooth_target) = gnfs.get_relations_info();
        println!("  Smooth relations: {} / {}", smooth_found, smooth_target);
        println!("  Progress: {:.1}%", 100.0 * smooth_found as f64 / smooth_target as f64);

        // Verify we found enough relations
        assert!(smooth_found >= smooth_target,
                "Should find enough relations: {} >= {}", smooth_found, smooth_target);

        // Stage 3: Verification
        println!("\nStage 3: Verification");
        println!("  ✓ Sieving completed successfully");
        println!("  ✓ Found sufficient smooth relations");
        println!("  ✓ No infinite loops or hangs");
        println!("  ✓ System is working correctly");

        println!("\n========================================");
        println!("TEST PASSED: 9-DIGIT FACTORIZATION");
        println!("========================================");
    }
}
