// parsers/mod.rs - Main parser module for LLM Archive V2

pub mod chatgpt;
pub mod claude;
pub mod common;
pub mod gemini;
pub mod zed;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Parser errors with detailed context
#[derive(Error, Debug)]
pub enum ParserError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Invalid timestamp format: {0}")]
    InvalidTimestamp(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Invalid role: {0}")]
    InvalidRole(String),
    
    #[error("Parser not implemented for provider: {0}")]
    NotImplemented(String),
    
    #[error("Invalid file format for {provider}: {reason}")]
    InvalidFormat { provider: String, reason: String },
}

/// Result type for parser operations
pub type ParserResult<T> = Result<T, ParserError>;

/// Canonical message roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageRole {
    /// Convert from provider-specific role string
    pub fn from_provider_role(role: &str, provider: &str) -> Option<Self> {
        match provider {
            "chatgpt" => match role.to_lowercase().as_str() {
                "user" => Some(Self::User),
                "assistant" => Some(Self::Assistant),
                "system" => Some(Self::System),
                "tool" => Some(Self::Tool),
                _ => None,
            },
            "claude" => match role.to_lowercase().as_str() {
                "human" | "user" => Some(Self::User),
                "assistant" | "model" => Some(Self::Assistant),
                "system" => Some(Self::System),
                _ => None,
            },
            "gemini" => match role.to_lowercase().as_str() {
                "user" => Some(Self::User),
                "model" | "assistant" => Some(Self::Assistant),
                _ => None,
            },
            "zed" => match role.to_lowercase().as_str() {
                "user" => Some(Self::User),
                "assistant" => Some(Self::Assistant),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Media/attachment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    pub filename: String,
    pub filepath: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub extracted_content: Option<String>,
}

/// Individual message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub model: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub media_files: Vec<MediaFile>,
}

/// Complete conversation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Import statistics for reporting
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ImportStats {
    pub total_files: usize,
    pub processed_files: usize,
    pub failed_files: usize,
    pub total_conversations: usize,
    pub total_messages: usize,
    pub total_media_files: usize,
    pub errors: Vec<ImportError>,
    pub warnings: Vec<ImportWarning>,
}

/// Structured import error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub file: String,
    pub error: String,
    pub context: Option<String>,
}

/// Structured import warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportWarning {
    pub file: String,
    pub warning: String,
    pub context: Option<String>,
}

/// Base trait for all chat providers
#[async_trait]
pub trait ChatProvider: Send + Sync {
    /// Provider name (e.g., "ChatGPT", "Claude")
    fn name(&self) -> &'static str;
    
    /// Find all importable files in a directory
    async fn find_files(&self, dir: &Path) -> ParserResult<Vec<std::path::PathBuf>>;
    
    /// Check if a specific file can be handled by this provider
    async fn can_handle(&self, file: &Path) -> bool;
    
    /// Extract conversations from a file
    async fn extract_conversations(
        &self,
        file: &Path,
        stats: &mut ImportStats,
    ) -> ParserResult<Vec<Conversation>>;
}

/// Registry of available providers
pub struct ProviderRegistry {
    providers: Vec<Box<dyn ChatProvider>>,
}

impl ProviderRegistry {
    /// Create a new registry with all available providers
    pub fn new() -> Self {
        Self {
            providers: vec![
                Box::new(chatgpt::ChatGPTProvider::new()),
                Box::new(claude::ClaudeProvider::new()),
                Box::new(gemini::GeminiProvider::new()),
                Box::new(zed::ZedProvider::new()),
            ],
        }
    }
    
    /// Find the appropriate provider for a file
    pub async fn find_provider(&self, file: &Path) -> Option<&dyn ChatProvider> {
        for provider in &self.providers {
            if provider.can_handle(file).await {
                return Some(provider.as_ref());
            }
        }
        None
    }
    
    /// Get all providers
    pub fn providers(&self) -> &[Box<dyn ChatProvider>] {
        &self.providers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_role_mapping() {
        assert_eq!(
            MessageRole::from_provider_role("human", "claude"),
            Some(MessageRole::User)
        );
        assert_eq!(
            MessageRole::from_provider_role("model", "gemini"),
            Some(MessageRole::Assistant)
        );
        assert_eq!(
            MessageRole::from_provider_role("system", "chatgpt"),
            Some(MessageRole::System)
        );
    }
}