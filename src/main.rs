// src/main.rs

use log::{info, warn, debug, trace, error};
use env_logger::Env;
use cache_size;



fn main() {
    // Initialize the logger
    let env = Env::default()
    .filter_or("MY_LOG_LEVEL", "debug")
    .write_style_or("MY_LOG_STYLE", "always");

    env_logger::Builder::from_env(env).init(); 

    info!("Hello, world!");
    let cpu_info = CpuInfo::new();
    let l1_cache_line_size = cache_size::l1_cache_line_size();
    let l1_cache_size = cache_size::l1_cache_size();
    info!("L1 cache size: {}", l1_cache_size);
    info!("L1 cache line size: {}", l1_cache_line_size);
}