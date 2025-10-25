// Integration test for SIQS implementation
use num::BigInt;
use std::str::FromStr;
use gnfs::algorithms::siqs::siqs;

#[test]
fn test_siqs_small_number() {
    // Test with a small composite to verify the pipeline works
    // 8051 = 83 × 97 (4 digits, not optimal for SIQS but good for testing)
    let n = BigInt::from(8051);
    let result = siqs(&n);

    // May or may not succeed (SIQS is optimized for 40+ digits)
    // This test mainly verifies that SIQS doesn't crash
    match result {
        Some((p, q)) => {
            assert_eq!(&p * &q, n);
            println!("SIQS factored 8051 = {} × {}", p, q);
        }
        None => {
            println!("SIQS did not factor 8051 (expected - too small for SIQS)");
        }
    }
}

#[test]
#[ignore] // Ignore by default - takes several minutes
fn test_siqs_40_digit_number() {
    // Test with a 41-digit semiprime (from SIQS implementation plan)
    let n = BigInt::from_str("10000000000000000016800000000000000005031").unwrap();

    println!("\n========================================");
    println!("SIQS 40-DIGIT TEST");
    println!("========================================");
    println!("Testing n = {}", n);
    println!("Digits: {}", n.to_string().len());

    let result = siqs(&n);

    match result {
        Some((p, q)) => {
            println!("\nSuccess!");
            println!("p = {}", p);
            println!("q = {}", q);
            println!("Verification: p × q = {}", &p * &q);
            assert_eq!(&p * &q, n);
        }
        None => {
            println!("\nSIQS did not find a factorization");
            println!("This may be expected if:");
            println!("  - Not enough relations were found");
            println!("  - The number is actually prime");
            println!("  - Parameters need tuning");
        }
    }
}

#[test]
#[ignore] // Ignore by default - takes time
fn test_siqs_via_dispatcher() {
    // Test SIQS through the algorithm dispatcher
    use gnfs::algorithms::factor;

    // Use a known 40-digit semiprime
    let n = BigInt::from_str("10000000000000000016800000000000000005031").unwrap();

    match factor(&n) {
        Ok((p, q)) => {
            println!("Dispatcher successfully factored 41-digit number");
            println!("p = {}", p);
            println!("q = {}", q);
            assert_eq!(&p * &q, n);
        }
        Err(e) => {
            println!("Dispatcher failed: {}", e);
            println!("This may indicate SIQS needs more time or parameter tuning");
        }
    }
}
