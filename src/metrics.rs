use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::{Duration, Instant};
use metrics::{counter, gauge, histogram, describe_counter, describe_gauge, describe_histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use tracing::info;

/// Initialize metrics system
pub fn init_metrics() -> anyhow::Result<()> {
    // Install Prometheus exporter
    PrometheusBuilder::new()
        .idle_timeout(
            metrics_util::MetricKindMask::COUNTER | metrics_util::MetricKindMask::HISTOGRAM,
            Some(Duration::from_secs(30)),
        )
        .install()?;
    
    // Register metric descriptions
    describe_counter!(
        "llm_archive_http_requests_total",
        "Total number of HTTP requests"
    );
    
    describe_counter!(
        "llm_archive_searches_total",
        "Total number of search queries"
    );
    
    describe_counter!(
        "llm_archive_imports_total",
        "Total number of import operations"
    );
    
    describe_counter!(
        "llm_archive_import_conversations_total",
        "Total number of conversations imported"
    );
    
    describe_counter!(
        "llm_archive_import_messages_total",
        "Total number of messages imported"
    );
    
    describe_gauge!(
        "llm_archive_database_size_bytes",
        "Size of the database file in bytes"
    );
    
    describe_gauge!(
        "llm_archive_conversations_count",
        "Total number of conversations in database"
    );
    
    describe_gauge!(
        "llm_archive_messages_count",
        "Total number of messages in database"
    );
    
    describe_histogram!(
        "llm_archive_http_request_duration_seconds",
        "HTTP request duration in seconds"
    );
    
    describe_histogram!(
        "llm_archive_search_duration_seconds",
        "Search query duration in seconds"
    );
    
    describe_histogram!(
        "llm_archive_import_duration_seconds",
        "Import operation duration in seconds"
    );
    
    info!("Metrics system initialized");
    Ok(())
}

/// Track HTTP request
pub fn track_http_request(method: &str, path: &str, status: u16, duration: Duration) {
    counter!(
        "llm_archive_http_requests_total",
        "method" => method.to_string(),
        "path" => sanitize_path(path),
        "status" => status.to_string(),
    ).increment(1);
    
    histogram!(
        "llm_archive_http_request_duration_seconds",
        "method" => method.to_string(),
        "path" => sanitize_path(path),
    ).record(duration.as_secs_f64());
}

/// Track search query
pub fn track_search(provider: Option<&str>, result_count: usize, duration: Duration) {
    counter!(
        "llm_archive_searches_total",
        "provider" => provider.unwrap_or("all").to_string(),
    ).increment(1);
    
    histogram!(
        "llm_archive_search_duration_seconds",
        "provider" => provider.unwrap_or("all").to_string(),
    ).record(duration.as_secs_f64());
}

/// Track import operation
pub fn track_import(provider: &str, conversations: usize, messages: usize, duration: Duration, success: bool) {
    counter!(
        "llm_archive_imports_total",
        "provider" => provider.to_string(),
        "status" => if success { "success" } else { "failure" }.to_string(),
    ).increment(1);
    
    if success {
        counter!(
            "llm_archive_import_conversations_total",
            "provider" => provider.to_string(),
        ).increment(conversations as u64);
        
        counter!(
            "llm_archive_import_messages_total",
            "provider" => provider.to_string(),
        ).increment(messages as u64);
    }
    
    histogram!(
        "llm_archive_import_duration_seconds",
        "provider" => provider.to_string(),
    ).record(duration.as_secs_f64());
}

/// Update database statistics
pub fn update_database_stats(size_bytes: u64, conversations: i64, messages: i64) {
    gauge!("llm_archive_database_size_bytes").set(size_bytes as f64);
    gauge!("llm_archive_conversations_count").set(conversations as f64);
    gauge!("llm_archive_messages_count").set(messages as f64);
}

/// Sanitize path for metrics labels
fn sanitize_path(path: &str) -> String {
    // Convert paths like /conversation/123 to /conversation/:id
    if path.starts_with("/conversation/") {
        return "/conversation/:id".to_string();
    }
    if path.starts_with("/api/conversation/") {
        return "/api/conversation/:id".to_string();
    }
    path.to_string()
}

/// Metrics middleware for Axum
pub mod middleware {
    use super::*;
    use axum::{
        body::Body,
        extract::Request,
        middleware::Next,
        response::Response,
    };
    use std::time::Instant;
    
    pub async fn track_metrics(
        req: Request,
        next: Next,
    ) -> Response {
        let start = Instant::now();
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        
        let response = next.run(req).await;
        let status = response.status().as_u16();
        let duration = start.elapsed();
        
        track_http_request(&method, &path, status, duration);
        
        response
    }
}

/// Background task to update database stats periodically
pub async fn update_stats_task(pool: sqlx::SqlitePool) {
    use tokio::time::{interval, Duration};
    
    let mut ticker = interval(Duration::from_secs(60)); // Update every minute
    
    loop {
        ticker.tick().await;
        
        // Get database file size
        if let Ok(metadata) = tokio::fs::metadata("./llm_archive.db").await {
            let size = metadata.len();
            
            // Get counts from database
            if let Ok(conv_count) = sqlx::query!("SELECT COUNT(*) as count FROM conversations")
                .fetch_one(&pool)
                .await
            {
                if let Ok(msg_count) = sqlx::query!("SELECT COUNT(*) as count FROM messages")
                    .fetch_one(&pool)
                    .await
                {
                    update_database_stats(size, conv_count.count, msg_count.count);
                }
            }
        }
    }
}