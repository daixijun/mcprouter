// Migration runner for applying database migrations
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::path::Path;

/// Run database migrations
pub async fn run_migrations(database_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let database_url = format!("sqlite:{}", database_path.display());

    // Create connection for migration
    let options = SqliteConnectOptions::new()
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(options).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("✅ All migrations applied successfully");

    Ok(())
}