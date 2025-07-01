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

/// Gemini export format structures
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GeminiExport {
    Single(GeminiConversation),
    Multiple(Vec<GeminiConversation>),
    Wrapped {
        conversations: Vec<GeminiConversation>,
    },
}

#[derive(Debug, Deserialize)]
struct GeminiConversation {
    #[serde(alias = "conversation_id")]
    id: Option<String>,
    #[serde(alias = "name")]
    title: Option<String>,
    created_at: Option<Value>,
    updated_at: Option<Value>,
    model: Option<String>,
    messages: Option<Vec<GeminiMessage>>,
    turns: Option<Vec<GeminiMessage>>, // Alternative field name
    settings: Option<GeminiSettings>,
}

#[derive(Debug, Deserialize)]
struct GeminiMessage {
    id: Option<String>,
    #[serde(alias = "author")]
    role: Option<String>,
    #[serde(alias = "text")]
    content: Option<String>,
    parts: Option<Vec<GeminiPart>>,
    created_at: Option<Value>,
    safety_ratings: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text(String),
    Object {
        text: Option<String>,
        inline_data: Option<InlineData>,
    },
}

#[derive(Debug, Deserialize)]
struct InlineData {
    mime_type: String,
    data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiSettings {
    temperature: Option<f32>,
    max_output_tokens: Option<i32>,
    system_instruction: Option<String>,
}

/// Import Gemini conversations from export file
pub async fn import(pool: &SqlitePool, path: &Path, stats: &mut ImportStats) -> Result<()> {
    info!("Starting native Gemini import from {:?}", path);
    
    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read Gemini export file")?;
    
    let export: GeminiExport = serde_json::from_str(&content)
        .context("Failed to parse Gemini export JSON")?;
    
    let conversations = match export {
        GeminiExport::Single(conv) => vec![conv],
        GeminiExport::Multiple(convs) => convs,
        GeminiExport::Wrapped { conversations } => conversations,
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

/// Parse a Gemini conversation into our domain model
fn parse_conversation(conv: &GeminiConversation) -> Result<(Conversation, Vec<Message>)> {
    let created_at = conv.created_at
        .as_ref()
        .and_then(parse_timestamp)
        .unwrap_or_else(Utc::now);
    
    let updated_at = conv.updated_at
        .as_ref()
        .and_then(parse_timestamp)
        .unwrap_or(created_at);
    
    let model = conv.model.clone()
        .unwrap_or_else(|| "gemini-pro".to_string());
    
    // Extract settings
    let (system_prompt, temperature, max_tokens) = if let Some(settings) = &conv.settings {
        (
            settings.system_instruction.clone(),
            settings.temperature,
            settings.max_output_tokens,
        )
    } else {
        (None, None, None)
    };
    
    let conversation = Conversation {
        id: 0,
        provider: "gemini".to_string(),
        external_id: conv.id.clone(),
        title: conv.title.clone(),
        model: Some(model),
        created_at,
        updated_at,
        raw_json: Some(serde_json::to_value(conv)?),
        system_prompt,
        temperature,
        max_tokens,
        user_id: None,
    };
    
    // Parse messages
    let messages_data = conv.messages.as_ref()
        .or(conv.turns.as_ref())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    
    let messages = messages_data
        .iter()
        .filter_map(|msg| parse_message(msg, created_at))
        .collect();
    
    Ok((conversation, messages))
}

/// Parse a Gemini message
fn parse_message(msg: &GeminiMessage, default_time: DateTime<Utc>) -> Option<Message> {
    let role = msg.role.as_ref()?.to_lowercase();
    let role = match role.as_str() {
        "user" | "human" => "user",
        "model" | "assistant" | "gemini" => "assistant",
        "system" => "system",
        _ => return None,
    };
    
    // Extract content from parts or direct content
    let content = if let Some(parts) = &msg.parts {
        let text_parts: Vec<String> = parts.iter()
            .filter_map(|part| match part {
                GeminiPart::Text(s) => Some(s.clone()),
                GeminiPart::Object { text, inline_data } => {
                    if let Some(t) = text {
                        Some(t.clone())
                    } else if let Some(data) = inline_data {
                        Some(format!("[Attached: {}]", data.mime_type))
                    } else {
                        None
                    }
                }
            })
            .collect();
        
        text_parts.join("\n")
    } else {
        msg.content.clone()?
    };
    
    let created_at = msg.created_at
        .as_ref()
        .and_then(parse_timestamp)
        .unwrap_or(default_time);
    
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
        attachments: None,
    })
}