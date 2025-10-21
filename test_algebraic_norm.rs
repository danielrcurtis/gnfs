// Test algebraic norm computation
use num::{BigInt, BigRational};

fn main() {
    // Polynomial: f(x) = x³ + 15x² + 29x + 8
    // For N = 45113, m = 31, f(31) = 45113

    // Test relation (a=1, b=3):
    let a = BigInt::from(1);
    let b = BigInt::from(3);

    // Calculate -a/b = -1/3
    let neg_a = -&a;
    let ab_ratio = BigRational::new(neg_a, b.clone());
    println!("-a/b = {}", ab_ratio);

    // Evaluate f(-a/b) = f(-1/3)
    // f(x) = x³ + 15x² + 29x + 8
    let x = &ab_ratio;
    let x2 = x * x;
    let x3 = &x2 * x;

    let term0 = BigRational::from(BigInt::from(8));
    let term1 = BigRational::from(BigInt::from(29)) * x;
    let term2 = BigRational::from(BigInt::from(15)) * &x2;
    let term3 = BigRational::from(BigInt::from(1)) * &x3;

    let f_val = term0 + term1 + term2 + term3;
    println!("f(-1/3) = {}", f_val);
    println!("f(-1/3) as decimal ≈ {}", f_val.numer().to_f64().unwrap() / f_val.denom().to_f64().unwrap());

    // Calculate (-b)^degree = (-3)^3 = -27
    let neg_b = -&b;
    let degree = 3;
    let right = neg_b.pow(degree);
    println!("(-b)^degree = (-3)^3 = {}", right);

    // Algebraic norm = f(-a/b) × (-b)^degree
    let product = f_val * BigRational::from(right);
    println!("Algebraic norm = f(-1/3) × (-27) = {}", product);
    println!("Is integer? {}", product.is_integer());

    let alg_norm = product.numer() / product.denom();
    println!("Final algebraic norm: {}", alg_norm);

    // Rational norm = a + b*m = 1 + 3*31 = 94
    let m = BigInt::from(31);
    let rat_norm = &a + &b * &m;
    println!("\nRational norm = {} + {} × {} = {}", a, b, m, rat_norm);

    println!("\nFactoring:");
    println!("94 = 2 × 47");
    println!("{} needs to factor over small primes", alg_norm);
}
