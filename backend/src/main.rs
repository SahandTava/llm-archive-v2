use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Row};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Data structures
#[derive(Debug, Serialize, Deserialize)]
struct Conversation {
    id: i64,
    provider: String,
    title: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    message_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    id: i64,
    conversation_id: i64,
    role: String,
    content: String,
    model: Option<String>,
    created_at: DateTime<Utc>,
    position: i32,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<i32>,
    offset: Option<i32>,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    conversation_id: i64,
    message_id: i64,
    content: String,
    snippet: String,
    conversation_title: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct ImportRequest {
    provider: String,
    data: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ImportResponse {
    conversations: i32,
    messages: i32,
    duration_ms: u128,
}

// Application state
struct AppState {
    pool: SqlitePool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_archive_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database connection
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:llm_archive.db".to_string());
    info!("Connecting to database: {}", database_url);
    
    let pool = SqlitePool::connect(&database_url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("Database migrations completed");

    // Create app state
    let state = Arc::new(AppState { pool });

    // Build router
    let app = Router::new()
        .route("/api/search", get(search))
        .route("/api/conversations", get(list_conversations))
        .route("/api/conversations/:id", get(get_conversation))
        .route("/api/conversations/:id/messages", get(get_messages))
        .route("/api/import", post(import_data))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let addr = "0.0.0.0:3000";
    info!("Starting server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

// Search endpoint - <100ms response time target
async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, StatusCode> {
    let start = std::time::Instant::now();
    
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);
    
    // Use FTS5 for fast full-text search
    let query = format!("%{}%", params.q);
    
    let results = sqlx::query!(
        r#"
        SELECT 
            m.id as message_id,
            m.conversation_id,
            m.content,
            m.created_at,
            c.title as conversation_title,
            snippet(messages_fts, 0, '<mark>', '</mark>', '...', 64) as snippet
        FROM messages_fts
        JOIN messages m ON messages_fts.rowid = m.id
        JOIN conversations c ON m.conversation_id = c.id
        WHERE messages_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2 OFFSET ?3
        "#,
        params.q,
        limit,
        offset
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        error!("Search query failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let search_results: Vec<SearchResult> = results
        .into_iter()
        .map(|row| SearchResult {
            conversation_id: row.conversation_id,
            message_id: row.message_id,
            content: row.content,
            snippet: row.snippet,
            conversation_title: row.conversation_title,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .unwrap_or_default()
                .with_timezone(&Utc),
        })
        .collect();

    let duration = start.elapsed();
    info!("Search completed in {:?} for query: {}", duration, params.q);
    
    Ok(Json(search_results))
}

// List conversations - paginated
async fn list_conversations(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<Conversation>>, StatusCode> {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);

    let conversations = sqlx::query!(
        r#"
        SELECT 
            c.id,
            c.title,
            c.created_at,
            c.updated_at,
            c.message_count,
            p.name as provider
        FROM conversations c
        JOIN providers p ON c.provider_id = p.id
        ORDER BY c.updated_at DESC
        LIMIT ?1 OFFSET ?2
        "#,
        limit,
        offset
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        error!("Failed to list conversations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let result: Vec<Conversation> = conversations
        .into_iter()
        .map(|row| Conversation {
            id: row.id,
            provider: row.provider,
            title: row.title,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .unwrap_or_default()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .unwrap_or_default()
                .with_timezone(&Utc),
            message_count: row.message_count,
        })
        .collect();

    Ok(Json(result))
}

// Get single conversation
async fn get_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Conversation>, StatusCode> {
    let conversation = sqlx::query!(
        r#"
        SELECT 
            c.id,
            c.title,
            c.created_at,
            c.updated_at,
            c.message_count,
            p.name as provider
        FROM conversations c
        JOIN providers p ON c.provider_id = p.id
        WHERE c.id = ?1
        "#,
        id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        error!("Failed to get conversation {}: {}", id, e);
        StatusCode::NOT_FOUND
    })?;

    Ok(Json(Conversation {
        id: conversation.id,
        provider: conversation.provider,
        title: conversation.title,
        created_at: DateTime::parse_from_rfc3339(&conversation.created_at)
            .unwrap_or_default()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&conversation.updated_at)
            .unwrap_or_default()
            .with_timezone(&Utc),
        message_count: conversation.message_count,
    }))
}

// Get messages for a conversation
async fn get_messages(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<i64>,
) -> Result<Json<Vec<Message>>, StatusCode> {
    let messages = sqlx::query!(
        r#"
        SELECT 
            id,
            conversation_id,
            role,
            content,
            model,
            created_at,
            position
        FROM messages
        WHERE conversation_id = ?1
        ORDER BY position ASC
        "#,
        conversation_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        error!("Failed to get messages for conversation {}: {}", conversation_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let result: Vec<Message> = messages
        .into_iter()
        .map(|row| Message {
            id: row.id,
            conversation_id: row.conversation_id,
            role: row.role,
            content: row.content,
            model: row.model,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .unwrap_or_default()
                .with_timezone(&Utc),
            position: row.position,
        })
        .collect();

    Ok(Json(result))
}

// Import data endpoint
async fn import_data(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ImportRequest>,
) -> Result<Json<ImportResponse>, StatusCode> {
    let start = std::time::Instant::now();
    
    // Get provider ID
    let provider_id = sqlx::query!("SELECT id FROM providers WHERE name = ?1", request.provider)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .id;

    // Parse based on provider type
    let (conversations, messages) = match request.provider.as_str() {
        "chatgpt" => parse_chatgpt(request.data, provider_id, &state.pool).await?,
        "claude" => parse_claude(request.data, provider_id, &state.pool).await?,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let duration_ms = start.elapsed().as_millis();
    
    Ok(Json(ImportResponse {
        conversations,
        messages,
        duration_ms,
    }))
}

// Simple ChatGPT parser
async fn parse_chatgpt(
    data: serde_json::Value,
    provider_id: i64,
    pool: &SqlitePool,
) -> Result<(i32, i32), StatusCode> {
    let mut conversation_count = 0;
    let mut message_count = 0;

    if let Some(conversations) = data.as_array() {
        for conv in conversations {
            // Extract conversation data
            let title = conv.get("title").and_then(|t| t.as_str()).unwrap_or("Untitled");
            let created_at = conv.get("create_time")
                .and_then(|t| t.as_f64())
                .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_default())
                .unwrap_or_else(Utc::now);
            let updated_at = conv.get("update_time")
                .and_then(|t| t.as_f64())
                .map(|t| DateTime::from_timestamp(t as i64, 0).unwrap_or_default())
                .unwrap_or(created_at);

            // Insert conversation
            let conv_id = sqlx::query!(
                "INSERT INTO conversations (provider_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                provider_id,
                title,
                created_at.to_rfc3339(),
                updated_at.to_rfc3339()
            )
            .execute(pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .last_insert_rowid();

            conversation_count += 1;

            // Insert messages
            if let Some(mapping) = conv.get("mapping").and_then(|m| m.as_object()) {
                let mut messages_vec = Vec::new();
                
                for (_, node) in mapping {
                    if let Some(message) = node.get("message") {
                        if let Some(content) = message.get("content").and_then(|c| c.get("parts")).and_then(|p| p.as_array()) {
                            let role = message.get("author").and_then(|a| a.get("role")).and_then(|r| r.as_str()).unwrap_or("user");
                            let text = content.iter()
                                .filter_map(|part| part.as_str())
                                .collect::<Vec<_>>()
                                .join("");
                            
                            if !text.is_empty() {
                                messages_vec.push((role.to_string(), text, created_at));
                            }
                        }
                    }
                }

                // Insert messages in order
                for (position, (role, content, timestamp)) in messages_vec.iter().enumerate() {
                    sqlx::query!(
                        "INSERT INTO messages (conversation_id, role, content, created_at, position) VALUES (?1, ?2, ?3, ?4, ?5)",
                        conv_id,
                        role,
                        content,
                        timestamp.to_rfc3339(),
                        position as i32
                    )
                    .execute(pool)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    
                    message_count += 1;
                }

                // Update message count
                sqlx::query!(
                    "UPDATE conversations SET message_count = ?1 WHERE id = ?2",
                    messages_vec.len() as i32,
                    conv_id
                )
                .execute(pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
        }
    }

    Ok((conversation_count, message_count))
}

// Simple Claude parser
async fn parse_claude(
    data: serde_json::Value,
    provider_id: i64,
    pool: &SqlitePool,
) -> Result<(i32, i32), StatusCode> {
    let mut conversation_count = 0;
    let mut message_count = 0;

    if let Some(conversations) = data.get("conversations").and_then(|c| c.as_array()) {
        for conv in conversations {
            let title = conv.get("name").and_then(|t| t.as_str()).unwrap_or("Untitled");
            let created_at = conv.get("created_at")
                .and_then(|t| t.as_str())
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            let updated_at = conv.get("updated_at")
                .and_then(|t| t.as_str())
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(created_at);

            // Insert conversation
            let conv_id = sqlx::query!(
                "INSERT INTO conversations (provider_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                provider_id,
                title,
                created_at.to_rfc3339(),
                updated_at.to_rfc3339()
            )
            .execute(pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .last_insert_rowid();

            conversation_count += 1;

            // Insert messages
            if let Some(messages) = conv.get("messages").and_then(|m| m.as_array()) {
                for (position, msg) in messages.iter().enumerate() {
                    let role = msg.get("sender").and_then(|s| s.as_str()).unwrap_or("user");
                    let content = msg.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    
                    if !content.is_empty() {
                        sqlx::query!(
                            "INSERT INTO messages (conversation_id, role, content, created_at, position) VALUES (?1, ?2, ?3, ?4, ?5)",
                            conv_id,
                            role,
                            content,
                            created_at.to_rfc3339(),
                            position as i32
                        )
                        .execute(pool)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                        
                        message_count += 1;
                    }
                }

                // Update message count
                sqlx::query!(
                    "UPDATE conversations SET message_count = ?1 WHERE id = ?2",
                    messages.len() as i32,
                    conv_id
                )
                .execute(pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
        }
    }

    Ok((conversation_count, message_count))
}