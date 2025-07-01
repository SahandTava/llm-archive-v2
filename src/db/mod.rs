use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use tracing::info;

pub mod schema;

/// Create a connection pool with optimized settings
pub async fn create_pool(path: &Path) -> Result<SqlitePool> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let url = format!("sqlite://{}?mode=rwc", path.display());
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    
    // Enable WAL mode for better concurrency
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    
    // Optimize for performance
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;
    
    sqlx::query("PRAGMA cache_size = -64000") // 64MB cache
        .execute(&pool)
        .await?;
    
    sqlx::query("PRAGMA temp_store = MEMORY")
        .execute(&pool)
        .await?;
    
    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    info!("Running database migrations");
    
    // Create tables with proper indexes from day 1
    sqlx::query(schema::CREATE_TABLES)
        .execute(pool)
        .await?;
    
    // Create FTS5 table for search
    sqlx::query(schema::CREATE_FTS)
        .execute(pool)
        .await?;
    
    // Create essential indexes
    sqlx::query(schema::CREATE_INDEXES)
        .execute(pool)
        .await?;
    
    info!("Database migrations completed");
    Ok(())
}