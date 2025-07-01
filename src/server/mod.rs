use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    middleware,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tower_http::services::ServeDir;
use tracing::info;

use crate::{
    config::Config,
    errors::{AppError, AppResult},
    models::{Conversation, Message, SearchResult},
    search,
};

mod templates;
use templates::*;

/// Application state
#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    config: Config,
}

/// Run the web server
pub async fn run(port: u16, database: PathBuf, config: Config) -> Result<()> {
    // Initialize metrics
    crate::metrics::init_metrics()?;
    
    // Create database pool
    let pool = crate::db::create_pool(&database).await?;
    crate::db::run_migrations(&pool).await?;
    
    // Start background stats updater
    let stats_pool = pool.clone();
    tokio::spawn(async move {
        crate::metrics::update_stats_task(stats_pool).await;
    });
    
    let state = Arc::new(AppState { pool, config });
    
    // Build router
    let app = Router::new()
        // Pages
        .route("/", get(index_page))
        .route("/search", get(search_page))
        .route("/conversation/:id", get(conversation_page))
        
        // API endpoints
        .route("/api/search", get(search_api))
        .route("/api/conversation/:id", get(conversation_api))
        .route("/api/conversation/:id/messages", get(messages_api))
        .route("/api/suggestions", get(suggestions_api))
        .route("/api/stats", get(stats_api))
        
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        
        // Health check
        .route("/health", get(health_check))
        
        // Metrics endpoint
        .route("/metrics", get(metrics_endpoint))
        
        // Add metrics middleware
        .layer(middleware::from_fn(crate::metrics::middleware::track_metrics))
        
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await?;
    
    info!("Server running at http://127.0.0.1:{}", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Index page
async fn index_page(State(state): State<Arc<AppState>>) -> AppResult<Html<String>> {
    let stats = get_stats(&state.pool).await?;
    let html = render_index(&stats)?;
    Ok(Html(html))
}

/// Search page
async fn search_page(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> AppResult<Html<String>> {
    let results = if let Some(query) = &params.q {
        search::search_with_snippets(
            &state.pool,
            query,
            params.limit.unwrap_or(20),
            state.config.search.snippet_length,
        )
        .await?
    } else {
        Vec::new()
    };
    
    let html = render_search_results(&params.q.unwrap_or_default(), &results)?;
    Ok(Html(html))
}

/// Conversation page
async fn conversation_page(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> AppResult<Html<String>> {
    let conversation = get_conversation(&state.pool, id).await?;
    let messages = search::get_conversation_messages(&state.pool, id).await?;
    
    let html = render_conversation(&conversation, &messages)?;
    Ok(Html(html))
}

/// Search API endpoint
#[derive(Deserialize)]
struct SearchParams {
    q: Option<String>,
    limit: Option<usize>,
    provider: Option<String>,
    model: Option<String>,
}

async fn search_api(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> AppResult<Json<Vec<SearchResult>>> {
    let query = params.q.ok_or_else(|| AppError::BadRequest("Missing query parameter".into()))?;
    
    let start = Instant::now();
    let results = search::search_with_snippets(
        &state.pool,
        &query,
        params.limit.unwrap_or(20),
        state.config.search.snippet_length,
    )
    .await?;
    
    let duration = start.elapsed();
    crate::metrics::track_search(params.provider.as_deref(), results.len(), duration);
    
    Ok(Json(results))
}

/// Get single conversation
async fn conversation_api(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> AppResult<Json<Conversation>> {
    let conversation = get_conversation(&state.pool, id).await?;
    Ok(Json(conversation))
}

/// Get conversation messages
async fn messages_api(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> AppResult<Json<Vec<Message>>> {
    let messages = search::get_conversation_messages(&state.pool, id).await?;
    Ok(Json(messages))
}

/// Search suggestions
#[derive(Deserialize)]
struct SuggestionsParams {
    prefix: String,
    limit: Option<usize>,
}

async fn suggestions_api(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SuggestionsParams>,
) -> AppResult<Json<Vec<String>>> {
    let suggestions = search::get_search_suggestions(
        &state.pool,
        &params.prefix,
        params.limit.unwrap_or(10),
    )
    .await?;
    
    Ok(Json(suggestions))
}

/// Statistics endpoint
#[derive(Serialize)]
struct Stats {
    total_conversations: i64,
    total_messages: i64,
    providers: Vec<ProviderStats>,
}

#[derive(Serialize)]
struct ProviderStats {
    name: String,
    count: i64,
}

async fn stats_api(State(state): State<Arc<AppState>>) -> AppResult<Json<Stats>> {
    let stats = get_stats(&state.pool).await?;
    Ok(Json(stats))
}

/// Health check
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

/// Metrics endpoint
async fn metrics_endpoint() -> impl IntoResponse {
    let encoder = metrics_exporter_prometheus::Encoder::new();
    let metric_families = encoder.encode();
    let mut buffer = String::new();
    
    if let Err(e) = encoder.encode_fmt(&metric_families, &mut buffer) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to encode metrics: {}", e));
    }
    
    (StatusCode::OK, buffer)
}

/// Helper functions
async fn get_conversation(pool: &SqlitePool, id: i64) -> AppResult<Conversation> {
    sqlx::query_as!(
        Conversation,
        r#"
        SELECT 
            id as "id!",
            provider as "provider!",
            external_id,
            title,
            model,
            created_at as "created_at!",
            updated_at as "updated_at!",
            raw_json,
            system_prompt,
            temperature,
            max_tokens,
            user_id
        FROM conversations
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Conversation {} not found", id)))
}

async fn get_stats(pool: &SqlitePool) -> AppResult<Stats> {
    let total_conversations = sqlx::query!("SELECT COUNT(*) as count FROM conversations")
        .fetch_one(pool)
        .await?
        .count;
    
    let total_messages = sqlx::query!("SELECT COUNT(*) as count FROM messages")
        .fetch_one(pool)
        .await?
        .count;
    
    let providers = sqlx::query!(
        r#"
        SELECT provider, COUNT(*) as count
        FROM conversations
        GROUP BY provider
        ORDER BY count DESC
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| ProviderStats {
        name: row.provider,
        count: row.count,
    })
    .collect();
    
    Ok(Stats {
        total_conversations,
        total_messages,
        providers,
    })
}