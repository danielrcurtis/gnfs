// src/main.rs

use log::{info, warn, debug, trace, error};
use env_logger::Env;




fn main() {
    // Initialize the logger
    let env = Env::default()
    .filter_or("MY_LOG_LEVEL", "debug")
    .write_style_or("MY_LOG_STYLE", "always");

    env_logger::Builder::from_env(env).init(); 
}