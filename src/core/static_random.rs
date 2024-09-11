// src/core/static_random.rs

use num::BigInt;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub struct StaticRandom {
    rng: ChaCha8Rng,
}

impl StaticRandom {
    pub fn new() -> Self {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill(&mut seed);
        let mut rng = ChaCha8Rng::from_seed(seed);
        let counter = rng.gen_range(100..200);
        for _ in 0..counter {
            rng.gen::<u32>();
        }
        StaticRandom { rng }
    }

    pub fn next(&mut self) -> u32 {
        self.rng.gen()
    }

    pub fn next_max(&mut self, max_value: u32) -> u32 {
        self.rng.gen_range(0..max_value)
    }

    pub fn next_range(&mut self, min_value: u32, max_value: u32) -> u32 {
        self.rng.gen_range(min_value..max_value)
    }

    pub fn next_double(&mut self) -> f64 {
        self.rng.gen()
    }

    pub fn next_bytes(&mut self, bytes: &mut [u8]) {
        self.rng.fill(bytes);
    }

    pub fn next_bigint(&mut self, lower: &BigInt, upper: &BigInt) -> BigInt {
        if lower > upper {
            panic!("Upper must be greater than or equal to lower");
        }

        let delta = upper - lower;
        let delta_bytes = delta.to_bytes_be().1;
        let mut buffer = vec![0u8; delta_bytes.len()];

        loop {
            self.next_bytes(&mut buffer);
            let result = BigInt::from_bytes_be(num::bigint::Sign::Plus, &buffer) + lower;

            if &result >= lower && &result <= upper {
                return result;
            }
        }
    }
}