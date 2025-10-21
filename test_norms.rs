// Quick test to check norm computation
use num::BigInt;

fn main() {
    // Polynomial: f(x) = x^3 + 15x^2 + 29x + 8
    // Polynomial base m = 31
    // f(31) = 45113

    let m = BigInt::from(31);
    let a = BigInt::from(1);
    let b = BigInt::from(3);

    // Rational norm should be: a + b*m = 1 + 3*31 = 94
    let rational_norm = &a + &b * &m;
    println!("Rational norm (a={}, b={}): {}", a, b, rational_norm);

    // Algebraic norm should be: f(a) = f(1) = 1 + 15 + 29 + 8 = 53
    let f_a = BigInt::from(1) + BigInt::from(15) + BigInt::from(29) + BigInt::from(8);
    println!("Algebraic norm (a={}): f(a) = {}", a, f_a);

    // For N = 45113, we need both 94 and 53 to factor over small primes
    // 94 = 2 * 47 (47 is prime, so needs to be in factor base if prime bound is >= 47)
    // 53 is prime, so needs to be in factor base

    println!("\nFactoring 94:");
    let mut n = 94;
    print!("94 = ");
    let mut first = true;
    for p in vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97] {
        while n % p == 0 {
            if !first {
                print!(" * ");
            }
            print!("{}", p);
            first = false;
            n /= p;
        }
    }
    if n > 1 {
        if !first {
            print!(" * ");
        }
        print!("{} (unfactored)", n);
    }
    println!();

    println!("\n53 is prime");
    println!("\nPrime bound in main.rs is 100, so factor bases include primes up to 100.");
    println!("Rational factor base should include 2, 47");
    println!("Algebraic factor base should include 53");
    println!("\nSo (a=1, b=3) should be smooth IF 47 and 53 are in their respective factor bases.");
}
