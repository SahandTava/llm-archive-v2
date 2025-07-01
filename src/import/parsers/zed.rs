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

/// Zed AI export format structures
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ZedExport {
    Single(ZedConversation),
    Multiple(Vec<ZedConversation>),
    Wrapped {
        #[serde(alias = "sessions", alias = "workspace_conversations")]
        conversations: Vec<ZedConversation>,
    },
}

#[derive(Debug, Deserialize)]
struct ZedConversation {
    #[serde(alias = "session_id")]
    id: Option<String>,
    title: Option<String>,
    file_path: Option<String>,
    #[serde(alias = "started_at")]
    created_at: Option<Value>,
    #[serde(alias = "ended_at")]
    updated_at: Option<Value>,
    model: Option<String>,
    workspace: Option<String>,
    language: Option<String>,
    #[serde(alias = "interactions")]
    messages: Option<Vec<ZedMessage>>,
}

#[derive(Debug, Deserialize)]
struct ZedMessage {
    id: Option<String>,
    #[serde(alias = "type")]
    role: Option<String>,
    #[serde(alias = "text")]
    content: Option<String>,
    code: Option<String>,
    language: Option<String>,
    context: Option<ZedContext>,
    #[serde(alias = "timestamp")]
    created_at: Option<Value>,
    diagnostics: Option<Value>,
    suggestions: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ZedContext {
    file: Option<String>,
    selection: Option<ZedSelection>,
}

#[derive(Debug, Deserialize)]
struct ZedSelection {
    start: i32,
    end: i32,
}

/// Import Zed conversations from export file
pub async fn import(pool: &SqlitePool, path: &Path, stats: &mut ImportStats) -> Result<()> {
    info!("Starting native Zed import from {:?}", path);
    
    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read Zed export file")?;
    
    let export: ZedExport = serde_json::from_str(&content)
        .context("Failed to parse Zed export JSON")?;
    
    let conversations = match export {
        ZedExport::Single(conv) => vec![conv],
        ZedExport::Multiple(convs) => convs,
        ZedExport::Wrapped { conversations } => conversations,
    };
    
    info!("Found {} conversations to import", conversations.len());
    
    let mut batch = Vec::new();
    
    for conv in conversations {
        match parse_conversation(&conv) {
            Ok((conversation, messages)) => {
                if messages.is_empty() {
                    debug!("Skipping conversation with no messages");
                    continue;
                }
                
                batch.push((conversation, messages));
                
                if batch.len() >= 100 {
                    let batch_to_process = std::mem::take(&mut batch);
                    let batch_stats = process_conversation_batch(pool, batch_to_process).await?;
                    stats.conversations += batch_stats.conversations;
                    stats.messages += batch_stats.messages;
                }
            }
            Err(e) => {
                warn!("Failed to parse conversation: {}", e);
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

/// Parse a Zed conversation into our domain model
fn parse_conversation(conv: &ZedConversation) -> Result<(Conversation, Vec<Message>)> {
    let created_at = conv.created_at
        .as_ref()
        .and_then(parse_timestamp)
        .unwrap_or_else(Utc::now);
    
    let updated_at = conv.updated_at
        .as_ref()
        .and_then(parse_timestamp)
        .unwrap_or(created_at);
    
    // Generate title from file path if not provided
    let title = conv.title.clone()
        .or_else(|| conv.file_path.clone())
        .unwrap_or_else(|| "Zed AI Session".to_string());
    
    let model = conv.model.clone()
        .unwrap_or_else(|| "zed-ai".to_string());
    
    // Store workspace info in raw_json along with other metadata
    let mut raw_json = serde_json::to_value(conv)?;
    if let Some(obj) = raw_json.as_object_mut() {
        if let Some(workspace) = &conv.workspace {
            obj.insert("workspace".to_string(), Value::String(workspace.clone()));
        }
        if let Some(language) = &conv.language {
            obj.insert("language".to_string(), Value::String(language.clone()));
        }
    }
    
    let conversation = Conversation {
        id: 0,
        provider: "zed".to_string(),
        external_id: conv.id.clone(),
        title: Some(title),
        model: Some(model),
        created_at,
        updated_at,
        raw_json: Some(raw_json),
        system_prompt: None,
        temperature: None,
        max_tokens: None,
        user_id: None,
    };
    
    // Parse messages
    let messages = conv.messages
        .as_ref()
        .map(|msgs| msgs.iter().filter_map(|msg| parse_message(msg, created_at)).collect())
        .unwrap_or_default();
    
    Ok((conversation, messages))
}

/// Parse a Zed message
fn parse_message(msg: &ZedMessage, default_time: DateTime<Utc>) -> Option<Message> {
    let role = msg.role.as_ref()?.to_lowercase();
    let role = match role.as_str() {
        "user" | "human" | "developer" => "user",
        "assistant" | "ai" | "zed" => "assistant",
        "system" => "system",
        _ => return None,
    };
    
    let mut content = msg.content.clone().unwrap_or_default();
    
    // Add code block if present
    if let Some(code) = &msg.code {
        let language = msg.language.as_deref().unwrap_or("text");
        content.push_str(&format!("\n\n```{}\n{}\n```", language, code));
    }
    
    // Add context information
    if let Some(ctx) = &msg.context {
        let mut context_str = String::new();
        if let Some(file) = &ctx.file {
            context_str.push_str(&format!("[File: {}]", file));
        }
        if let Some(sel) = &ctx.selection {
            context_str.push_str(&format!(" [Selection: lines {}-{}]", sel.start, sel.end));
        }
        if !context_str.is_empty() {
            content = format!("{}\n{}", context_str, content);
        }
    }
    
    let created_at = msg.created_at
        .as_ref()
        .and_then(parse_timestamp)
        .unwrap_or(default_time);
    
    // Store additional metadata in attachments
    let mut metadata = serde_json::Map::new();
    if let Some(lang) = &msg.language {
        metadata.insert("language".to_string(), Value::String(lang.clone()));
    }
    if let Some(diag) = &msg.diagnostics {
        metadata.insert("diagnostics".to_string(), diag.clone());
    }
    if let Some(sugg) = &msg.suggestions {
        metadata.insert("suggestions".to_string(), sugg.clone());
    }
    
    let attachments = if metadata.is_empty() {
        None
    } else {
        Some(Value::Object(metadata))
    };
    
    Some(Message {
        id: 0,
        conversation_id: 0,
        role: role.to_string(),
        content,
        model: None,
        created_at,
        tokens: None,
        finish_reason: None,
        tool_calls: None,
        attachments,
    })
}