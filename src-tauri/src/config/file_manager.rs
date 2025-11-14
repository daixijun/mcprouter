//! File management tools
//!
//! Provide configuration file read/write, creation, backup and other functions

use super::{ConfigError, Result};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

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

/// Read JSON file
pub fn read_json<P: AsRef<Path>, T: serde::de::DeserializeOwned>(path: P) -> Result<T> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(ConfigError::NotFound(path.to_path_buf()));
    }

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data = serde_json::from_reader(reader)?;

    Ok(data)
}

/// Check if file exists
pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

/// Delete file
pub fn remove_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_file(path).map_err(ConfigError::Io)
    } else {
        Ok(())
    }
}

/// Get all files in directory
pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path.as_ref())
        .map_err(ConfigError::Io)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();

    Ok(entries)
}
