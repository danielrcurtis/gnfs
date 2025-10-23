// src/relation_sieve/relation_container.rs

use std::vec::Vec;
use std::path::PathBuf;
use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter, BufReader, BufRead};
use log::{info, warn};

use super::relation::Relation;
use crate::core::serialization::types::SerializableRelation;
use crate::core::gnfs_integer::GnfsInteger;
use std::marker::PhantomData;

/// Buffer size for flushing relations to disk
/// Configurable via GNFS_RELATION_BUFFER_SIZE environment variable (default: 50)
/// Balance between disk I/O frequency and memory usage
/// Too small (e.g., 5) causes excessive context switches and high system CPU usage
/// Original value of 50 provides good balance for most workloads
const SMOOTH_RELATIONS_BUFFER_SIZE: usize = 50;

#[derive(Debug, Clone)]
pub struct RelationContainer<T: GnfsInteger> {
    pub smooth_relations: Vec<Relation<T>>,
    pub rough_relations: Vec<Relation<T>>,
    pub free_relations: Vec<Vec<Relation<T>>>,

    // Streaming support for smooth relations
    pub streaming_file_path: Option<PathBuf>,
    pub total_streamed_count: usize,
    pub buffer_size: usize,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T: GnfsInteger> RelationContainer<T> {
    pub fn new() -> Self {
        // Allow buffer size to be configured via environment variable
        let buffer_size = std::env::var("GNFS_RELATION_BUFFER_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(SMOOTH_RELATIONS_BUFFER_SIZE);

        if buffer_size != SMOOTH_RELATIONS_BUFFER_SIZE {
            info!("Using custom relation buffer size: {} (default: {})",
                  buffer_size, SMOOTH_RELATIONS_BUFFER_SIZE);
        }

        RelationContainer {
            smooth_relations: Vec::new(),
            rough_relations: Vec::new(),
            free_relations: Vec::new(),
            streaming_file_path: None,
            total_streamed_count: 0,
            buffer_size,
            _phantom: PhantomData,
        }
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

        // Add to buffer
        self.smooth_relations.append(&mut relations);

        // MEMORY LEAK FIX: Log buffer capacity to track over-allocation
        log::trace!("Buffer state: len={}, capacity={}, wasted_capacity={}",
                   self.smooth_relations.len(),
                   self.smooth_relations.capacity(),
                   self.smooth_relations.capacity().saturating_sub(self.smooth_relations.len()));

        // Flush if buffer is full
        if self.smooth_relations.len() >= self.buffer_size {
            self.flush_to_disk()?;
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