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

/// XAI/Grok export format structures
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum XAIExport {
    Direct(XAIConversation),
    WrappedList {
        conversations: Vec<XAIConversation>,
    },
    WrappedData {
        data: XAIData,
    },
    List(Vec<XAIConversation>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum XAIData {
    Single(XAIConversation),
    Multiple(Vec<XAIConversation>),
    Wrapped {
        conversations: Vec<XAIConversation>,
    },
}

#[derive(Debug, Deserialize)]
struct XAIConversation {
    #[serde(alias = "thread_id", alias = "conversation_id")]
    id: Option<String>,
    #[serde(alias = "subject")]
    title: Option<String>,
    #[serde(alias = "timestamp")]
    created_at: Option<Value>,
    #[serde(alias = "last_updated")]
    updated_at: Option<Value>,
    model: Option<String>,
    user: Option<Value>,
    #[serde(alias = "exchanges", alias = "turns")]
    messages: Option<Vec<XAIMessage>>,
    settings: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct XAIMessage {
    #[serde(alias = "message_id")]
    id: Option<String>,
    #[serde(alias = "sender", alias = "type")]
    role: Option<String>,
    #[serde(alias = "text", alias = "message")]
    content: Option<String>,
    #[serde(alias = "timestamp")]
    created_at: Option<Value>,
    model: Option<String>,
    engine: Option<String>,
    token_count: Option<i32>,
    tokens: Option<i32>,
    attachments: Option<Value>,
    references: Option<Value>,
}

/// Import XAI/Grok conversations from export file
pub async fn import(pool: &SqlitePool, path: &Path, stats: &mut ImportStats) -> Result<()> {
    info!("Starting native XAI/Grok import from {:?}", path);
    
    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read XAI export file")?;
    
    let export: XAIExport = serde_json::from_str(&content)
        .context("Failed to parse XAI export JSON")?;
    
    let conversations = match export {
        XAIExport::Direct(conv) => vec![conv],
        XAIExport::List(convs) => convs,
        XAIExport::WrappedList { conversations } => conversations,
        XAIExport::WrappedData { data } => match data {
            XAIData::Single(conv) => vec![conv],
            XAIData::Multiple(convs) => convs,
            XAIData::Wrapped { conversations } => conversations,
        },
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

/// Parse an XAI conversation into our domain model
fn parse_conversation(conv: &XAIConversation) -> Result<(Conversation, Vec<Message>)> {
    let created_at = conv.created_at
        .as_ref()
        .and_then(parse_timestamp)
        .unwrap_or_else(Utc::now);
    
    let updated_at = conv.updated_at
        .as_ref()
        .and_then(parse_timestamp)
        .unwrap_or(created_at);
    
    let model = conv.model.clone()
        .unwrap_or_else(|| "grok-1".to_string());
    
    // Extract user ID
    let user_id = conv.user.as_ref().and_then(|u| match u {
        Value::String(s) => Some(s.clone()),
        Value::Object(obj) => obj.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        _ => None,
    });
    
    let conversation = Conversation {
        id: 0,
        provider: "xai".to_string(),
        external_id: conv.id.clone(),
        title: conv.title.clone(),
        model: Some(model),
        created_at,
        updated_at,
        raw_json: Some(serde_json::to_value(conv)?),
        system_prompt: None,
        temperature: None,
        max_tokens: None,
        user_id,
    };
    
    // Parse messages
    let messages = conv.messages
        .as_ref()
        .map(|msgs| msgs.iter().filter_map(|msg| parse_message(msg, created_at)).collect())
        .unwrap_or_default();
    
    Ok((conversation, messages))
}

/// Parse an XAI message
fn parse_message(msg: &XAIMessage, default_time: DateTime<Utc>) -> Option<Message> {
    let role = msg.role.as_ref()?.to_lowercase();
    let role = match role.as_str() {
        "user" | "human" | "question" => "user",
        "grok" | "assistant" | "ai" | "model" | "answer" => "assistant",
        "system" => "system",
        _ => return None,
    };
    
    let content = msg.content.clone()?;
    
    let created_at = msg.created_at
        .as_ref()
        .and_then(parse_timestamp)
        .unwrap_or(default_time);
    
    let model = msg.model.clone().or_else(|| msg.engine.clone());
    
    let tokens = msg.token_count.or(msg.tokens);
    
    // Handle attachments/references
    let attachments = msg.attachments.as_ref()
        .or(msg.references.as_ref())
        .cloned();
    
    Some(Message {
        id: 0,
        conversation_id: 0,
        role: role.to_string(),
        content,
        model,
        created_at,
        tokens,
        finish_reason: None,
        tool_calls: None,
        attachments,
    })
}