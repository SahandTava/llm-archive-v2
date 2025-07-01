use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use sqlx::SqlitePool;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::models::{Conversation, ImportStats, Message, ProviderType};
use super::process_conversation_batch;

/// Import conversations using Python parsers via PyO3
pub async fn import_with_python(
    pool: &SqlitePool,
    provider_type: ProviderType,
    path: &Path,
    stats: &mut ImportStats,
) -> Result<()> {
    info!("Using Python bridge for {} import", provider_type.as_str());
    
    // Initialize Python runtime and import parsers
    Python::with_gil(|py| {
        // Add V1 parsers directory to Python path
        let sys = py.import("sys")?;
        let path_list: &PyList = sys.getattr("path")?.downcast()?;
        
        let parsers_path = "/home/bijan/LLMArchGH/llm-archive-v2/parsers";
        path_list.append(parsers_path)?;
        
        debug!("Added parsers to Python path: {}", parsers_path);
        
        // Import the appropriate parser module
        let parser_module = match provider_type {
            ProviderType::ChatGPT => py.import("chatgpt_parser")?,
            ProviderType::Claude => py.import("claude_parser")?,
            ProviderType::Gemini => py.import("gemini_parser")?,
            ProviderType::XAI => py.import("xai_parser")?,
            ProviderType::Zed => py.import("zed_parser")?,
            _ => return Err(anyhow::anyhow!("No Python parser for {}", provider_type.as_str())),
        };
        
        // Call the parse function
        let parse_fn = parser_module.getattr("parse_export")?;
        let file_path_str = path.to_string_lossy();
        let conversations_py = parse_fn.call1((file_path_str.as_ref(),))?;
        
        // Convert Python objects to Rust structs
        let conversations_list: &PyList = conversations_py.downcast()?;
        let mut batch = Vec::new();
        
        for conv_py in conversations_list {
            match parse_conversation(py, conv_py, provider_type.as_str()) {
                Ok((conv, messages)) => {
                    batch.push((conv, messages));
                    
                    // Process in batches of 100
                    if batch.len() >= 100 {
                        let batch_to_process = std::mem::take(&mut batch);
                        py.allow_threads(|| {
                            let rt = tokio::runtime::Handle::current();
                            rt.block_on(async {
                                match process_conversation_batch(pool, batch_to_process).await {
                                    Ok(batch_stats) => {
                                        stats.conversations += batch_stats.conversations;
                                        stats.messages += batch_stats.messages;
                                    }
                                    Err(e) => {
                                        warn!("Failed to process batch: {}", e);
                                        stats.errors += 1;
                                    }
                                }
                            });
                        });
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
            py.allow_threads(|| {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async {
                    match process_conversation_batch(pool, batch).await {
                        Ok(batch_stats) => {
                            stats.conversations += batch_stats.conversations;
                            stats.messages += batch_stats.messages;
                        }
                        Err(e) => {
                            warn!("Failed to process final batch: {}", e);
                            stats.errors += 1;
                        }
                    }
                });
            });
        }
        
        Ok::<(), anyhow::Error>(())
    })?;
    
    Ok(())
}

/// Parse a Python conversation object into Rust structs
fn parse_conversation(
    py: Python,
    conv_py: &PyAny,
    provider: &str,
) -> Result<(Conversation, Vec<Message>)> {
    let conv_dict: &PyDict = conv_py.downcast()
        .context("Expected conversation to be a dict")?;
    
    // Extract conversation fields
    let external_id = conv_dict
        .get_item("id")?
        .and_then(|v| v.extract::<String>().ok());
    
    let title = conv_dict
        .get_item("title")?
        .and_then(|v| v.extract::<String>().ok());
    
    let model = conv_dict
        .get_item("model")?
        .and_then(|v| v.extract::<String>().ok());
    
    let created_at = conv_dict
        .get_item("created_at")?
        .and_then(|v| v.extract::<String>().ok())
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    
    let updated_at = conv_dict
        .get_item("updated_at")?
        .and_then(|v| v.extract::<String>().ok())
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(created_at);
    
    // Extract metadata
    let system_prompt = conv_dict
        .get_item("system_prompt")?
        .and_then(|v| v.extract::<String>().ok());
    
    let temperature = conv_dict
        .get_item("temperature")?
        .and_then(|v| v.extract::<f32>().ok());
    
    let max_tokens = conv_dict
        .get_item("max_tokens")?
        .and_then(|v| v.extract::<i32>().ok());
    
    let user_id = conv_dict
        .get_item("user_id")?
        .and_then(|v| v.extract::<String>().ok());
    
    // Store raw JSON for future migrations
    let raw_json = pythonize::depythonize::<serde_json::Value>(conv_py).ok();
    
    let conversation = Conversation {
        id: 0, // Will be assigned by database
        provider: provider.to_string(),
        external_id,
        title,
        model,
        created_at,
        updated_at,
        raw_json,
        system_prompt,
        temperature,
        max_tokens,
        user_id,
    };
    
    // Parse messages
    let messages_py = conv_dict
        .get_item("messages")?
        .ok_or_else(|| anyhow::anyhow!("No messages in conversation"))?;
    
    let messages_list: &PyList = messages_py.downcast()
        .context("Expected messages to be a list")?;
    
    let mut messages = Vec::new();
    
    for msg_py in messages_list {
        match parse_message(py, msg_py) {
            Ok(msg) => messages.push(msg),
            Err(e) => debug!("Skipping message: {}", e),
        }
    }
    
    Ok((conversation, messages))
}

/// Parse a Python message object
fn parse_message(py: Python, msg_py: &PyAny) -> Result<Message> {
    let msg_dict: &PyDict = msg_py.downcast()
        .context("Expected message to be a dict")?;
    
    let role = msg_dict
        .get_item("role")?
        .ok_or_else(|| anyhow::anyhow!("No role in message"))?
        .extract::<String>()?;
    
    let content = msg_dict
        .get_item("content")?
        .ok_or_else(|| anyhow::anyhow!("No content in message"))?
        .extract::<String>()?;
    
    let model = msg_dict
        .get_item("model")?
        .and_then(|v| v.extract::<String>().ok());
    
    let created_at = msg_dict
        .get_item("created_at")?
        .and_then(|v| v.extract::<String>().ok())
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    
    let tokens = msg_dict
        .get_item("tokens")?
        .and_then(|v| v.extract::<i32>().ok());
    
    let finish_reason = msg_dict
        .get_item("finish_reason")?
        .and_then(|v| v.extract::<String>().ok());
    
    let tool_calls = msg_dict
        .get_item("tool_calls")?
        .and_then(|v| pythonize::depythonize::<serde_json::Value>(v).ok());
    
    let attachments = msg_dict
        .get_item("attachments")?
        .and_then(|v| pythonize::depythonize::<serde_json::Value>(v).ok());
    
    Ok(Message {
        id: 0, // Will be assigned by database
        conversation_id: 0, // Will be set during insert
        role,
        content,
        model,
        created_at,
        tokens,
        finish_reason,
        tool_calls,
        attachments,
    })
}

/// Test Python bridge connectivity
pub fn test_python_bridge() -> Result<()> {
    Python::with_gil(|py| {
        let version = py.version();
        info!("Python bridge initialized: Python {}", version);
        
        // Try to import parsers
        let sys = py.import("sys")?;
        let path_list: &PyList = sys.getattr("path")?.downcast()?;
        path_list.append("/home/bijan/LLMArchGH/llm-archive-v2/parsers")?;
        
        // Test importing each parser
        for parser in ["chatgpt_parser", "claude_parser", "gemini_parser", "xai_parser", "zed_parser"] {
            match py.import(parser) {
                Ok(_) => debug!("Successfully imported {}", parser),
                Err(e) => warn!("Failed to import {}: {}", parser, e),
            }
        }
        
        Ok::<(), anyhow::Error>(())
    })?;
    
    Ok(())
}