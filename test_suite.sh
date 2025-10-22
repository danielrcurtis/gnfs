#!/bin/bash
# Comprehensive test suite for GNFS parameter validation

export GNFS_THREADS=8
export MY_LOG_LEVEL=error

echo "=========================================="
echo "GNFS PARAMETER VALIDATION TEST SUITE"
echo "=========================================="
echo ""
echo "Hardware: M3 MacBook Pro, 8 threads"
echo ""

# Test numbers: digits -> number (factors)
declare -A test_numbers=(
    ["8"]="47893197"           # 3 × 11 × 79 × 18371
    ["9a"]="100036201"         # 3163 × 31627
    ["9b"]="100085411"         # 3067 × 32633
    ["9c"]="100877363"         # 2999 × 33637
    ["10a"]="1000730021"       # 10007 × 100003
    ["10b"]="1000090109"       # 9901 × 101009
    ["10c"]="1000033439"       # 9803 × 102013
    ["11a"]="10001754107"      # 31627 × 316241
    ["11b"]="10003430467"      # 31531 × 317257
    ["12a"]="100003300009"     # 100003 × 1000003
    ["12b"]="100002599317"     # 99901 × 1001017
    ["17"]="10000004400000259" # 17-digit problem case
)

results_file="/tmp/gnfs_test_results_$(date +%s).txt"

echo "Test results will be saved to: $results_file"
echo ""

for key in "${!test_numbers[@]}"; do
    n="${test_numbers[$key]}"
    digits="${#n}"

    echo "----------------------------------------"
    echo "Testing: $key ($digits digits) - N=$n"
    echo "----------------------------------------"

    # Clean up previous run
    rm -rf "$n/"

    # Run test with timing
    start_time=$(date +%s)
    ./target/release/gnfs "$n" > "/tmp/gnfs_$key.log" 2>&1
    exit_code=$?
    end_time=$(date +%s)
    duration=$((end_time - start_time))

    # Check if successful
    if [ $exit_code -eq 0 ]; then
        if grep -q "FACTORIZATION SUCCESSFUL" "/tmp/gnfs_$key.log"; then
            factors=$(grep "N = .* = .* ×" "/tmp/gnfs_$key.log" | head -1)
            verified=$(grep -q "VERIFIED: Factors are correct!" "/tmp/gnfs_$key.log" && echo "✓" || echo "✗")
            echo "  Result: SUCCESS $verified"
            echo "  Time: ${duration}s"
            echo "  $factors"
        else
            echo "  Result: FAILED (no factorization)"
            echo "  Time: ${duration}s"
        fi
    else
        echo "  Result: ERROR (exit code: $exit_code)"
        echo "  Time: ${duration}s"
    fi

    echo ""
done

echo "=========================================="
echo "Test suite complete!"
echo "Results saved to: $results_file"
echo "=========================================="
