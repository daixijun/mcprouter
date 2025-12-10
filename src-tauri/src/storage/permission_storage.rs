// Permission storage implementation (placeholder)
#![allow(dead_code)]

use super::SqlitePool;

/// Permission storage placeholder
/// TODO: Implement full permission storage with SQLite
pub struct PermissionStorage {
    pool: SqlitePool,
}

impl PermissionStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}