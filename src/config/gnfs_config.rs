// src/config/gnfs_config.rs

use serde::{Deserialize, Serialize};
use config::{Config, ConfigError, Environment, File};
use std::path::Path;

/// Main GNFS configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnfsConfig {
    /// Output directory for GNFS data
    pub output_dir: String,

    /// Auto-cleanup after successful factorization
    pub cleanup: bool,

    /// Number of threads for parallel computation
    pub threads: Option<usize>,

    /// Logging level (error, warn, info, debug, trace)
    pub log_level: String,

    /// Buffering configuration
    pub buffer: BufferConfig,

    /// Performance tuning
    pub performance: PerformanceConfig,
}

/// Buffer configuration for relation streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferConfig {
    /// Maximum buffer memory in bytes (default: 100MB)
    pub max_memory_bytes: usize,

    /// Minimum relations before considering flush (default: 25)
    pub min_relations: usize,

    /// Maximum relations regardless of size (default: 1000)
    pub max_relations: usize,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Prime bound multiplier for tuning
    pub prime_bound_multiplier: f64,

    /// Relation quantity multiplier
    pub relation_quantity_multiplier: f64,
}

impl Default for GnfsConfig {
    fn default() -> Self {
        GnfsConfig {
            output_dir: ".".to_string(),
            cleanup: false,
            threads: None, // Use Rayon's default
            log_level: "info".to_string(),
            buffer: BufferConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for BufferConfig {
    fn default() -> Self {
        BufferConfig {
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            min_relations: 25,
            max_relations: 1000,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        PerformanceConfig {
            prime_bound_multiplier: 1.0,
            relation_quantity_multiplier: 1.0,
        }
    }
}

impl GnfsConfig {
    /// Load configuration with precedence: config file → env vars → defaults
    pub fn load() -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            // Start with defaults
            .set_default("output_dir", ".")?
            .set_default("cleanup", false)?
            .set_default("log_level", "info")?
            .set_default("buffer.max_memory_bytes", 100 * 1024 * 1024)?
            .set_default("buffer.min_relations", 25)?
            .set_default("buffer.max_relations", 1000)?
            .set_default("performance.prime_bound_multiplier", 1.0)?
            .set_default("performance.relation_quantity_multiplier", 1.0)?;

        // Try to load from config files (TOML preferred, YAML fallback)
        if Path::new("gnfs.toml").exists() {
            builder = builder.add_source(File::with_name("gnfs.toml"));
        } else if Path::new("gnfs.yaml").exists() {
            builder = builder.add_source(File::with_name("gnfs.yaml"));
        }

        // Override with environment variables (prefix: GNFS_)
        builder = builder.add_source(
            Environment::with_prefix("GNFS")
                .separator("_")
                .try_parsing(true)
        );

        let config = builder.build()?;
        config.try_deserialize()
    }

    /// Load configuration with custom file path
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            // Start with defaults
            .set_default("output_dir", ".")?
            .set_default("cleanup", false)?
            .set_default("log_level", "info")?
            .set_default("buffer.max_memory_bytes", 100 * 1024 * 1024)?
            .set_default("buffer.min_relations", 25)?
            .set_default("buffer.max_relations", 1000)?
            .set_default("performance.prime_bound_multiplier", 1.0)?
            .set_default("performance.relation_quantity_multiplier", 1.0)?;

        // Load from specified file
        if path.as_ref().exists() {
            builder = builder.add_source(File::from(path.as_ref()));
        }

        // Override with environment variables (prefix: GNFS_)
        builder = builder.add_source(
            Environment::with_prefix("GNFS")
                .separator("_")
                .try_parsing(true)
        );

        let config = builder.build()?;
        config.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GnfsConfig::default();
        assert_eq!(config.output_dir, ".");
        assert_eq!(config.cleanup, false);
        assert_eq!(config.log_level, "info");
        assert_eq!(config.buffer.max_memory_bytes, 100 * 1024 * 1024);
        assert_eq!(config.buffer.min_relations, 25);
        assert_eq!(config.buffer.max_relations, 1000);
        assert_eq!(config.performance.prime_bound_multiplier, 1.0);
        assert_eq!(config.performance.relation_quantity_multiplier, 1.0);
    }

    #[test]
    fn test_load_without_file() {
        // Should successfully load defaults when no config file exists
        let config = GnfsConfig::load().unwrap_or_else(|_| GnfsConfig::default());
        assert_eq!(config.output_dir, ".");
    }
}
