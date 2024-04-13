// src/integer_math/fast_prime_sieve.rs

use std::mem::size_of;
use num::Zero;
use num::{BigUint, ToPrimitive};
use crate::core::cpu_info;
use std::cell::RefCell;

pub struct FastPrimeSieve {
    page_size: usize,
    buffer_bits: usize,
    buffer_bits_next: usize,
}

impl FastPrimeSieve {
    pub fn new() -> Self {
        let mut cache_size = 393216;
        let cache_sizes = cpu_info::CPUInfo::l1_cache_size();
        if cache_sizes.unwrap() != 0 {
            cache_size = cache_sizes.unwrap() * 1024;
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

    pub fn get_range<'a>(floor: &'a BigUint, ceiling: &'a BigUint) -> impl Iterator<Item = BigUint> + 'a {
        let mut primes_paged = FastPrimeSieve::new();
        let mut enumerator = primes_paged.iterator();
    
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

    fn iterator(&self) -> FastPrimeSieveIterator {
        FastPrimeSieveIterator {
            base_primes_array: RefCell::new(vec![]),
            page_size: self.page_size,
            buffer_bits: self.buffer_bits,
            buffer_bits_next: self.buffer_bits_next,
            low: 0,
            bottom_item: 0,
            cull_buffer: vec![0u32; self.page_size / size_of::<u32>()],
            base_primes: None,
        }
    }
}

impl IntoIterator for FastPrimeSieve {
    type Item = BigUint;
    type IntoIter = FastPrimeSieveIterator;

    fn into_iter(self) -> Self::IntoIter {
        self.iterator()
    }
}

struct FastPrimeSieveIterator {
    base_primes_array: RefCell<Vec<u32>>,
    page_size: usize,
    buffer_bits: usize,
    buffer_bits_next: usize,
    low: u32,
    bottom_item: usize,
    cull_buffer: Vec<u32>,
    base_primes: Option<std::iter::Flatten<std::iter::Once<FastPrimeSieve>>>,
}

impl Iterator for FastPrimeSieveIterator {
    type Item = BigUint;

    fn next(&mut self) -> Option<Self::Item> {
        while self.bottom_item < self.buffer_bits {
            if self.bottom_item < 1 {
                if self.bottom_item < 0 {
                    self.bottom_item = 0;
                    return Some(BigUint::from(2u32));
                }

                let next = BigUint::from(3u32) + BigUint::from(self.low) + BigUint::from(self.low) + BigUint::from(self.buffer_bits_next);
                if BigUint::from(self.low) <= BigUint::zero() {
                    // cull very first page
                    let mut i = 0;
                    let mut sqr = 9;
                    let mut p = 3;
                    while sqr < next.to_usize().expect("next is too large") {
                        if (self.cull_buffer[i >> 5] & (1 << (i & 31))) == 0 {
                            let mut j = (sqr - 3) >> 1;
                            while j < self.buffer_bits {
                                self.cull_buffer[j >> 5] |= 1 << j;
                                j += p;
                            }
                        }
                        i += 1;
                        p += 2;
                        sqr = p * p;
                    }
                } else {
                    // Cull for the rest of the pages
                    self.cull_buffer.fill(0);

                    if self.base_primes_array.borrow().is_empty() {
                        // Init second base primes stream
                        self.base_primes = Some(std::iter::once(FastPrimeSieve::new()).flatten());
                        self.base_primes.as_mut().unwrap().next();
                        self.base_primes.as_mut().unwrap().next();
                        let prime = self.base_primes.as_mut().unwrap().next().unwrap().to_u32().expect("base prime is too large");
                        self.base_primes_array.borrow_mut().push(prime); // Add 3 to base primes array
                        self.base_primes.as_mut().unwrap().next();
                    }

                    // Make sure base_primes_array contains enough base primes...
                    let mut p = BigUint::from(self.base_primes_array.borrow()[self.base_primes_array.borrow().len() - 1]);
                    let mut square = &p * &p;
                    while square < next {
                        p = self.base_primes.as_mut().unwrap().next().unwrap();
                        square = &p * &p;
                        self.base_primes_array.borrow_mut().push(p.to_u32().expect("base prime is too large"));
                    }

                    let limit = self.base_primes_array.borrow().len() - 1;
                    for i in 0..limit {
                        let p = BigUint::from(self.base_primes_array.borrow()[i]);
                        let start = (&p * &p - BigUint::from(3u32)).to_usize().expect("start is too large") >> 1;

                        // adjust start index based on page lower limit...
                        let mut start = if start >= self.low as usize {
                            start - self.low as usize
                        } else {
                            let r = (self.low as usize - start) % p.to_usize().expect("p is too large");
                            if r != 0 {
                                p.to_usize().expect("p is too large") - r
                            } else {
                                0
                            }
                        };

                        while start < self.buffer_bits {
                            self.cull_buffer[start >> 5] |= 1 << start;
                            start += p.to_usize().expect("p is too large");
                        }
                    }
                }
            }

            while self.bottom_item < self.buffer_bits && (self.cull_buffer[self.bottom_item >> 5] & (1 << (self.bottom_item & 31))) != 0 {
                self.bottom_item += 1;
            }

            if self.bottom_item < self.buffer_bits {
                let result = BigUint::from(3u32) + (BigUint::from(self.bottom_item) + BigUint::from(self.low)) * BigUint::from(2u32);
                self.bottom_item += 1;
                return Some(result);
            } else {
                self.low += 1;
                self.bottom_item = 0;
            }
        }

        None
    }
}
