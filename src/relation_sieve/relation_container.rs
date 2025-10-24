// src/relation_sieve/relation_container.rs

use std::vec::Vec;
use std::path::PathBuf;
use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter, BufReader, BufRead};
use log::{info, warn};

use super::relation::Relation;
use crate::core::serialization::types::SerializableRelation;
use crate::core::gnfs_integer::GnfsInteger;
use crate::config::BufferConfig;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct RelationContainer<T: GnfsInteger> {
    pub smooth_relations: Vec<Relation<T>>,
    pub rough_relations: Vec<Relation<T>>,
    pub free_relations: Vec<Vec<Relation<T>>>,

    // Streaming support for smooth relations
    pub streaming_file_path: Option<PathBuf>,
    pub total_streamed_count: usize,

    // Size-based buffering configuration
    pub buffer_config: BufferConfig,
    pub current_buffer_size_bytes: usize,
    pub digit_count: usize, // For size estimation

    pub(crate) _phantom: PhantomData<T>,
}

impl<T: GnfsInteger> RelationContainer<T> {
    /// Create a new RelationContainer with default buffer configuration
    pub fn new() -> Self {
        Self::with_config(BufferConfig::default(), 0)
    }

    /// Create a new RelationContainer with custom buffer configuration
    pub fn with_config(buffer_config: BufferConfig, digit_count: usize) -> Self {
        info!("Initializing RelationContainer:");
        info!("  Buffer max memory: {:.2} MB", buffer_config.max_memory_bytes as f64 / (1024.0 * 1024.0));
        info!("  Min relations before flush: {}", buffer_config.min_relations);
        info!("  Max relations (safety limit): {}", buffer_config.max_relations);
        info!("  Digit count for size estimation: {}", digit_count);

        RelationContainer {
            smooth_relations: Vec::new(),
            rough_relations: Vec::new(),
            free_relations: Vec::new(),
            streaming_file_path: None,
            total_streamed_count: 0,
            buffer_config,
            current_buffer_size_bytes: 0,
            digit_count,
            _phantom: PhantomData,
        }
    }

    /// Estimate the size of a single relation in bytes
    /// Based on: 2 norms + 2 quotients + 2 factorizations (each with ~10-50 BigInt pairs)
    fn estimate_relation_size(&self) -> usize {
        // Base overhead: 4 BigInt values (a, b, algebraic_norm, rational_norm, etc.)
        let base_size = 8 * 100; // ~800 bytes for metadata

        // Factor count scales with digit count and prime bound
        let avg_factors = (self.digit_count * 5).max(10); // Conservative estimate

        // Each factor pair: (BigInt key, i32 exponent) ≈ 20-50 bytes per pair
        let factor_size = avg_factors * 35;

        base_size + (factor_size * 2) // algebraic + rational factorizations
    }

    /// Initialize streaming to disk
    pub fn init_streaming(&mut self, file_path: PathBuf) {
        self.streaming_file_path = Some(file_path.clone());
        info!("Initialized relation streaming to: {}", file_path.display());

        // Clear any existing streaming file
        if file_path.exists() {
            if let Err(e) = std::fs::remove_file(&file_path) {
                warn!("Failed to remove existing streaming file: {}", e);
            }
        }
    }

    /// Add smooth relations with automatic streaming to disk
    pub fn add_smooth_relations(&mut self, mut relations: Vec<Relation<T>>) -> Result<(), String> {
        // CRITICAL FIX: Shrink incoming vector BEFORE appending to prevent capacity inflation
        // Rayon's parallel collect() over-allocates, this prevents propagation to buffer
        relations.shrink_to_fit();

        // Estimate size of incoming relations
        let estimated_size = relations.len() * self.estimate_relation_size();
        self.current_buffer_size_bytes += estimated_size;

        // Add to buffer
        self.smooth_relations.append(&mut relations);

        // MEMORY LEAK FIX: Log buffer capacity to track over-allocation
        log::trace!("Buffer state: len={}, capacity={}, size_bytes={:.2}MB, wasted_capacity={}",
                   self.smooth_relations.len(),
                   self.smooth_relations.capacity(),
                   self.current_buffer_size_bytes as f64 / (1024.0 * 1024.0),
                   self.smooth_relations.capacity().saturating_sub(self.smooth_relations.len()));

        // Flush if:
        // 1. Buffer exceeds max memory, AND
        // 2. We have at least min_relations, OR
        // 3. We've exceeded max_relations (safety limit)
        let should_flush =
            (self.current_buffer_size_bytes >= self.buffer_config.max_memory_bytes
             && self.smooth_relations.len() >= self.buffer_config.min_relations)
            || self.smooth_relations.len() >= self.buffer_config.max_relations;

        if should_flush {
            info!("Flushing {} relations (~{:.2}MB buffer)",
                  self.smooth_relations.len(),
                  self.current_buffer_size_bytes as f64 / (1024.0 * 1024.0));
            self.flush_to_disk()?;
            self.current_buffer_size_bytes = 0;
        }

        Ok(())
    }

    /// Force flush all buffered relations to disk
    pub fn flush_to_disk(&mut self) -> Result<(), String> {
        if self.smooth_relations.is_empty() {
            return Ok(());
        }

        let file_path = match &self.streaming_file_path {
            Some(path) => path,
            None => {
                // No streaming configured, keep in memory
                return Ok(());
            }
        };

        // Open file in append mode
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .map_err(|e| format!("Failed to open streaming file: {}", e))?;

        let mut writer = BufWriter::new(file);

        // Write relations as JSON lines (one JSON object per line)
        // CRITICAL FIX: Use reference-based conversion to eliminate clone and prevent 6GB memory spikes
        for relation in &self.smooth_relations {
            // Use From<&Relation<T>> instead of From<Relation<T>> - NO CLONE!
            let serializable = SerializableRelation::from(relation);
            let json = serde_json::to_string(&serializable)
                .map_err(|e| format!("Failed to serialize relation: {}", e))?;

            writeln!(writer, "{}", json)
                .map_err(|e| format!("Failed to write relation: {}", e))?;
        }

        writer.flush()
            .map_err(|e| format!("Failed to flush writer: {}", e))?;

        let flushed_count = self.smooth_relations.len();
        self.total_streamed_count += flushed_count;

        info!("Flushed {} relations to disk (total streamed: {})",
              flushed_count, self.total_streamed_count);

        // MEMORY LEAK FIX: Clear buffer AND shrink capacity to reclaim memory
        // Vec::clear() does not deallocate, which causes capacity to grow unbounded
        // We explicitly shrink to minimal capacity to avoid memory leak
        let old_capacity = self.smooth_relations.capacity();
        self.smooth_relations.clear();
        self.smooth_relations.shrink_to_fit();

        log::debug!("Buffer cleared and shrunk: old_capacity={}, new_capacity={}",
                   old_capacity, self.smooth_relations.capacity());

        Ok(())
    }

    /// Load all relations from disk (for matrix construction stage)
    pub fn load_all_from_disk(&mut self) -> Result<Vec<Relation<T>>, String> {
        // First flush any remaining buffered relations
        self.flush_to_disk()?;

        let file_path = match &self.streaming_file_path {
            Some(path) => path,
            None => {
                // No streaming file, return buffer
                return Ok(self.smooth_relations.clone());
            }
        };

        if !file_path.exists() {
            // No file exists yet, return buffer
            return Ok(self.smooth_relations.clone());
        }

        info!("Loading relations from disk: {}", file_path.display());

        let file = File::open(file_path)
            .map_err(|e| format!("Failed to open streaming file: {}", e))?;

        let reader = BufReader::new(file);
        let mut relations = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| format!("Failed to read line {}: {}", line_num, e))?;

            if line.trim().is_empty() {
                continue;
            }

            let serializable: SerializableRelation = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to deserialize relation on line {}: {}", line_num, e))?;

            // Convert from SerializableRelation directly to Relation<T> using to_relation method
            let native_relation = serializable.to_relation::<T>();

            relations.push(native_relation);
        }

        info!("Loaded {} relations from disk", relations.len());

        // Also include any relations still in buffer
        relations.extend(self.smooth_relations.clone());

        Ok(relations)
    }

    /// Get current smooth relations count (buffer + streamed)
    pub fn smooth_relations_count(&self) -> usize {
        self.smooth_relations.len() + self.total_streamed_count
    }
}

impl<T: GnfsInteger> Default for RelationContainer<T> {
    fn default() -> Self {
        Self::new()
    }
}