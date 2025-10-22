# Fix for Single-Core Parallelization Issue

## Problem
CPU usage stays at 100% (only using 1 core) because the workload per B iteration is too small (~25-50 A values). Rayon sees this as not worth the overhead of parallelization.

## Solution
Batch multiple B iterations together to create larger work chunks.

## File to Modify
`src/relation_sieve/poly_relations_sieve_progress.rs`

## Changes

### Replace lines 106-156 with this code:

```rust
            debug!("About to call get_sieve_range_continuation with self.a = {}, self.value_range = {}", self.a, self.value_range);

            // Batch multiple B values together for better parallelization
            // This gives Rayon enough work to effectively use multiple cores
            let batch_size = 10; // Process 10 B values worth of work at once

            // Collect (A, B) pairs for a batch of B values
            let mut ab_pairs = Vec::new();
            let batch_start_b = self.b.clone();

            for b_offset in 0..batch_size {
                let current_b = &batch_start_b + b_offset;
                if &current_b > &self.max_b {
                    break;
                }

                let a_values: Vec<BigInt> = SieveRange::get_sieve_range_continuation(&start_a, &self.value_range)
                    .collect();

                for a in a_values {
                    ab_pairs.push((a, current_b.clone()));
                }
            }

            let total_pairs = ab_pairs.len();
            debug!("Processing {} (A,B) pairs in parallel across up to {} B values", total_pairs, batch_size);

            // Use Mutex to collect smooth relations from parallel threads
            let smooth_relations_found = Mutex::new(Vec::new());

            // Parallel iteration over all (A, B) pairs
            ab_pairs.par_iter()
                .filter(|_| !cancel_token.is_cancellation_requested())
                .filter(|(a, b)| GCD::are_coprime(&[a.clone(), b.clone()]))
                .for_each(|(a, b)| {
                    // Each thread creates and tests its own relation
                    let mut rel = Relation::new(gnfs, a, b);
                    rel.sieve(gnfs);

                    if rel.is_smooth() {
                        smooth_relations_found.lock().unwrap().push(rel);
                    }
                });

            // Collect results from parallel processing
            let mut found = smooth_relations_found.into_inner().unwrap();
            let num_found = found.len();

            // Update progress tracking
            self.relations.smooth_relations.append(&mut found);
            self.smooth_relations_counter += num_found;

            // Update B to the next batch
            self.b = &self.b + batch_size;
            self.a = start_a.clone();

            debug!("Completed parallel processing of {} pairs, found {} smooth relations", total_pairs, num_found);
            debug!("{}", &format!("Now at B = {}", self.b));
            debug!("{}", &format!("SmoothRelations.Count: {}", self.relations.smooth_relations.len()));
```

## What This Does

1. **Batches 10 B values together** instead of processing one at a time
2. Creates ~250-500 (A,B) pairs per batch (10 B × 25-50 A values each)
3. This gives Rayon enough work to justify spawning multiple threads
4. Increments B by 10 each iteration instead of 1

## Expected Result

- CPU usage should jump to 200% (2 cores), 400% (4 cores), etc.
- Significantly faster relation sieving
- Same results, just computed in parallel

## Testing

After applying the fix:

```bash
# Build
cargo build --release

# Test with 2 threads (should see ~200% CPU)
GNFS_THREADS=2 cargo run --release

# Test with 4 threads (should see ~400% CPU)
GNFS_THREADS=4 cargo run --release
```

Watch Activity Monitor - you should now see CPU usage go above 100%!
