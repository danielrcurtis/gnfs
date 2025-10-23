// Test cases for GNFS relation finding with different parameters
use gnfs::core::gnfs::GNFS;
use gnfs::core::cancellation_token::CancellationToken;
use num::BigInt;
use env_logger::Env;

#[test]
fn test_option1_larger_prime_bounds() {
    // Option 1: Use larger prime bounds with N=45113
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let n = BigInt::from(45113);
    let polynomial_base = BigInt::from(31);
    let poly_degree = 3;
    let prime_bound = BigInt::from(500); // Much larger than original 100
    let relation_quantity = 20; // More reasonable target
    let relation_value_range = 200; // Wider search range
    let created_new_data = true;

    println!("\n========================================");
    println!("TEST: Option 1 - Larger Prime Bounds");
    println!("========================================");
    println!("N = {}", n);
    println!("Polynomial Base = {}", polynomial_base);
    println!("Polynomial Degree = {}", poly_degree);
    println!("Prime Bound = {}", prime_bound);
    println!("Relation Target = {}", relation_quantity);
    println!("Relation Value Range = {}", relation_value_range);
    println!("========================================\n");

    let cancel_token = CancellationToken::new();

    let mut gnfs = GNFS::new(
        &cancel_token,
        &n,
        &polynomial_base,
        poly_degree,
        &prime_bound,
        relation_quantity,
        relation_value_range,
        created_new_data,
    );

    println!("Factor bases created:");
    println!("  Rational: {} primes", gnfs.prime_factor_base.rational_factor_base.len());
    println!("  Algebraic: {} primes", gnfs.prime_factor_base.algebraic_factor_base.len());
    println!("  Quadratic: {} primes", gnfs.prime_factor_base.quadratic_factor_base.len());
    println!("  Target relations: {}", gnfs.current_relations_progress.smooth_relations_target_quantity);

    // Sieve for a limited time
    println!("\nSieving for relations (max 100 B iterations)...");
    let initial_b = gnfs.current_relations_progress.b.clone();
    let max_iterations = 100;

    for i in 0..max_iterations {
        if gnfs.current_relations_progress.smooth_relations_counter >= 5 {
            break; // Found enough for test
        }

        // Need to temporarily move progress out to avoid borrowing issues
        let mut progress = std::mem::replace(
            &mut gnfs.current_relations_progress,
            gnfs::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress::default()
        );
        progress.generate_relations(&gnfs, &cancel_token);
        gnfs.current_relations_progress = progress;

        if i % 10 == 0 {
            println!("  B = {}, Smooth = {}/{}",
                gnfs.current_relations_progress.b,
                gnfs.current_relations_progress.smooth_relations_counter,
                gnfs.current_relations_progress.smooth_relations_target_quantity
            );
        }

        if gnfs.current_relations_progress.b == initial_b {
            break; // No progress
        }
    }

    println!("\nResults:");
    println!("  Smooth relations found: {}", gnfs.current_relations_progress.smooth_relations_counter);
    println!("  Final B value: {}", gnfs.current_relations_progress.b);

    // Assert we found at least one smooth relation
    assert!(gnfs.current_relations_progress.smooth_relations_counter > 0,
        "Should find at least one smooth relation with larger prime bounds");
}

#[test]
fn test_option2_simpler_number() {
    // Option 2: Use much simpler number N=143 (11 × 13)
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let n = BigInt::from(143); // 11 × 13
    let polynomial_base = BigInt::from(11); // One of the factors
    let poly_degree = 2; // Simpler polynomial
    let prime_bound = BigInt::from(30); // Small prime bound
    let relation_quantity = 10; // Smaller target
    let relation_value_range = 50;
    let created_new_data = true;

    println!("\n========================================");
    println!("TEST: Option 2 - Simpler Number");
    println!("========================================");
    println!("N = {} = 11 × 13", n);
    println!("Polynomial Base = {}", polynomial_base);
    println!("Polynomial Degree = {}", poly_degree);
    println!("Prime Bound = {}", prime_bound);
    println!("Relation Target = {}", relation_quantity);
    println!("Relation Value Range = {}", relation_value_range);
    println!("========================================\n");

    let cancel_token = CancellationToken::new();

    let mut gnfs = GNFS::new(
        &cancel_token,
        &n,
        &polynomial_base,
        poly_degree,
        &prime_bound,
        relation_quantity,
        relation_value_range,
        created_new_data,
    );

    println!("Factor bases created:");
    println!("  Rational: {} primes", gnfs.prime_factor_base.rational_factor_base.len());
    println!("  Algebraic: {} primes", gnfs.prime_factor_base.algebraic_factor_base.len());
    println!("  Quadratic: {} primes", gnfs.prime_factor_base.quadratic_factor_base.len());
    println!("  Target relations: {}", gnfs.current_relations_progress.smooth_relations_target_quantity);

    // Show first few primes in each base
    println!("\nRational factor base (first 10): {:?}",
        gnfs.prime_factor_base.rational_factor_base.iter().take(10).collect::<Vec<_>>());
    println!("Algebraic factor base (first 10): {:?}",
        gnfs.prime_factor_base.algebraic_factor_base.iter().take(10).collect::<Vec<_>>());

    // Sieve for relations
    println!("\nSieving for relations (max 50 B iterations)...");
    let initial_b = gnfs.current_relations_progress.b.clone();
    let max_iterations = 50;

    for i in 0..max_iterations {
        if gnfs.current_relations_progress.smooth_relations_counter >= gnfs.current_relations_progress.smooth_relations_target_quantity {
            break;
        }

        // Need to temporarily move progress out to avoid borrowing issues
        let mut progress = std::mem::replace(
            &mut gnfs.current_relations_progress,
            gnfs::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress::default()
        );
        progress.generate_relations(&gnfs, &cancel_token);
        gnfs.current_relations_progress = progress;

        if i % 5 == 0 {
            println!("  B = {}, Smooth = {}/{}",
                gnfs.current_relations_progress.b,
                gnfs.current_relations_progress.smooth_relations_counter,
                gnfs.current_relations_progress.smooth_relations_target_quantity
            );
        }

        if gnfs.current_relations_progress.b == initial_b {
            break;
        }
    }

    println!("\nResults:");
    println!("  Smooth relations found: {}", gnfs.current_relations_progress.smooth_relations_counter);
    println!("  Final B value: {}", gnfs.current_relations_progress.b);

    // Print any smooth relations found
    if gnfs.current_relations_progress.smooth_relations_counter > 0 {
        println!("\nFirst smooth relations:");
        for (i, rel) in gnfs.current_relations_progress.relations.smooth_relations.iter().take(5).enumerate() {
            println!("  {}. (a={}, b={}) → alg_norm={}, rat_norm={}",
                i+1, rel.a, rel.b, rel.algebraic_norm, rel.rational_norm);
        }
    }

    assert!(gnfs.current_relations_progress.smooth_relations_counter > 0,
        "Should find smooth relations with simpler number N=143");
}

#[test]
fn test_verify_first_relations() {
    // Test to verify the first few relations are computed correctly
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let n = BigInt::from(45113);
    let polynomial_base = BigInt::from(31);
    let poly_degree = 3;
    let prime_bound = BigInt::from(100);
    let relation_quantity = 5;
    let relation_value_range = 50;
    let created_new_data = true;

    println!("\n========================================");
    println!("TEST: Verify First Relations");
    println!("========================================");

    let cancel_token = CancellationToken::new();

    let gnfs = GNFS::new(
        &cancel_token,
        &n,
        &polynomial_base,
        poly_degree,
        &prime_bound,
        relation_quantity,
        relation_value_range,
        created_new_data,
    );

    println!("Polynomial: {}", gnfs.current_polynomial);
    println!("Polynomial degree: {}", gnfs.current_polynomial.degree());

    // Manually create and check a few relations
    use gnfs::relation_sieve::relation::Relation;

    println!("\nManually checking relations:");
    for b in 3..10 {
        for a in 1..10 {
            let mut rel = Relation::new(&gnfs, &BigInt::from(a), &BigInt::from(b));
            rel.sieve(&gnfs);

            let is_smooth = rel.is_smooth();
            println!("  (a={}, b={}) → alg_norm={}, rat_norm={}, alg_quot={}, rat_quot={}, smooth={}",
                a, b, rel.algebraic_norm, rel.rational_norm,
                rel.algebraic_quotient, rel.rational_quotient, is_smooth);

            if is_smooth {
                println!("    ✓ FOUND SMOOTH RELATION!");
                println!("    Algebraic factorization: {}", rel.algebraic_factorization.format_string_as_factorization());
                println!("    Rational factorization: {}", rel.rational_factorization.format_string_as_factorization());
            }
        }
    }
}

#[test]
fn test_main_program_flow() {
    // Test that exactly mimics the main program flow
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let n = BigInt::from(45113);
    let polynomial_base = BigInt::from(31);
    let poly_degree = 3;
    let prime_bound = BigInt::from(100);
    let relation_quantity = 5;
    let relation_value_range = 50;
    let created_new_data = true;

    println!("\n========================================");
    println!("TEST: Main Program Flow");
    println!("========================================");

    let cancel_token = CancellationToken::new();

    let mut gnfs = GNFS::new(
        &cancel_token,
        &n,
        &polynomial_base,
        poly_degree,
        &prime_bound,
        relation_quantity,
        relation_value_range,
        created_new_data,
    );

    println!("Initial state:");
    println!("  A = {}", gnfs.current_relations_progress.a);
    println!("  B = {}", gnfs.current_relations_progress.b);
    println!("  Smooth counter = {}", gnfs.current_relations_progress.smooth_relations_counter);
    println!("  Target = {}", gnfs.current_relations_progress.smooth_relations_target_quantity);

    // Call generate_relations once (one round)
    println!("\nCalling generate_relations...");
    // Need to temporarily move progress out to avoid borrowing issues
    let mut progress = std::mem::replace(
        &mut gnfs.current_relations_progress,
        gnfs::relation_sieve::poly_relations_sieve_progress::PolyRelationsSieveProgress::default()
    );
    progress.generate_relations(&gnfs, &cancel_token);
    gnfs.current_relations_progress = progress;

    println!("\nAfter generate_relations:");
    println!("  A = {}", gnfs.current_relations_progress.a);
    println!("  B = {}", gnfs.current_relations_progress.b);
    println!("  Smooth counter = {}", gnfs.current_relations_progress.smooth_relations_counter);
    println!("  Smooth relations found: {}", gnfs.current_relations_progress.relations.smooth_relations.len());

    // Print the smooth relations
    if gnfs.current_relations_progress.smooth_relations_counter > 0 {
        println!("\nSmooth relations:");
        for rel in &gnfs.current_relations_progress.relations.smooth_relations {
            println!("  (a={}, b={}) → alg_norm={}, rat_norm={}",
                rel.a, rel.b, rel.algebraic_norm, rel.rational_norm);
        }
    }

    assert!(gnfs.current_relations_progress.smooth_relations_counter > 0,
        "Should find smooth relations using generate_relations");
}
