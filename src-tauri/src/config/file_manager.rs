//! 文件管理工具
//!
//! 提供配置文件的读写、创建、备份等功能

use super::{ConfigError, Result};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// 原子性文件写入
pub fn write_json_atomic<P: AsRef<Path>, T: serde::Serialize>(path: P, data: &T) -> Result<()> {
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");

    tracing::info!("write_json_atomic: 开始写入文件 {:?}", path);

    // 创建目录
    if let Some(parent) = path.parent() {
        tracing::info!("write_json_atomic: 创建父目录 {:?}", parent);
        fs::create_dir_all(parent)?;
    }

    // 写入临时文件
    let file = File::create(&temp_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, data)?;
    tracing::info!("write_json_atomic: 数据写入临时文件成功");

    // 原子性重命名
    fs::rename(&temp_path, path)?;
    tracing::info!("write_json_atomic: 文件重命名成功");

    Ok(())
}

/// 读取JSON文件
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

/// 检查文件是否存在
pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

/// 创建目录（如果不存在）
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::create_dir_all(path.as_ref()).map_err(ConfigError::Io)
}

/// 删除文件
pub fn remove_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_file(path).map_err(ConfigError::Io)
    } else {
        Ok(())
    }
}

/// 获取目录下的所有文件
pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path.as_ref())
        .map_err(ConfigError::Io)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();

    Ok(entries)
}

/// 备份文件
pub fn backup_file<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = path.as_ref();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = path.with_extension(format!("bak.{}", timestamp));

    if path.exists() {
        fs::copy(path, &backup_path).map_err(ConfigError::Io)?;
    }

    Ok(backup_path)
}

/// 恢复备份文件
pub fn restore_backup<P: AsRef<Path>>(backup_path: P, target_path: P) -> Result<()> {
    fs::copy(backup_path.as_ref(), target_path.as_ref()).map_err(ConfigError::Io)?;
    Ok(())
}

/// 获取父目录或当前目录
fn parent_or_current(path: &Path) -> &Path {
    path.parent().unwrap_or_else(|| Path::new("."))
}
