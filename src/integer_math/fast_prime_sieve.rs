// src/integer_math/fast_prime_sieve.rs

use std::cmp::min;
use std::mem::size_of;
use num::BigUint;
use crate::core::cpu_info;

pub struct FastPrimeSieve {
    page_size: usize,
    buffer_bits: usize,
    buffer_bits_next: usize,
}

impl FastPrimeSieve {
    pub fn new() -> Self {
        let mut cache_size = 393216;
        let cache_sizes = cpu_info::get_cache_sizes(cpu_info::CacheLevel::Level1);
        if !cache_sizes.is_empty() {
            cache_size = cache_sizes[0] * 1024;
        }

        let page_size = cache_size;
        let buffer_bits = page_size * 8;
        let buffer_bits_next = buffer_bits * 2;

        FastPrimeSieve {
            page_size,
            buffer_bits,
            buffer_bits_next,
        }
    }

    pub fn get_range(floor: &BigUint, ceiling: &BigUint) -> impl Iterator<Item = BigUint> {
        let mut primes_paged = FastPrimeSieve::new();
        let mut enumerator = primes_paged.into_iter();

        while let Some(current) = enumerator.next() {
            if &current >= floor {
                break;
            }
        }

        std::iter::from_fn(move || {
            if let Some(current) = enumerator.next() {
                if &current > ceiling {
                    None
                } else {
                    Some(current)
                }
            } else {
                None
            }
        })
    }

    fn iterator(&self) -> impl Iterator<Item = BigUint> {
        let mut base_primes = None;
        let mut base_primes_array = vec![];
        let mut cull_buffer = vec![0u32; self.page_size / size_of::<u32>()];

        std::iter::once(BigUint::from(2u32)).chain(
            (0..).map(move |low| {
                let low = BigUint::from(low);
                let low_usize = low.to_usize().expect("low is too large");
                let mut bottom_item = 0;

                while bottom_item < self.buffer_bits {
                    if bottom_item < 1 {
                        if bottom_item < 0 {
                            bottom_item = 0;
                            yield BigUint::from(2u32);
                        }

                        let next = BigUint::from(3u32) + &low + &low + BigUint::from(self.buffer_bits_next);
                        if low <= BigUint::zero() {
                            // cull very first page
                            let mut i = 0;
                            let mut sqr = 9;
                            let mut p = 3;
                            while sqr < next.to_usize().expect("next is too large") {
                                if (cull_buffer[i >> 5] & (1 << (i & 31))) == 0 {
                                    let mut j = (sqr - 3) >> 1;
                                    while j < self.buffer_bits {
                                        cull_buffer[j >> 5] |= 1 << j;
                                        j += p;
                                    }
                                }
                                i += 1;
                                p += 2;
                                sqr = p * p;
                            }
                        } else {
                            // Cull for the rest of the pages
                            cull_buffer.fill(0);

                            if base_primes_array.is_empty() {
                                // Init second base primes stream
                                base_primes = Some(self.iterator());
                                base_primes.as_mut().unwrap().next();
                                base_primes.as_mut().unwrap().next();
                                base_primes_array.push(base_primes.as_ref().unwrap().next().unwrap().to_u32().expect("base prime is too large")); // Add 3 to base primes array
                                base_primes.as_mut().unwrap().next();
                            }

                            // Make sure base_primes_array contains enough base primes...
                            let mut p = BigUint::from(base_primes_array[base_primes_array.len() - 1]);
                            let mut square = &p * &p;
                            while square < next {
                                p = base_primes.as_ref().unwrap().next().unwrap();
                                square = &p * &p;
                                base_primes_array.push(p.to_u32().expect("base prime is too large"));
                            }

                            let limit = base_primes_array.len() - 1;
                            for i in 0..limit {
                                let p = BigUint::from(base_primes_array[i]);
                                let start = (&p * &p - BigUint::from(3u32)).to_usize().expect("start is too large") >> 1;

                                // adjust start index based on page lower limit...
                                let mut start = if start >= low_usize {
                                    start - low_usize
                                } else {
                                    let r = (low_usize - start) % p.to_usize().expect("p is too large");
                                    if r != 0 {
                                        p.to_usize().expect("p is too large") - r
                                    } else {
                                        0
                                    }
                                };

                                while start < self.buffer_bits {
                                    cull_buffer[start >> 5] |= 1 << start;
                                    start += p.to_usize().expect("p is too large");
                                }
                            }
                        }
                    }

                    while bottom_item < self.buffer_bits && (cull_buffer[bottom_item >> 5] & (1 << (bottom_item & 31))) != 0 {
                        bottom_item += 1;
                    }

                    if bottom_item < self.buffer_bits {
                        let result = BigUint::from(3u32) + (BigUint::from(bottom_item) + &low) * BigUint::from(2u32);
                        yield result;
                        bottom_item += 1;
                    } else {
                        break; // outer loop for next page segment...
                    }
                }
            })
        )
    }
}

impl IntoIterator for FastPrimeSieve {
    type Item = BigUint;
    type IntoIter = std::iter::Flatten<std::iter::Once<std::iter::Once<BigUint>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iterator().flatten()
    }
}