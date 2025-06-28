use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Core domain models - kept minimal and focused
/// These match the simplified database schema

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Provider {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Conversation {
    pub id: i64,
    pub provider: String,
    pub external_id: Option<String>,
    pub title: Option<String>,
    pub model: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    
    // Store raw JSON for future schema migrations
    pub raw_json: Option<serde_json::Value>,
    
    // Metadata that could be useful later
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,  // user, assistant, system, tool
    pub content: String,
    pub model: Option<String>,  // Model can vary per message
    pub created_at: DateTime<Utc>,
    
    // Additional metadata
    pub tokens: Option<i32>,
    pub finish_reason: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub attachments: Option<serde_json::Value>,
}

/// Search result with snippets
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub conversation: Conversation,
    pub snippet: String,
    pub rank: f32,
}

/// Import statistics
#[derive(Debug, Default)]
pub struct ImportStats {
    pub conversations: usize,
    pub messages: usize,
    pub errors: usize,
    pub duration_ms: u64,
}

/// Supported providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    ChatGPT,
    Claude,
    Gemini,
    XAI,
    Zed,
    Unknown,
}

impl ProviderType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "chatgpt" | "openai" => Self::ChatGPT,
            "claude" | "anthropic" => Self::Claude,
            "gemini" | "google" => Self::Gemini,
            "xai" | "grok" => Self::XAI,
            "zed" => Self::Zed,
            _ => Self::Unknown,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChatGPT => "chatgpt",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
            Self::XAI => "xai",
            Self::Zed => "zed",
            Self::Unknown => "unknown",
        }
    }
}