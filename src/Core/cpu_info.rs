// src/core/cpu_info.rs

use cache_size::{l1_cache_size, l1_cache_line_size, l2_cache_size, l2_cache_line_size, l3_cache_size, l3_cache_line_size};

pub struct CPUInfo;

impl CPUInfo {
    // Fetches the total size in bytes of the L1 data cache.
    pub fn l1_cache_size() -> Option<usize> {
        l1_cache_size()
    }

    // Fetches the line size in bytes of the L1 data cache.
    pub fn l1_cache_line_size() -> Option<usize> {
        l1_cache_line_size()
    }

    // Fetches the total size in bytes of the unified L2 cache.
    pub fn l2_cache_size() -> Option<usize> {
        l2_cache_size()
    }

    // Fetches the line size in bytes of the unified L2 cache.
    pub fn l2_cache_line_size() -> Option<usize> {
        l2_cache_line_size()
    }

    // Fetches the total size in bytes of the unified L3 cache.
    pub fn l3_cache_size() -> Option<usize> {
        l3_cache_size()
    }

    // Fetches the line size in bytes of the unified L3 cache.
    pub fn l3_cache_line_size() -> Option<usize> {
        l3_cache_line_size()
    }
}


pub enum CacheLevel {
    Level1 = 3,
    Level2 = 4,
    Level3 = 5,
}