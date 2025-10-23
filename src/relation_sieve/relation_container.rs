// src/relation_sieve/relation_container.rs

use std::vec::Vec;
use std::path::PathBuf;
use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter, BufReader, BufRead};
use log::{info, warn};

use super::relation::Relation;
use crate::core::serialization::types::SerializableRelation;

/// Buffer size for flushing relations to disk
const SMOOTH_RELATIONS_BUFFER_SIZE: usize = 50;

#[derive(Default, Debug, Clone)]
pub struct RelationContainer {
    pub smooth_relations: Vec<Relation>,
    pub rough_relations: Vec<Relation>,
    pub free_relations: Vec<Vec<Relation>>,

    // Streaming support for smooth relations
    pub streaming_file_path: Option<PathBuf>,
    pub total_streamed_count: usize,
    pub buffer_size: usize,
}

impl RelationContainer {
    pub fn new() -> Self {
        RelationContainer {
            smooth_relations: Vec::new(),
            rough_relations: Vec::new(),
            free_relations: Vec::new(),
            streaming_file_path: None,
            total_streamed_count: 0,
            buffer_size: SMOOTH_RELATIONS_BUFFER_SIZE,
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
    pub fn add_smooth_relations(&mut self, mut relations: Vec<Relation>) -> Result<(), String> {
        // Add to buffer
        self.smooth_relations.append(&mut relations);

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
        for relation in &self.smooth_relations {
            let serializable = SerializableRelation::from(relation.clone());
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

        // Clear buffer
        self.smooth_relations.clear();

        Ok(())
    }

    /// Load all relations from disk (for matrix construction stage)
    pub fn load_all_from_disk(&mut self) -> Result<Vec<Relation>, String> {
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

            relations.push(Relation::from(serializable));
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