pub mod cache;
pub mod search_enhanced;
pub mod streaming;

// Re-export key types for external use
pub use cache::{CacheConfig, SmartCache, CacheStats};
pub use search_enhanced::{IncrementalSearch, SearchConfig, EnhancedSearchResult};
pub use streaming::{StreamingImporter, StreamConfig, StreamImportResult};

// Performance utilities
pub mod perf {
    use std::time::Instant;
    
    /// Simple performance timer
    pub struct Timer {
        start: Instant,
        name: String,
    }
    
    impl Timer {
        pub fn new(name: &str) -> Self {
            Self {
                start: Instant::now(),
                name: name.to_string(),
            }
        }
        
        pub fn elapsed_ms(&self) -> u128 {
            self.start.elapsed().as_millis()
        }
    }
    
    impl Drop for Timer {
        fn drop(&mut self) {
            tracing::debug!("{} completed in {}ms", self.name, self.elapsed_ms());
        }
    }
}

// Common error types
#[derive(Debug, thiserror::Error)]
pub enum LlmArchiveError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Search error: {0}")]
    Search(String),
    
    #[error("Import error: {0}")]
    Import(String),
}

pub type Result<T> = std::result::Result<T, LlmArchiveError>;