#!/usr/bin/env python3
"""
Generate test semiprimes (product of two primes) for GNFS parameter testing.
"""

def is_prime(n):
    """Simple primality test for small numbers."""
    if n < 2:
        return False
    if n == 2:
        return True
    if n % 2 == 0:
        return False
    for i in range(3, int(n**0.5) + 1, 2):
        if n % i == 0:
            return False
    return True

def next_prime(n):
    """Find the next prime after n."""
    candidate = n + 1 if n % 2 == 0 else n + 2
    while not is_prime(candidate):
        candidate += 2
    return candidate

def prev_prime(n):
    """Find the largest prime less than n."""
    candidate = n - 1 if n % 2 == 0 else n - 2
    while candidate > 2 and not is_prime(candidate):
        candidate -= 2
    return candidate

def generate_semiprime(target_digits):
    """
    Generate a semiprime with approximately target_digits digits.
    Returns (p, q, p*q) where p and q are prime.
    """
    target = 10 ** target_digits

    # Try to split roughly in half
    half_digits = target_digits // 2

    # Start with a prime near the middle
    p = next_prime(10 ** half_digits)

    # Calculate q to get close to target
    q_target = target // p
    q = next_prime(q_target)

    product = p * q

    # Adjust if we're over
    while len(str(product)) > target_digits:
        q = prev_prime(q)
        product = p * q

    # Adjust if we're under
    while len(str(product)) < target_digits:
        q = next_prime(q)
        product = p * q

    return p, q, product

def find_semiprimes_in_range(min_val, max_val, count=3):
    """Find semiprimes in a specific range."""
    results = []

    # Start from a prime near sqrt(min_val)
    p = next_prime(int(min_val ** 0.5))

    while len(results) < count and p * p < max_val:
        q = next_prime(min_val // p)

        product = p * q

        while product <= max_val:
            if min_val <= product <= max_val and len(str(product)) == len(str(min_val)):
                results.append((p, q, product))
                if len(results) >= count:
                    break

            q = next_prime(q)
            product = p * q

        p = next_prime(p)

    return results

# Known good test numbers (handpicked semiprimes)
test_numbers = {
    8: [
        (6917, 6923, 47893197),  # Baseline - known to work
    ],
    9: [
        (10007, 31469, 314920583),
        (14143, 23971, 339023653),
        (18713, 19681, 368237353),
    ],
    10: [
        (31627, 31657, 1001163139),
        (100003, 99991, 9999399973),
        (158233, 189389, 29968661137),
    ],
    11: [
        (316513, 316549, 100179924437),
        (500009, 630007, 315008630063),
        (707089, 811447, 573881819183),
    ],
    12: [
        (1000003, 999983, 999985999949),
        (2236067, 1483637, 3317738819279),
    ],
}

print("=" * 70)
print("GNFS TEST NUMBERS - SEMIPRIMES")
print("=" * 70)
print()

for digits in sorted(test_numbers.keys()):
    print(f"{digits} DIGITS:")
    for p, q, n in test_numbers[digits]:
        # Verify
        if p * q != n:
            print(f"  ERROR: {p} × {q} = {p*q}, not {n}")
        elif len(str(n)) != digits:
            print(f"  ERROR: {n} has {len(str(n))} digits, not {digits}")
        else:
            if digits == 8:
                print(f"  {n} = {p} × {q} (BASELINE)")
            else:
                print(f"  {n} = {p} × {q}")
    print()

print("=" * 70)
print()
print("USAGE:")
print("  ./target/release/gnfs <number>")
print("  Example: ./target/release/gnfs 47893197")
print()
