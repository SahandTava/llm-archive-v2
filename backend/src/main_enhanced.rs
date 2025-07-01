use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

mod cache;
mod parsers;
mod search_enhanced;
mod streaming;

use cache::{cache_maintenance_task, ConversationCache, SearchCache};
use search_enhanced::{EnhancedSearch, SearchDSL};
use streaming::StreamingImporter;

#[derive(Clone)]
struct AppState {
    db: Pool<Sqlite>,
    search_cache: Arc<SearchCache>,
    conv_cache: Arc<ConversationCache>,
    search_engine: Arc<EnhancedSearch>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize database
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:llm_archive.db".to_string());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Initialize caches
    let search_cache = Arc::new(SearchCache::new());
    let conv_cache = Arc::new(ConversationCache::new());
    let search_engine = Arc::new(EnhancedSearch::new(search_cache.clone()));

    // Start cache maintenance task
    let cache_search = search_cache.clone();
    let cache_conv = conv_cache.clone();
    tokio::spawn(async move {
        cache_maintenance_task(cache_search, cache_conv).await;
    });

    let state = AppState {
        db: pool,
        search_cache,
        conv_cache,
        search_engine,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/search", get(search_handler))
        .route("/api/advanced-search", get(advanced_search_handler))
        .route("/api/conversations", get(list_conversations))
        .route("/api/conversations/:id", get(get_conversation))
        .route("/api/conversations/:id/messages", get(get_messages))
        .route("/api/conversations/:id/export", get(export_conversation))
        .route("/api/import", post(import_handler))
        .route("/api/import/stream", post(streaming_import_handler))
        .route("/api/stats", get(stats_handler))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .into_inner(),
        )
        .with_state(state);

    println!("Server running on http://localhost:8000");
    axum::Server::bind(&"0.0.0.0:8000".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn health() -> &'static str {
    "OK"
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: i32,
}

fn default_limit() -> i32 {
    50
}

async fn search_handler(
    Query(params): Query<SearchQuery>,
    State(state): State<AppState>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let mut conn = state.db.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let results = state
        .search_engine
        .incremental_search(&mut conn, &params.q, params.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SearchResponse { results }))
}

async fn advanced_search_handler(
    Query(params): Query<SearchQuery>,
    State(state): State<AppState>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let mut conn = state.db.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let dsl = SearchDSL::parse(&params.q);
    let results = state
        .search_engine
        .advanced_search(&mut conn, &dsl)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SearchResponse { results }))
}

#[derive(Serialize)]
struct SearchResponse {
    results: Vec<cache::SearchResult>,
}

#[derive(Serialize)]
struct StatsResponse {
    total_conversations: i64,
    total_messages: i64,
    providers: std::collections::HashMap<String, i64>,
    models: std::collections::HashMap<String, i64>,
    messages_by_role: std::collections::HashMap<String, i64>,
    activity_by_hour: Vec<i64>,
    growth_by_month: Vec<MonthGrowth>,
}

#[derive(Serialize)]
struct MonthGrowth {
    month: String,
    count: i64,
}

async fn stats_handler(State(state): State<AppState>) -> Result<Json<StatsResponse>, StatusCode> {
    let mut conn = state.db.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Get basic counts
    let total_conversations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations")
        .fetch_one(&mut conn)
        .await
        .unwrap_or(0);
        
    let total_messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(&mut conn)
        .await
        .unwrap_or(0);
    
    // Provider distribution
    let provider_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT provider, COUNT(*) as count FROM conversations GROUP BY provider"
    )
    .fetch_all(&mut conn)
    .await
    .unwrap_or_default();
    
    let providers: std::collections::HashMap<String, i64> = provider_rows.into_iter().collect();
    
    // Model distribution
    let model_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT model_slug, COUNT(*) as count FROM conversations 
         WHERE model_slug IS NOT NULL 
         GROUP BY model_slug 
         ORDER BY count DESC"
    )
    .fetch_all(&mut conn)
    .await
    .unwrap_or_default();
    
    let models: std::collections::HashMap<String, i64> = model_rows.into_iter().collect();
    
    // Messages by role
    let role_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT role, COUNT(*) as count FROM messages GROUP BY role"
    )
    .fetch_all(&mut conn)
    .await
    .unwrap_or_default();
    
    let messages_by_role: std::collections::HashMap<String, i64> = role_rows.into_iter().collect();
    
    // Activity by hour (simplified - would need timestamp parsing in real implementation)
    let activity_by_hour = vec![0; 24]; // Placeholder
    
    // Growth by month
    let growth_by_month = vec![]; // Placeholder
    
    Ok(Json(StatsResponse {
        total_conversations,
        total_messages,
        providers,
        models,
        messages_by_role,
        activity_by_hour,
        growth_by_month,
    }))
}

#[derive(Deserialize)]
struct ExportQuery {
    format: Option<String>,
}

async fn export_conversation(
    Path(id): Path<i64>,
    Query(params): Query<ExportQuery>,
    State(state): State<AppState>,
) -> Result<String, StatusCode> {
    let format = params.format.unwrap_or_else(|| "markdown".to_string());
    
    let mut conn = state.db.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Get conversation and messages
    let messages = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT role, content, timestamp FROM messages WHERE conversation_id = ? ORDER BY position"
    )
    .bind(id)
    .fetch_all(&mut conn)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match format.as_str() {
        "json" => {
            // Export as JSON
            Ok(serde_json::to_string_pretty(&messages).unwrap())
        }
        "academic" => {
            // Academic format with citations
            let mut output = String::from("# Conversation Transcript\n\n");
            output.push_str("## Abstract\n\nThis document presents a transcript of a conversation between a human user and an AI assistant.\n\n");
            output.push_str("## Transcript\n\n");
            
            for (i, (role, content, timestamp)) in messages.iter().enumerate() {
                output.push_str(&format!("### Exchange {} [{}]\n\n", i + 1, timestamp));
                output.push_str(&format!("**{}**: {}\n\n", role.to_uppercase(), content));
            }
            
            output.push_str("## References\n\n[1] AI Conversation Archive, ");
            output.push_str(&format!("accessed {}\n", chrono::Local::now().format("%Y-%m-%d")));
            
            Ok(output)
        }
        "blog" => {
            // Blog post format
            let mut output = String::from("# AI Conversation Highlights\n\n");
            output.push_str("*An interesting discussion with AI about various topics.*\n\n");
            
            for (role, content, _) in messages {
                if role == "user" {
                    output.push_str(&format!("> **Question**: {}\n\n", content));
                } else {
                    output.push_str(&format!("{}\n\n---\n\n", content));
                }
            }
            
            Ok(output)
        }
        _ => {
            // Default markdown format
            let mut output = String::from("# Conversation Export\n\n");
            
            for (role, content, timestamp) in messages {
                output.push_str(&format!("## {} ({})\n\n{}\n\n", 
                    role.to_uppercase(), 
                    chrono::NaiveDateTime::from_timestamp_opt(timestamp, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    content
                ));
            }
            
            Ok(output)
        }
    }
}

async fn streaming_import_handler(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<StreamingImportResponse>, StatusCode> {
    let importer = StreamingImporter::new();
    
    // In production, this would handle file uploads
    // For now, assume body contains a file path
    let file_path = body.trim();
    
    let stream = importer
        .stream_file(file_path)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let stats = importer
        .parallel_import(stream, |conversations| async {
            // Insert batch into database
            // This is simplified - real implementation would use transactions
            Ok(conversations.len())
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(StreamingImportResponse {
        imported: stats.imported,
        errors: stats.errors,
    }))
}

#[derive(Serialize)]
struct StreamingImportResponse {
    imported: usize,
    errors: Vec<String>,
}

// Placeholder implementations for other handlers
async fn list_conversations(State(_state): State<AppState>) -> Json<Vec<String>> {
    Json(vec![])
}

async fn get_conversation(Path(_id): Path<i64>, State(_state): State<AppState>) -> Json<String> {
    Json("Conversation".to_string())
}

async fn get_messages(Path(_id): Path<i64>, State(_state): State<AppState>) -> Json<Vec<String>> {
    Json(vec![])
}

async fn import_handler(State(_state): State<AppState>) -> Json<String> {
    Json("Import successful".to_string())
}