// src/main.rs

use log::{info, warn, debug, trace, error};
use env_logger::Env;
use crate::core::cpu_info::CpuInfo;



fn main() {
    // Initialize the logger
    let env = Env::default()
    .filter_or("MY_LOG_LEVEL", "debug")
    .write_style_or("MY_LOG_STYLE", "always");

    env_logger::Builder::from_env(env).init(); 

    info!("Hello, world!");
    let cpu_info = CpuInfo::new();
    let l1_cache_size = cpu_info.l1_cache_size();
    info!("L1 cache size: {}", l1_cache_size);
}