use anyhow::{Context, Result};
use sqlx::SqlitePool;
use tracing::{debug, info};

use crate::models::{Conversation, Message, SearchResult};

/// Search conversations using FTS5
pub async fn search_conversations(
    pool: &SqlitePool,
    query: &str,
    limit: usize,
) -> Result<Vec<Conversation>> {
    info!("Searching for: '{}' (limit: {})", query, limit);
    
    // Use FTS5 for full-text search
    let results = sqlx::query_as!(
        Conversation,
        r#"
        SELECT DISTINCT 
            c.id as "id!",
            c.provider as "provider!",
            c.external_id,
            c.title,
            c.model,
            c.created_at as "created_at!",
            c.updated_at as "updated_at!",
            c.raw_json,
            c.system_prompt,
            c.temperature,
            c.max_tokens,
            c.user_id
        FROM conversations c
        JOIN messages m ON c.id = m.conversation_id
        JOIN messages_fts ON m.id = messages_fts.rowid
        WHERE messages_fts MATCH $1
        ORDER BY rank
        LIMIT $2
        "#,
        query,
        limit as i64
    )
    .fetch_all(pool)
    .await
    .context("Failed to search conversations")?;
    
    debug!("Found {} conversations matching '{}'", results.len(), query);
    
    Ok(results)
}

/// Search with snippets and ranking
pub async fn search_with_snippets(
    pool: &SqlitePool,
    query: &str,
    limit: usize,
    snippet_length: usize,
) -> Result<Vec<SearchResult>> {
    info!("Searching with snippets for: '{}' (limit: {})", query, limit);
    
    let results = sqlx::query!(
        r#"
        SELECT 
            c.id as conversation_id,
            c.provider,
            c.external_id,
            c.title,
            c.model,
            c.created_at,
            c.updated_at,
            c.raw_json,
            c.system_prompt,
            c.temperature,
            c.max_tokens,
            c.user_id,
            snippet(messages_fts, 0, '[', ']', '...', $3) as snippet,
            rank
        FROM conversations c
        JOIN messages m ON c.id = m.conversation_id
        JOIN messages_fts ON m.id = messages_fts.rowid
        WHERE messages_fts MATCH $1
        ORDER BY rank
        LIMIT $2
        "#,
        query,
        limit as i64,
        snippet_length as i64 / 10 // Approximate token count
    )
    .fetch_all(pool)
    .await
    .context("Failed to search with snippets")?;
    
    let search_results: Vec<SearchResult> = results
        .into_iter()
        .map(|row| {
            let conversation = Conversation {
                id: row.conversation_id,
                provider: row.provider,
                external_id: row.external_id,
                title: row.title,
                model: row.model,
                created_at: row.created_at,
                updated_at: row.updated_at,
                raw_json: row.raw_json.and_then(|s| serde_json::from_str(&s).ok()),
                system_prompt: row.system_prompt,
                temperature: row.temperature,
                max_tokens: row.max_tokens,
                user_id: row.user_id,
            };
            
            SearchResult {
                conversation,
                snippet: row.snippet,
                rank: row.rank,
            }
        })
        .collect();
    
    debug!("Found {} results with snippets for '{}'", search_results.len(), query);
    
    Ok(search_results)
}

/// Advanced search with filters
pub async fn advanced_search(
    pool: &SqlitePool,
    query: &str,
    provider: Option<&str>,
    model: Option<&str>,
    user_id: Option<&str>,
    date_from: Option<chrono::DateTime<chrono::Utc>>,
    date_to: Option<chrono::DateTime<chrono::Utc>>,
    limit: usize,
) -> Result<Vec<Conversation>> {
    let mut sql = String::from(
        r#"
        SELECT DISTINCT 
            c.id,
            c.provider,
            c.external_id,
            c.title,
            c.model,
            c.created_at,
            c.updated_at,
            c.raw_json,
            c.system_prompt,
            c.temperature,
            c.max_tokens,
            c.user_id
        FROM conversations c
        JOIN messages m ON c.id = m.conversation_id
        JOIN messages_fts ON m.id = messages_fts.rowid
        WHERE messages_fts MATCH ?
        "#
    );
    
    let mut params: Vec<String> = vec![query.to_string()];
    let mut param_count = 1;
    
    // Add filters
    if let Some(p) = provider {
        param_count += 1;
        sql.push_str(&format!(" AND c.provider = ?{}", param_count));
        params.push(p.to_string());
    }
    
    if let Some(m) = model {
        param_count += 1;
        sql.push_str(&format!(" AND c.model = ?{}", param_count));
        params.push(m.to_string());
    }
    
    if let Some(u) = user_id {
        param_count += 1;
        sql.push_str(&format!(" AND c.user_id = ?{}", param_count));
        params.push(u.to_string());
    }
    
    if let Some(from) = date_from {
        param_count += 1;
        sql.push_str(&format!(" AND c.created_at >= ?{}", param_count));
        params.push(from.to_rfc3339());
    }
    
    if let Some(to) = date_to {
        param_count += 1;
        sql.push_str(&format!(" AND c.created_at <= ?{}", param_count));
        params.push(to.to_rfc3339());
    }
    
    sql.push_str(&format!(" ORDER BY rank LIMIT {}", limit));
    
    // Execute dynamic query
    let mut query = sqlx::query_as::<_, Conversation>(&sql);
    for param in params {
        query = query.bind(param);
    }
    
    let results = query
        .fetch_all(pool)
        .await
        .context("Failed to execute advanced search")?;
    
    Ok(results)
}

/// Get conversation messages for display
pub async fn get_conversation_messages(
    pool: &SqlitePool,
    conversation_id: i64,
) -> Result<Vec<Message>> {
    let messages = sqlx::query_as!(
        Message,
        r#"
        SELECT 
            id as "id!",
            conversation_id as "conversation_id!",
            role as "role!",
            content as "content!",
            model,
            created_at as "created_at!",
            tokens,
            finish_reason,
            tool_calls,
            attachments
        FROM messages
        WHERE conversation_id = $1
        ORDER BY created_at ASC
        "#,
        conversation_id
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch conversation messages")?;
    
    Ok(messages)
}

/// Get search suggestions based on existing data
pub async fn get_search_suggestions(
    pool: &SqlitePool,
    prefix: &str,
    limit: usize,
) -> Result<Vec<String>> {
    // This is a simple implementation - could be enhanced with:
    // - Frequent search terms tracking
    // - Model name suggestions
    // - Smart completions
    
    let suggestions = sqlx::query!(
        r#"
        SELECT DISTINCT title
        FROM conversations
        WHERE title LIKE $1 || '%'
        AND title IS NOT NULL
        ORDER BY created_at DESC
        LIMIT $2
        "#,
        prefix,
        limit as i64
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .filter_map(|row| row.title)
    .collect();
    
    Ok(suggestions)
}