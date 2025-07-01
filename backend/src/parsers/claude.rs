// parsers/claude.rs - Claude/Anthropic export parser

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

use super::{
    common::*, ChatProvider, Conversation, ImportError, ImportStats, ImportWarning, MediaFile,
    Message, MessageRole, ParserError, ParserResult,
};

/// Claude provider implementation
pub struct ClaudeProvider;

impl ClaudeProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ChatProvider for ClaudeProvider {
    fn name(&self) -> &'static str {
        "Claude"
    }

    async fn find_files(&self, dir: &Path) -> ParserResult<Vec<PathBuf>> {
        let mut files = Vec::new();

        // Look for conversations.json (preferred bulk export)
        for filename in &["conversations.json", "conversations"] {
            let conv_file = dir.join(filename);
            if conv_file.exists() && self.can_handle(&conv_file).await {
                files.push(conv_file);
                return Ok(files); // Prefer bulk export if found
            }
        }

        // Otherwise scan for individual JSON files
        let entries = tokio::fs::read_dir(dir).await?;
        let mut entries = tokio_stream::wrappers::ReadDirStream::new(entries);
        
        use tokio_stream::StreamExt;
        while let Some(entry) = entries.next().await {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str());
                let filename = path.file_name().unwrap().to_string_lossy();
                
                // Check JSON files and "conversations" without extension
                if ext == Some("json") || (ext.is_none() && filename == "conversations") {
                    if self.can_handle(&path).await {
                        files.push(path);
                    }
                }
            }
        }

        Ok(files)
    }

    async fn can_handle(&self, file: &Path) -> bool {
        // Special case for "conversations" file without extension
        let filename = file.file_name().unwrap().to_string_lossy();
        if filename == "conversations" && !is_valid_json_file(file) {
            return false;
        }

        // For other files, must be valid JSON
        if file.extension().is_some() && !is_valid_json_file(file) {
            return false;
        }

        // Check file structure
        match tokio::fs::read_to_string(file).await {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(Value::Array(arr)) => {
                    // Check first item for Claude structure
                    arr.first()
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            // Claude conversations.json format
                            obj.contains_key("uuid") && 
                            obj.contains_key("name") &&
                            obj.contains_key("chat_messages")
                        })
                        .unwrap_or(true) // Empty array is valid
                }
                Ok(Value::Object(obj)) => {
                    // Single conversation format
                    (obj.contains_key("uuid") || obj.contains_key("name")) &&
                    (obj.contains_key("chat_messages") || 
                     obj.contains_key("transcript") || 
                     obj.contains_key("messages"))
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

        // Handle both array (bulk) and single conversation formats
        let items = match data {
            Value::Array(arr) => arr,
            Value::Object(obj) => vec![Value::Object(obj)],
            _ => {
                return Err(ParserError::InvalidFormat {
                    provider: "Claude".to_string(),
                    reason: "Root must be array or object".to_string(),
                });
            }
        };

        for (idx, item) in items.iter().enumerate() {
            match self.extract_single_conversation(item, file, idx).await {
                Ok(Some(conv)) => {
                    stats.total_messages += conv.messages.len();
                    stats.total_media_files += conv.messages
                        .iter()
                        .map(|m| m.media_files.len())
                        .sum::<usize>();
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

        stats.total_conversations += conversations.len();
        Ok(conversations)
    }
}

impl ClaudeProvider {
    async fn extract_single_conversation(
        &self,
        data: &Value,
        file: &Path,
        index: usize,
    ) -> ParserResult<Option<Conversation>> {
        let obj = data.as_object().ok_or_else(|| ParserError::InvalidFormat {
            provider: "Claude".to_string(),
            reason: "Conversation must be an object".to_string(),
        })?;

        // Extract basic info
        let title = obj.get("name")
            .or_else(|| obj.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("Claude Conversation");

        let conv_uuid = obj.get("uuid")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Parse timestamps
        let created_at = obj.get("created_at")
            .and_then(|v| v.as_str())
            .and_then(|s| self.parse_claude_timestamp(s))
            .unwrap_or_else(|| Utc::now());

        let updated_at = obj.get("updated_at")
            .and_then(|v| v.as_str())
            .and_then(|s| self.parse_claude_timestamp(s))
            .unwrap_or(created_at);

        // Find messages array
        let messages_arr = obj.get("chat_messages")
            .or_else(|| obj.get("transcript"))
            .or_else(|| obj.get("messages"))
            .and_then(|v| v.as_array());

        if messages_arr.is_none() {
            warn!("No messages found in conversation: {}", title);
            return Ok(None);
        }

        let messages_arr = messages_arr.unwrap();
        let mut messages = Vec::new();
        let mut all_media_files = HashMap::new();
        let mut last_timestamp = created_at;

        // Process each message
        for (msg_idx, msg_data) in messages_arr.iter().enumerate() {
            let msg_obj = match msg_data.as_object() {
                Some(obj) => obj,
                None => {
                    warn!("Invalid message format at index {}", msg_idx);
                    continue;
                }
            };

            // Extract role
            let sender = msg_obj.get("sender")
                .or_else(|| msg_obj.get("role"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let role = match MessageRole::from_provider_role(sender, "claude") {
                Some(r) => r,
                None => {
                    warn!("Unknown role '{}' in message {}", sender, msg_idx);
                    continue;
                }
            };

            // Extract content
            let content = if let Some(content_arr) = msg_obj.get("content").and_then(|v| v.as_array()) {
                self.extract_text_from_content_array(content_arr)
            } else if let Some(text) = msg_obj.get("text").and_then(|v| v.as_str()) {
                text.to_string()
            } else {
                String::new()
            };

            // Extract timestamp
            let timestamp = msg_obj.get("created_at")
                .or_else(|| msg_obj.get("timestamp"))
                .and_then(|v| v.as_str())
                .and_then(|s| self.parse_claude_timestamp(s))
                .unwrap_or_else(|| {
                    // Use small offset from last timestamp to maintain order
                    last_timestamp + chrono::Duration::microseconds(1)
                });
            last_timestamp = timestamp;

            // Extract metadata
            let mut metadata = HashMap::new();
            if let Some(uuid) = msg_obj.get("uuid").and_then(|v| v.as_str()) {
                metadata.insert("message_uuid".to_string(), serde_json::json!(uuid));
            }
            if let Some(updated) = msg_obj.get("updated_at").and_then(|v| v.as_str()) {
                metadata.insert("message_updated_at".to_string(), serde_json::json!(updated));
            }

            // Handle attachments
            let mut message_media = Vec::new();
            if let Some(attachments) = msg_obj.get("attachments").and_then(|v| v.as_array()) {
                for (att_idx, attachment) in attachments.iter().enumerate() {
                    if let Some(media) = self.process_attachment(
                        attachment, 
                        conv_uuid, 
                        msg_idx, 
                        att_idx,
                        &mut all_media_files,
                        &content
                    ) {
                        message_media.push(media);
                    }
                }
            }

            // Add attachment references to metadata if present
            if !message_media.is_empty() {
                let refs: Vec<_> = message_media.iter()
                    .map(|m| serde_json::json!({
                        "filename": m.filename,
                        "logical_path": m.filepath
                    }))
                    .collect();
                metadata.insert("attachments_references".to_string(), serde_json::json!(refs));
            }

            // Only add message if it has content or introduced new media
            if !content.trim().is_empty() || !message_media.is_empty() {
                messages.push(Message {
                    role,
                    content: content.trim().to_string(),
                    timestamp,
                    model: msg_obj.get("model_slug")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    metadata,
                    media_files: message_media,
                });
            }
        }

        if messages.is_empty() {
            return Ok(None);
        }

        // Extract system prompt if first message is system role
        let system_prompt = if messages.first().map(|m| m.role) == Some(MessageRole::System) {
            Some(messages.remove(0).content)
        } else {
            None
        };

        // Calculate time range from actual messages
        let start_time = messages.first()
            .map(|m| m.timestamp)
            .unwrap_or(created_at)
            .min(created_at);
        
        let end_time = messages.last()
            .map(|m| m.timestamp)
            .unwrap_or(updated_at)
            .max(updated_at);

        // Build conversation metadata
        let mut conv_metadata = extract_metadata(obj);
        conv_metadata.insert("conversation_uuid".to_string(), serde_json::json!(conv_uuid));
        conv_metadata.insert("source_file".to_string(), serde_json::json!(file.to_string_lossy()));
        conv_metadata.insert(
            "original_format".to_string(),
            serde_json::json!(if file.file_name().unwrap().to_string_lossy() == "conversations.json" {
                "claude_bulk_export"
            } else {
                "claude_json"
            })
        );

        if let Some(account) = obj.get("account").and_then(|v| v.as_object()) {
            if let Some(acc_uuid) = account.get("uuid").and_then(|v| v.as_str()) {
                conv_metadata.insert("account_uuid".to_string(), serde_json::json!(acc_uuid));
            }
        }

        let conversation = Conversation {
            id: if !conv_uuid.is_empty() {
                format!("claude_{}", conv_uuid)
            } else {
                generate_conversation_id("claude", file, index)
            },
            title: sanitize_title(title, "Claude Conversation"),
            provider: "Claude".to_string(),
            messages,
            system_prompt,
            model: obj.get("model_slug")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            start_time,
            end_time,
            metadata: conv_metadata.into_iter().collect(),
        };

        Ok(Some(conversation))
    }

    fn parse_claude_timestamp(&self, timestamp_str: &str) -> Option<DateTime<Utc>> {
        // Claude uses ISO 8601 format, often with 'Z' suffix
        match parse_timestamp(&timestamp_str.replace("Z", "+00:00")) {
            Ok(dt) => Some(dt),
            Err(_) => {
                // Try parsing without subseconds
                match DateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S") {
                    Ok(dt) => Some(dt.with_timezone(&Utc)),
                    Err(_) => {
                        warn!("Failed to parse Claude timestamp: {}", timestamp_str);
                        None
                    }
                }
            }
        }
    }

    fn extract_text_from_content_array(&self, content_array: &[Value]) -> String {
        let mut text_parts = Vec::new();
        
        for block in content_array {
            if let Some(obj) = block.as_object() {
                if obj.get("type").and_then(|v| v.as_str()) == Some("text") {
                    if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                        text_parts.push(text.to_string());
                    }
                }
            }
        }
        
        text_parts.join("\n")
    }

    fn process_attachment(
        &self,
        attachment: &Value,
        conv_uuid: &str,
        msg_idx: usize,
        att_idx: usize,
        all_media: &mut HashMap<String, MediaFile>,
        message_content: &str,
    ) -> Option<MediaFile> {
        let att_obj = attachment.as_object()?;
        let file_name = att_obj.get("file_name").and_then(|v| v.as_str())?;
        
        let msg_uuid = att_obj.get("uuid")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("msg{}_att{}", msg_idx, att_idx));
        
        let logical_path = format!("claude_attachments/{}/{}/{}", conv_uuid, msg_uuid, file_name);
        
        // Check if we've already processed this file
        if let Some(existing) = all_media.get(&logical_path) {
            return Some(existing.clone());
        }
        
        let mut media = MediaFile {
            filename: file_name.to_string(),
            filepath: logical_path.clone(),
            mime_type: att_obj.get("file_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| detect_mime_type(Path::new(file_name))),
            size_bytes: att_obj.get("file_size")
                .and_then(|v| v.as_u64()),
            extracted_content: att_obj.get("extracted_content")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };
        
        // If extracted content exists and not already in message, store it
        if let Some(ref extracted) = media.extracted_content {
            if !message_content.contains(extracted) {
                // Keep extracted content in media file
            } else {
                // Content already in message, don't duplicate
                media.extracted_content = None;
            }
        }
        
        all_media.insert(logical_path, media.clone());
        Some(media)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio;

    #[tokio::test]
    async fn test_claude_provider_basic() {
        let provider = ClaudeProvider::new();
        assert_eq!(provider.name(), "Claude");
    }

    #[tokio::test]
    async fn test_can_handle_valid_file() {
        let provider = ClaudeProvider::new();
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("conversations.json");
        
        // Write valid Claude format
        let data = serde_json::json!([{
            "uuid": "test-uuid",
            "name": "Test Chat",
            "chat_messages": [{
                "sender": "human",
                "text": "Hello",
                "created_at": "2024-01-15T10:30:00Z"
            }]
        }]);
        tokio::fs::write(&file, data.to_string()).await.unwrap();
        
        assert!(provider.can_handle(&file).await);
    }

    #[tokio::test]
    async fn test_timestamp_parsing() {
        let provider = ClaudeProvider::new();
        
        // ISO 8601 with Z
        let ts = provider.parse_claude_timestamp("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:30:00+00:00");
        
        // Without timezone
        let ts = provider.parse_claude_timestamp("2024-01-15 10:30:00").unwrap();
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:30:00+00:00");
    }
}