//! File management tools
//!
//! Provide configuration file read/write, creation, backup and other functions

use super::Result;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

/// Atomic file write
pub fn write_json_atomic<P: AsRef<Path>, T: serde::Serialize>(path: P, data: &T) -> Result<()> {
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");

    tracing::info!("write_json_atomic: starting to write file {:?}", path);

    // Create directory
    if let Some(parent) = path.parent() {
        tracing::info!("write_json_atomic: creating parent directory {:?}", parent);
        fs::create_dir_all(parent)?;
    }

    // Write temp file
    let file = File::create(&temp_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, data)?;
    tracing::info!("write_json_atomic: data written to temp file successfully");

    // Atomic rename
    fs::rename(&temp_path, path)?;
    tracing::info!("write_json_atomic: file rename successful");

    Ok(())
}
