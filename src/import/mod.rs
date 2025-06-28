use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::path::Path;
use tracing::{error, info, warn};

pub mod python_bridge;
pub mod parsers;

use crate::models::{Conversation, ImportStats, Message, ProviderType};

/// Import conversations from export files
pub async fn import_conversations(
    pool: &SqlitePool,
    provider: &str,
    path: &Path,
    use_python_bridge: bool,
) -> Result<usize> {
    let provider_type = ProviderType::from_str(provider);
    
    if provider_type == ProviderType::Unknown {
        return Err(anyhow::anyhow!("Unknown provider: {}", provider));
    }
    
    info!("Starting import for provider: {}", provider);
    
    // Log import event
    let event_id = log_import_start(pool, provider, path).await?;
    
    let mut stats = ImportStats::default();
    let start = std::time::Instant::now();
    
    let result = if use_python_bridge {
        // Use Python parsers via PyO3 bridge (temporary)
        python_bridge::import_with_python(pool, provider_type, path, &mut stats).await
    } else {
        // Use native Rust parsers
        match provider_type {
            ProviderType::ChatGPT => parsers::chatgpt::import(pool, path, &mut stats).await,
            ProviderType::Claude => parsers::claude::import(pool, path, &mut stats).await,
            ProviderType::Gemini => parsers::gemini::import(pool, path, &mut stats).await,
            ProviderType::XAI => parsers::xai::import(pool, path, &mut stats).await,
            ProviderType::Zed => parsers::zed::import(pool, path, &mut stats).await,
            _ => Err(anyhow::anyhow!("Native parser not implemented for {}", provider)),
        }
    };
    
    stats.duration_ms = start.elapsed().as_millis() as u64;
    
    // Log import completion
    log_import_complete(pool, event_id, &stats, result.as_ref().err()).await?;
    
    match result {
        Ok(_) => {
            info!(
                "Import completed: {} conversations, {} messages in {}ms",
                stats.conversations, stats.messages, stats.duration_ms
            );
            Ok(stats.conversations)
        }
        Err(e) => {
            error!("Import failed: {}", e);
            Err(e)
        }
    }
}

/// Process a single conversation batch
pub async fn process_conversation_batch(
    pool: &SqlitePool,
    conversations: Vec<(Conversation, Vec<Message>)>,
) -> Result<ImportStats> {
    let mut stats = ImportStats::default();
    
    // Start transaction for atomic import
    let mut tx = pool.begin().await?;
    
    for (conv, messages) in conversations {
        // Insert conversation
        let conv_id = sqlx::query!(
            r#"
            INSERT INTO conversations (
                provider, external_id, title, model, 
                created_at, updated_at, raw_json,
                system_prompt, temperature, max_tokens, user_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT(provider, external_id) DO UPDATE SET
                title = excluded.title,
                model = excluded.model,
                updated_at = excluded.updated_at,
                raw_json = excluded.raw_json,
                system_prompt = excluded.system_prompt,
                temperature = excluded.temperature,
                max_tokens = excluded.max_tokens,
                user_id = excluded.user_id
            RETURNING id
            "#,
            conv.provider,
            conv.external_id,
            conv.title,
            conv.model,
            conv.created_at,
            conv.updated_at,
            conv.raw_json.as_ref().map(|v| v.to_string()),
            conv.system_prompt,
            conv.temperature,
            conv.max_tokens,
            conv.user_id,
        )
        .fetch_one(&mut *tx)
        .await
        .context("Failed to insert conversation")?;
        
        stats.conversations += 1;
        
        // Insert messages in batches
        for message in messages {
            sqlx::query!(
                r#"
                INSERT INTO messages (
                    conversation_id, role, content, model,
                    created_at, tokens, finish_reason, 
                    tool_calls, attachments
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
                conv_id.id,
                message.role,
                message.content,
                message.model,
                message.created_at,
                message.tokens,
                message.finish_reason,
                message.tool_calls.as_ref().map(|v| v.to_string()),
                message.attachments.as_ref().map(|v| v.to_string()),
            )
            .execute(&mut *tx)
            .await
            .context("Failed to insert message")?;
            
            stats.messages += 1;
        }
    }
    
    // Commit transaction
    tx.commit().await?;
    
    Ok(stats)
}

/// Log import start event
async fn log_import_start(pool: &SqlitePool, provider: &str, path: &Path) -> Result<i64> {
    let result = sqlx::query!(
        r#"
        INSERT INTO import_events (event_type, provider, file_path, status)
        VALUES ('import_start', $1, $2, 'in_progress')
        RETURNING id
        "#,
        provider,
        path.to_string_lossy()
    )
    .fetch_one(pool)
    .await?;
    
    Ok(result.id)
}

/// Log import completion
async fn log_import_complete(
    pool: &SqlitePool,
    event_id: i64,
    stats: &ImportStats,
    error: Option<&anyhow::Error>,
) -> Result<()> {
    let status = if error.is_some() { "failed" } else { "completed" };
    let stats_json = serde_json::json!({
        "conversations": stats.conversations,
        "messages": stats.messages,
        "errors": stats.errors,
        "duration_ms": stats.duration_ms,
    });
    
    sqlx::query!(
        r#"
        UPDATE import_events 
        SET status = $1, stats = $2, error = $3
        WHERE id = $4
        "#,
        status,
        stats_json.to_string(),
        error.map(|e| e.to_string()),
        event_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

/// Detect provider from file content
pub fn detect_provider(content: &str) -> Option<ProviderType> {
    // Quick heuristics to detect format
    if content.contains("\"conversation_id\"") && content.contains("\"message\"") {
        return Some(ProviderType::ChatGPT);
    }
    
    if content.contains("\"uuid\"") && content.contains("\"chat_messages\"") {
        return Some(ProviderType::Claude);
    }
    
    if content.contains("\"conversations\"") && content.contains("\"gemini\"") {
        return Some(ProviderType::Gemini);
    }
    
    None
}