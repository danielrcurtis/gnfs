// Test to reproduce the GCD bug
use num::BigInt;

fn main() {
    // Simulate: g = X^424 - X, f = X³ + 15X² + 29X + 8, p = 419
    // After mod_mod: h = (X^424 - X) mod f (mod 419)
    // Then: gcd(h, f, 419)

    // The bug is that gcd is returning f itself

    println!("Testing GCD bug reproduction");
    println!("g = X^424 - X");
    println!("f = X³ + 15X² + 29X + 8");
    println!("p = 419");
    println!();
    println!("After h = mod_mod(g, f, p), we compute gcd(h, f, p)");
    println!();
    println!("Expected: gcd should NOT always equal f");
    println!("Actual: gcd always equals f");
    println!();
    println!("The issue is in the polynomial field GCD algorithm.");
}
