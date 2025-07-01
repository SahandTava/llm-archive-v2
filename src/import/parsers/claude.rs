use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::models::{Conversation, ImportStats, Message};
use crate::import::process_conversation_batch;
use super::{get_f32, get_i32, get_string, parse_timestamp};

/// Claude export format structures
#[derive(Debug, Deserialize)]
struct ClaudeExport {
    #[serde(rename = "uuid")]
    id: String,
    name: String,
    #[serde(rename = "created_at")]
    created_at: String,
    #[serde(rename = "updated_at")]
    updated_at: Option<String>,
    #[serde(rename = "chat_messages")]
    messages: Vec<ClaudeMessage>,
    #[serde(default)]
    project_uuid: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    settings: Option<ClaudeSettings>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessage {
    #[serde(rename = "uuid")]
    id: String,
    text: String,
    sender: String, // "human" or "assistant"
    #[serde(rename = "created_at")]
    created_at: String,
    #[serde(rename = "updated_at")]
    updated_at: Option<String>,
    #[serde(default)]
    files: Option<Vec<ClaudeFile>>,
    #[serde(default)]
    edited: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ClaudeFile {
    #[serde(rename = "file_name")]
    file_name: String,
    #[serde(rename = "file_type")]
    file_type: String,
    #[serde(rename = "file_size")]
    file_size: Option<i64>,
    #[serde(rename = "extracted_content")]
    extracted_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeSettings {
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    max_tokens: Option<i32>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    system_prompt: Option<String>,
}

/// Import Claude conversations from export file(s)
pub async fn import(pool: &SqlitePool, path: &Path, stats: &mut ImportStats) -> Result<()> {
    info!("Starting native Claude import from {:?}", path);
    
    // Claude exports can be single file or directory of files
    let conversations = if path.is_file() {
        vec![import_single_file(path).await?]
    } else if path.is_dir() {
        import_directory(path).await?
    } else {
        return Err(anyhow::anyhow!("Path is neither file nor directory"));
    };
    
    info!("Found {} conversations to import", conversations.len());
    
    // Process conversations in batches
    let mut batch = Vec::new();
    
    for conv in conversations {
        match parse_conversation(&conv) {
            Ok((conversation, messages)) => {
                if messages.is_empty() {
                    debug!("Skipping conversation {} with no messages", conv.id);
                    continue;
                }
                
                batch.push((conversation, messages));
                
                // Process batch when it reaches 100 conversations
                if batch.len() >= 100 {
                    let batch_to_process = std::mem::take(&mut batch);
                    let batch_stats = process_conversation_batch(pool, batch_to_process).await?;
                    stats.conversations += batch_stats.conversations;
                    stats.messages += batch_stats.messages;
                    
                    debug!("Processed batch: {} conversations, {} messages", 
                           batch_stats.conversations, batch_stats.messages);
                }
            }
            Err(e) => {
                warn!("Failed to parse conversation {}: {}", conv.id, e);
                stats.errors += 1;
            }
        }
    }
    
    // Process remaining conversations
    if !batch.is_empty() {
        let batch_stats = process_conversation_batch(pool, batch).await?;
        stats.conversations += batch_stats.conversations;
        stats.messages += batch_stats.messages;
    }
    
    Ok(())
}

/// Import single Claude export file
async fn import_single_file(path: &Path) -> Result<ClaudeExport> {
    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read Claude export file")?;
    
    serde_json::from_str(&content)
        .context("Failed to parse Claude export JSON")
}

/// Import all Claude export files from a directory
async fn import_directory(dir: &Path) -> Result<Vec<ClaudeExport>> {
    let mut conversations = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        
        // Only process JSON files
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match import_single_file(&path).await {
                Ok(conv) => conversations.push(conv),
                Err(e) => warn!("Failed to import {:?}: {}", path, e),
            }
        }
    }
    
    Ok(conversations)
}

/// Parse a Claude conversation into our domain model
fn parse_conversation(conv: &ClaudeExport) -> Result<(Conversation, Vec<Message>)> {
    let created_at = DateTime::parse_from_rfc3339(&conv.created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    
    let updated_at = conv.updated_at.as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(created_at);
    
    // Extract model and settings
    let model = conv.model.clone()
        .or_else(|| conv.settings.as_ref().and_then(|s| s.model.clone()))
        .map(|m| normalize_model_name(&m));
    
    let system_prompt = conv.settings.as_ref()
        .and_then(|s| s.system_prompt.clone());
    
    let temperature = conv.settings.as_ref()
        .and_then(|s| s.temperature);
    
    let max_tokens = conv.settings.as_ref()
        .and_then(|s| s.max_tokens);
    
    // Parse messages
    let messages = conv.messages.iter()
        .filter_map(|msg| parse_message(msg))
        .collect();
    
    // Create conversation
    let conversation = Conversation {
        id: 0,
        provider: "claude".to_string(),
        external_id: Some(conv.id.clone()),
        title: Some(conv.name.clone()),
        model,
        created_at,
        updated_at,
        raw_json: Some(serde_json::to_value(conv)?),
        system_prompt,
        temperature,
        max_tokens,
        user_id: conv.project_uuid.clone(),
    };
    
    Ok((conversation, messages))
}

/// Parse a Claude message into our domain model
fn parse_message(msg: &ClaudeMessage) -> Option<Message> {
    let role = match msg.sender.as_str() {
        "human" => "user",
        "assistant" => "assistant",
        _ => return None, // Skip unknown roles
    };
    
    let created_at = DateTime::parse_from_rfc3339(&msg.created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    
    // Handle attachments
    let attachments = msg.files.as_ref().map(|files| {
        serde_json::to_value(files.iter().map(|f| {
            serde_json::json!({
                "file_name": f.file_name,
                "file_type": f.file_type,
                "file_size": f.file_size,
                "has_content": f.extracted_content.is_some(),
            })
        }).collect::<Vec<_>>()).ok()
    }).flatten();
    
    // Combine text with file contents if present
    let mut content = msg.text.clone();
    if let Some(files) = &msg.files {
        for file in files {
            if let Some(extracted) = &file.extracted_content {
                content.push_str(&format!("\n\n[File: {}]\n{}", file.file_name, extracted));
            }
        }
    }
    
    Some(Message {
        id: 0,
        conversation_id: 0,
        role: role.to_string(),
        content,
        model: None, // Claude doesn't specify per-message model
        created_at,
        tokens: None, // Claude doesn't export token counts
        finish_reason: None,
        tool_calls: None,
        attachments,
    })
}

/// Normalize Claude model names to standard format
fn normalize_model_name(name: &str) -> String {
    match name {
        "claude-3-opus" => "claude-3-opus".to_string(),
        "claude-3-sonnet" => "claude-3-sonnet".to_string(),
        "claude-3-haiku" => "claude-3-haiku".to_string(),
        "claude-3.5-sonnet" => "claude-3.5-sonnet".to_string(),
        "claude-2.1" => "claude-2.1".to_string(),
        "claude-2" => "claude-2".to_string(),
        "claude-instant-1.2" => "claude-instant-1.2".to_string(),
        _ => name.to_string(),
    }
}