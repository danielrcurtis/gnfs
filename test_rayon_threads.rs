// Simple test to verify Rayon thread pool configuration
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn main() {
    // Configure Rayon thread pool based on environment variable
    let num_threads = std::env::var("GNFS_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_else(|| {
            let total_cores = num_cpus::get();
            let default_threads = (total_cores / 4).max(1);
            println!("GNFS_THREADS not set, using default: {}", default_threads);
            default_threads
        });

    println!("Attempting to configure Rayon with {} threads", num_threads);

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .expect("Failed to configure thread pool");

    println!("Rayon configured with {} threads (total cores: {})", num_threads, num_cpus::get());

    // Query Rayon's actual thread count
    let actual_threads = rayon::current_num_threads();
    println!("Rayon reports actual thread count: {}", actual_threads);

    // Test 1: Simple parallel work
    println!("\n=== Test 1: Simple Parallel Work ===");
    let counter = Arc::new(AtomicUsize::new(0));

    let work_items: Vec<usize> = (0..1000).collect();

    let start = std::time::Instant::now();
    work_items.par_iter().for_each(|_| {
        counter.fetch_add(1, Ordering::Relaxed);
        // Simulate some work
        let mut sum = 0u64;
        for i in 0..10000 {
            sum += i;
        }
        std::hint::black_box(sum);
    });
    let duration = start.elapsed();

    println!("Processed {} items in {:?}", counter.load(Ordering::Relaxed), duration);

    // Test 2: Track which threads are actually used
    println!("\n=== Test 2: Thread Usage Tracking ===");
    let thread_ids = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));

    let work_items: Vec<usize> = (0..10000).collect();

    work_items.par_iter().for_each(|_| {
        let thread_id = std::thread::current().id();
        thread_ids.lock().unwrap().insert(format!("{:?}", thread_id));

        // Simulate some work
        let mut sum = 0u64;
        for i in 0..5000 {
            sum += i;
        }
        std::hint::black_box(sum);
    });

    let unique_threads = thread_ids.lock().unwrap().len();
    println!("Unique threads used: {}", unique_threads);
    println!("Expected threads: {}", num_threads);

    if unique_threads == num_threads {
        println!("✓ SUCCESS: All configured threads were used");
    } else {
        println!("✗ WARNING: Only {} of {} configured threads were used", unique_threads, num_threads);
    }

    // Test 3: CPU-intensive work to max out cores
    println!("\n=== Test 3: CPU-Intensive Work ===");
    println!("Running CPU-intensive parallel work for 5 seconds...");
    println!("Monitor Activity Monitor to see if all {} threads are active", num_threads);

    let start = std::time::Instant::now();
    let duration_target = std::time::Duration::from_secs(5);

    let work_items: Vec<usize> = (0..1000).collect();

    while start.elapsed() < duration_target {
        work_items.par_iter().for_each(|_| {
            // CPU-intensive work
            let mut result = 0u64;
            for i in 0..1_000_000 {
                result = result.wrapping_add(i);
            }
            std::hint::black_box(result);
        });
    }

    println!("CPU-intensive work completed. Check Activity Monitor for CPU usage.");
}
