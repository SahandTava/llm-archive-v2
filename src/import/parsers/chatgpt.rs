use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::models::{Conversation, ImportStats, Message};
use crate::import::process_conversation_batch;
use super::{get_f32, get_i32, get_string, parse_timestamp};

/// ChatGPT export format structures
#[derive(Debug, Deserialize)]
struct ChatGPTExport {
    conversations: Vec<ChatGPTConversation>,
}

#[derive(Debug, Deserialize)]
struct ChatGPTConversation {
    id: String,
    title: String,
    create_time: Option<f64>,
    update_time: Option<f64>,
    mapping: HashMap<String, ChatGPTNode>,
    conversation_id: Option<String>,
    #[serde(default)]
    conversation_template_id: Option<String>,
    #[serde(default)]
    gizmo_id: Option<String>,
    #[serde(default)]
    is_archived: bool,
}

#[derive(Debug, Deserialize)]
struct ChatGPTNode {
    id: String,
    message: Option<ChatGPTMessage>,
    parent: Option<String>,
    children: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ChatGPTMessage {
    id: String,
    author: ChatGPTAuthor,
    content: ChatGPTContent,
    status: Option<String>,
    end_turn: Option<bool>,
    weight: Option<f32>,
    metadata: Option<ChatGPTMetadata>,
    recipient: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatGPTAuthor {
    role: String,
    name: Option<String>,
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ChatGPTContent {
    content_type: String,
    parts: Option<Vec<Value>>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatGPTMetadata {
    model_slug: Option<String>,
    finish_details: Option<ChatGPTFinishDetails>,
    #[serde(default)]
    timestamp_: Option<String>,
    #[serde(default)]
    message_type: Option<String>,
    #[serde(default)]
    is_complete: Option<bool>,
    #[serde(default)]
    citations: Option<Vec<Value>>,
    #[serde(default)]
    content_references: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
struct ChatGPTFinishDetails {
    #[serde(rename = "type")]
    finish_type: Option<String>,
    stop_tokens: Option<Vec<i32>>,
}

/// Import ChatGPT conversations from export file
pub async fn import(pool: &SqlitePool, path: &Path, stats: &mut ImportStats) -> Result<()> {
    info!("Starting native ChatGPT import from {:?}", path);
    
    // Read and parse JSON file
    let content = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read ChatGPT export file")?;
    
    let export: ChatGPTExport = serde_json::from_str(&content)
        .context("Failed to parse ChatGPT export JSON")?;
    
    info!("Found {} conversations to import", export.conversations.len());
    
    // Process conversations in batches
    let mut batch = Vec::new();
    
    for conv in export.conversations {
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

/// Parse a ChatGPT conversation into our domain model
fn parse_conversation(conv: &ChatGPTConversation) -> Result<(Conversation, Vec<Message>)> {
    let created_at = conv.create_time
        .and_then(|ts| chrono::DateTime::from_timestamp(ts as i64, 0))
        .unwrap_or_else(Utc::now);
    
    let updated_at = conv.update_time
        .and_then(|ts| chrono::DateTime::from_timestamp(ts as i64, 0))
        .unwrap_or(created_at);
    
    // Extract messages from the mapping
    let messages = extract_messages(&conv.mapping)?;
    
    // Determine model from messages
    let model = messages.iter()
        .find_map(|m| m.model.clone())
        .or_else(|| Some("gpt-3.5-turbo".to_string())); // Default model
    
    // Create conversation
    let conversation = Conversation {
        id: 0,
        provider: "chatgpt".to_string(),
        external_id: Some(conv.id.clone()),
        title: Some(conv.title.clone()),
        model,
        created_at,
        updated_at,
        raw_json: Some(serde_json::to_value(conv)?),
        system_prompt: None, // ChatGPT doesn't expose this in exports
        temperature: None,
        max_tokens: None,
        user_id: None,
    };
    
    Ok((conversation, messages))
}

/// Extract messages from ChatGPT's node mapping
fn extract_messages(mapping: &HashMap<String, ChatGPTNode>) -> Result<Vec<Message>> {
    let mut messages = Vec::new();
    let mut processed = std::collections::HashSet::new();
    
    // Find root node(s)
    let roots: Vec<_> = mapping.iter()
        .filter(|(_, node)| node.parent.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    
    // Traverse from each root
    for root_id in roots {
        traverse_messages(&root_id, mapping, &mut messages, &mut processed);
    }
    
    // Sort messages by their order in the conversation
    // Since we traverse in order, they should already be sorted
    
    Ok(messages)
}

/// Recursively traverse the message tree
fn traverse_messages(
    node_id: &str,
    mapping: &HashMap<String, ChatGPTNode>,
    messages: &mut Vec<Message>,
    processed: &mut std::collections::HashSet<String>,
) {
    if processed.contains(node_id) {
        return;
    }
    
    processed.insert(node_id.to_string());
    
    if let Some(node) = mapping.get(node_id) {
        // Process this node's message
        if let Some(msg) = &node.message {
            if let Some(parsed) = parse_message(msg) {
                messages.push(parsed);
            }
        }
        
        // Process children
        for child_id in &node.children {
            traverse_messages(child_id, mapping, messages, processed);
        }
    }
}

/// Parse a ChatGPT message into our domain model
fn parse_message(msg: &ChatGPTMessage) -> Option<Message> {
    let role = match msg.author.role.as_str() {
        "user" => "user",
        "assistant" => "assistant",
        "system" => "system",
        "tool" => "tool",
        _ => return None, // Skip unknown roles
    };
    
    // Extract content based on content type
    let content = match msg.content.content_type.as_str() {
        "text" => {
            // Try text field first, then parts
            msg.content.text.clone().or_else(|| {
                msg.content.parts.as_ref().and_then(|parts| {
                    parts.iter()
                        .filter_map(|p| p.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                        .join("\n")
                        .into()
                })
            })?
        }
        "code" => {
            // Handle code content
            msg.content.text.clone().or_else(|| {
                msg.content.parts.as_ref().and_then(|parts| {
                    parts.iter()
                        .filter_map(|p| p.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                        .join("\n")
                        .into()
                })
            })?
        }
        _ => return None, // Skip other content types for now
    };
    
    // Extract model from metadata
    let model = msg.metadata.as_ref()
        .and_then(|m| m.model_slug.clone())
        .map(|slug| normalize_model_name(&slug));
    
    // Extract finish reason
    let finish_reason = msg.metadata.as_ref()
        .and_then(|m| m.finish_details.as_ref())
        .and_then(|f| f.finish_type.clone());
    
    // Create timestamp (ChatGPT doesn't provide per-message timestamps)
    let created_at = Utc::now();
    
    Some(Message {
        id: 0,
        conversation_id: 0,
        role: role.to_string(),
        content,
        model,
        created_at,
        tokens: None, // ChatGPT doesn't export token counts
        finish_reason,
        tool_calls: None, // TODO: Extract from content if needed
        attachments: None, // TODO: Extract if present
    })
}

/// Normalize ChatGPT model names to standard format
fn normalize_model_name(slug: &str) -> String {
    match slug {
        "gpt-4" => "gpt-4".to_string(),
        "gpt-4-gizmo" => "gpt-4".to_string(),
        "gpt-4-browsing" => "gpt-4-browsing".to_string(),
        "gpt-4-plugins" => "gpt-4-plugins".to_string(),
        "gpt-4-mobile" => "gpt-4".to_string(),
        "gpt-4o" => "gpt-4o".to_string(),
        "gpt-4o-mini" => "gpt-4o-mini".to_string(),
        "text-davinci-002-render-sha" => "gpt-3.5-turbo".to_string(),
        "text-davinci-002-render-paid" => "gpt-3.5-turbo".to_string(),
        _ => slug.to_string(),
    }
}