// parsers/chatgpt.rs - ChatGPT/OpenAI export parser

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use super::{
    common::*, ChatProvider, Conversation, ImportError, ImportStats, ImportWarning, MediaFile,
    Message, MessageRole, ParserError, ParserResult,
};

/// ChatGPT provider implementation
pub struct ChatGPTProvider;

impl ChatGPTProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ChatProvider for ChatGPTProvider {
    fn name(&self) -> &'static str {
        "ChatGPT"
    }

    async fn find_files(&self, dir: &Path) -> ParserResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        // Look for conversations.json first
        let conversations_json = dir.join("conversations.json");
        if conversations_json.exists() && self.can_handle(&conversations_json).await {
            files.push(conversations_json);
            return Ok(files); // Prefer bulk export if found
        }

        // Otherwise look for individual JSON files
        let entries = tokio::fs::read_dir(dir).await?;
        let mut entries = tokio_stream::wrappers::ReadDirStream::new(entries);
        
        use tokio_stream::StreamExt;
        while let Some(entry) = entries.next().await {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Skip obvious non-chat files
                let filename = path.file_name().unwrap().to_string_lossy();
                if !filename.contains("feedback") && !filename.contains("user") {
                    if self.can_handle(&path).await {
                        files.push(path);
                    }
                }
            }
        }

        Ok(files)
    }

    async fn can_handle(&self, file: &Path) -> bool {
        if !is_valid_json_file(file) {
            return false;
        }

        // Quick check of file structure
        match tokio::fs::read_to_string(file).await {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(Value::Array(arr)) => {
                    // Check first item for ChatGPT structure
                    arr.first()
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            obj.contains_key("mapping") || 
                            obj.contains_key("conversation_id") ||
                            obj.contains_key("title")
                        })
                        .unwrap_or(false)
                }
                Ok(Value::Object(obj)) => {
                    obj.contains_key("mapping") || 
                    obj.contains_key("messages") ||
                    (obj.contains_key("title") && obj.contains_key("create_time"))
                }
                _ => false,
            },
            Err(_) => false,
        }
    }

    async fn extract_conversations(
        &self,
        file: &Path,
        stats: &mut ImportStats,
    ) -> ParserResult<Vec<Conversation>> {
        let content = tokio::fs::read_to_string(file).await?;
        let data: Value = serde_json::from_str(&content)?;
        
        let mut conversations = Vec::new();
        let file_str = file.to_string_lossy().to_string();

        match data {
            Value::Array(arr) => {
                // conversations.json format
                for (idx, item) in arr.iter().enumerate() {
                    match self.extract_single_conversation(item, file, idx).await {
                        Ok(Some(conv)) => {
                            stats.total_messages += conv.messages.len();
                            conversations.push(conv);
                        }
                        Ok(None) => {
                            stats.warnings.push(ImportWarning {
                                file: file_str.clone(),
                                warning: format!("Empty conversation at index {}", idx),
                                context: None,
                            });
                        }
                        Err(e) => {
                            stats.errors.push(ImportError {
                                file: file_str.clone(),
                                error: e.to_string(),
                                context: Some(format!("Conversation index {}", idx)),
                            });
                        }
                    }
                }
            }
            Value::Object(_) => {
                // Single conversation file
                match self.extract_single_conversation(&data, file, 0).await {
                    Ok(Some(conv)) => {
                        stats.total_messages += conv.messages.len();
                        conversations.push(conv);
                    }
                    Ok(None) => {
                        stats.warnings.push(ImportWarning {
                            file: file_str.clone(),
                            warning: "Empty conversation".to_string(),
                            context: None,
                        });
                    }
                    Err(e) => {
                        stats.errors.push(ImportError {
                            file: file_str.clone(),
                            error: e.to_string(),
                            context: None,
                        });
                    }
                }
            }
            _ => {
                return Err(ParserError::InvalidFormat {
                    provider: "ChatGPT".to_string(),
                    reason: "Root must be array or object".to_string(),
                });
            }
        }

        stats.total_conversations += conversations.len();
        Ok(conversations)
    }
}

impl ChatGPTProvider {
    async fn extract_single_conversation(
        &self,
        data: &Value,
        file: &Path,
        index: usize,
    ) -> ParserResult<Option<Conversation>> {
        let obj = data.as_object().ok_or_else(|| ParserError::InvalidFormat {
            provider: "ChatGPT".to_string(),
            reason: "Conversation must be an object".to_string(),
        })?;

        // Determine format and extract accordingly
        if obj.contains_key("mapping") {
            self.extract_from_mapping_format(obj, file, index).await
        } else if obj.contains_key("messages") {
            self.extract_from_message_array_format(obj, file, index).await
        } else if obj.contains_key("conversation") {
            // Nested conversation object
            if let Some(conv_obj) = obj.get("conversation").and_then(|v| v.as_object()) {
                if let Some(messages) = conv_obj.get("messages") {
                    let mut modified_obj = obj.clone();
                    modified_obj.insert("messages".to_string(), messages.clone());
                    self.extract_from_message_array_format(&modified_obj, file, index).await
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn extract_from_mapping_format(
        &self,
        data: &serde_json::Map<String, Value>,
        file: &Path,
        index: usize,
    ) -> ParserResult<Option<Conversation>> {
        let mapping = data.get("mapping")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ParserError::MissingField("mapping".to_string()))?;

        let title = data.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled ChatGPT Conversation");

        // Extract timestamps
        let create_time = data.get("create_time")
            .and_then(|v| v.as_f64())
            .map(parse_timestamp_numeric)
            .unwrap_or_else(|| Utc::now());

        let update_time = data.get("update_time")
            .and_then(|v| v.as_f64())
            .map(parse_timestamp_numeric)
            .unwrap_or(create_time);

        // Find root node and traverse tree
        let root_id = self.find_root_node(mapping)?;
        let mut messages = Vec::new();
        let mut visited = std::collections::HashSet::new();
        
        self.traverse_message_tree(mapping, &root_id, &mut messages, &mut visited, create_time)?;

        if messages.is_empty() {
            return Ok(None);
        }

        // Sort messages by timestamp
        messages.sort_by_key(|m| m.timestamp);

        // Extract system prompt if present
        let system_prompt = if messages.first().map(|m| m.role) == Some(MessageRole::System) {
            Some(messages.remove(0).content)
        } else {
            None
        };

        // Calculate actual time range
        let start_time = messages.first().map(|m| m.timestamp).unwrap_or(create_time);
        let end_time = messages.last().map(|m| m.timestamp).unwrap_or(update_time);

        let conversation = Conversation {
            id: generate_conversation_id("chatgpt", file, index),
            title: sanitize_title(title, "Untitled ChatGPT Conversation"),
            provider: "ChatGPT".to_string(),
            messages,
            system_prompt,
            model: data.get("default_model_slug")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            start_time,
            end_time,
            metadata: extract_metadata(data).into_iter().collect(),
        };

        Ok(Some(conversation))
    }

    async fn extract_from_message_array_format(
        &self,
        data: &serde_json::Map<String, Value>,
        file: &Path,
        index: usize,
    ) -> ParserResult<Option<Conversation>> {
        let messages_arr = data.get("messages")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ParserError::MissingField("messages".to_string()))?;

        let title = data.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled ChatGPT Conversation");

        let create_time = data.get("create_time")
            .and_then(|v| v.as_f64())
            .map(parse_timestamp_numeric)
            .unwrap_or_else(|| Utc::now());

        let mut messages = Vec::new();
        
        for (msg_idx, msg_data) in messages_arr.iter().enumerate() {
            match self.parse_message(msg_data, create_time) {
                Ok(Some(msg)) => messages.push(msg),
                Ok(None) => {
                    debug!("Skipping empty message at index {}", msg_idx);
                }
                Err(e) => {
                    warn!("Error parsing message {}: {}", msg_idx, e);
                }
            }
        }

        if messages.is_empty() {
            return Ok(None);
        }

        // Extract system prompt if present
        let system_prompt = if messages.first().map(|m| m.role) == Some(MessageRole::System) {
            Some(messages.remove(0).content)
        } else {
            None
        };

        let start_time = messages.first().map(|m| m.timestamp).unwrap_or(create_time);
        let end_time = messages.last().map(|m| m.timestamp).unwrap_or(start_time);

        let conversation = Conversation {
            id: generate_conversation_id("chatgpt", file, index),
            title: sanitize_title(title, "Untitled ChatGPT Conversation"),
            provider: "ChatGPT".to_string(),
            messages,
            system_prompt,
            model: data.get("default_model_slug")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            start_time,
            end_time,
            metadata: extract_metadata(data).into_iter().collect(),
        };

        Ok(Some(conversation))
    }

    fn find_root_node(&self, mapping: &serde_json::Map<String, Value>) -> ParserResult<String> {
        // Look for node with no parent or parent = "ROOT"
        for (node_id, node_data) in mapping {
            if let Some(obj) = node_data.as_object() {
                if let Some(parent) = obj.get("parent") {
                    if parent.is_null() || parent.as_str() == Some("ROOT") {
                        return Ok(node_id.clone());
                    }
                } else {
                    // No parent field means root
                    return Ok(node_id.clone());
                }
            }
        }

        // Fallback: find node with earliest timestamp
        let mut earliest_time = f64::MAX;
        let mut root_id = None;

        for (node_id, node_data) in mapping {
            if let Some(msg) = node_data.get("message")
                .and_then(|v| v.as_object())
                .and_then(|o| o.get("create_time"))
                .and_then(|v| v.as_f64()) 
            {
                if msg < earliest_time {
                    earliest_time = msg;
                    root_id = Some(node_id.clone());
                }
            }
        }

        root_id.ok_or_else(|| ParserError::InvalidFormat {
            provider: "ChatGPT".to_string(),
            reason: "Could not find root node in mapping".to_string(),
        })
    }

    fn traverse_message_tree(
        &self,
        mapping: &serde_json::Map<String, Value>,
        node_id: &str,
        messages: &mut Vec<Message>,
        visited: &mut std::collections::HashSet<String>,
        default_timestamp: DateTime<Utc>,
    ) -> ParserResult<()> {
        if visited.contains(node_id) {
            return Ok(());
        }
        visited.insert(node_id.to_string());

        let node = mapping.get(node_id)
            .and_then(|v| v.as_object())
            .ok_or_else(|| ParserError::InvalidFormat {
                provider: "ChatGPT".to_string(),
                reason: format!("Node {} not found in mapping", node_id),
            })?;

        // Extract message if present
        if let Some(msg_data) = node.get("message") {
            if let Ok(Some(msg)) = self.parse_message(msg_data, default_timestamp) {
                messages.push(msg);
            }
        }

        // Recursively process children
        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            for child in children {
                if let Some(child_id) = child.as_str() {
                    self.traverse_message_tree(mapping, child_id, messages, visited, default_timestamp)?;
                }
            }
        }

        Ok(())
    }

    fn parse_message(
        &self,
        msg_data: &Value,
        default_timestamp: DateTime<Utc>,
    ) -> ParserResult<Option<Message>> {
        let obj = msg_data.as_object()
            .ok_or_else(|| ParserError::InvalidFormat {
                provider: "ChatGPT".to_string(),
                reason: "Message must be an object".to_string(),
            })?;

        // Extract role
        let role_str = obj.get("role")
            .or_else(|| obj.get("author").and_then(|a| a.get("role")))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let role = MessageRole::from_provider_role(role_str, "chatgpt")
            .ok_or_else(|| ParserError::InvalidRole(role_str.to_string()))?;

        // Extract content
        let content = if let Some(content_obj) = obj.get("content").and_then(|v| v.as_object()) {
            self.extract_content_from_object(content_obj)?
        } else if let Some(text) = obj.get("content").and_then(|v| v.as_str()) {
            text.to_string()
        } else if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
            text.to_string()
        } else {
            return Ok(None); // Skip messages without content
        };

        if content.trim().is_empty() {
            return Ok(None);
        }

        // Extract timestamp
        let timestamp = obj.get("create_time")
            .or_else(|| obj.get("timestamp"))
            .and_then(|v| v.as_f64())
            .map(parse_timestamp_numeric)
            .unwrap_or(default_timestamp);

        // Extract model from metadata
        let model = obj.get("metadata")
            .and_then(|v| v.as_object())
            .and_then(|m| m.get("model_slug"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut metadata = HashMap::new();
        if let Some(meta_obj) = obj.get("metadata").and_then(|v| v.as_object()) {
            for (k, v) in extract_metadata(meta_obj) {
                metadata.insert(k, v);
            }
        }

        Ok(Some(Message {
            role,
            content,
            timestamp,
            model,
            metadata,
            media_files: Vec::new(), // ChatGPT exports don't include media references
        }))
    }

    fn extract_content_from_object(&self, content: &serde_json::Map<String, Value>) -> ParserResult<String> {
        // Handle different content types
        let content_type = content.get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text");

        match content_type {
            "text" => {
                // Try parts array first, then text field
                if let Some(parts) = content.get("parts").and_then(|v| v.as_array()) {
                    Ok(extract_text_from_parts(parts))
                } else if let Some(text) = content.get("text").and_then(|v| v.as_str()) {
                    Ok(text.to_string())
                } else {
                    Ok(String::new())
                }
            }
            "code" => {
                // Format code content
                if let Some(code) = content.get("text").and_then(|v| v.as_str()) {
                    Ok(format!("```\n{}\n```", code))
                } else {
                    Ok(String::new())
                }
            }
            "tether_browsing_display" => {
                // Handle browsing results
                let result = content.get("result")
                    .or_else(|| content.get("domain"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                Ok(format!("[Browsing Result: {}]", result))
            }
            _ => {
                // Try to extract any text content
                Ok(extract_text_content(&Value::Object(content.clone())))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio;

    #[tokio::test]
    async fn test_chatgpt_provider_basic() {
        let provider = ChatGPTProvider::new();
        assert_eq!(provider.name(), "ChatGPT");
    }

    #[tokio::test]
    async fn test_can_handle_valid_file() {
        let provider = ChatGPTProvider::new();
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("chat.json");
        
        // Write valid ChatGPT format
        let data = serde_json::json!({
            "title": "Test Chat",
            "mapping": {},
            "create_time": 1234567890.0
        });
        tokio::fs::write(&file, data.to_string()).await.unwrap();
        
        assert!(provider.can_handle(&file).await);
    }
}