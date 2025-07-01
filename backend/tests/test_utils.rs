// Test utilities for LLM Archive V2 tests

use serde_json::{json, Value};
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};
use tokio::fs;

/// Create a temporary directory for testing
pub fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// Create a temporary file with given content
pub async fn create_temp_file(content: &str, extension: &str) -> NamedTempFile {
    let file = NamedTempFile::with_suffix(extension).expect("Failed to create temp file");
    fs::write(file.path(), content).await.expect("Failed to write temp file");
    file
}

/// ChatGPT test data samples
pub mod chatgpt_samples {
    use super::*;

    pub fn simple_conversation() -> Value {
        json!({
            "title": "Test ChatGPT Conversation",
            "create_time": 1672531200.0,
            "update_time": 1672531300.0,
            "mapping": {
                "root": {
                    "id": "root",
                    "parent": null,
                    "children": ["msg1"]
                },
                "msg1": {
                    "id": "msg1",
                    "parent": "root",
                    "children": ["msg2"],
                    "message": {
                        "id": "msg1",
                        "role": "user",
                        "content": {
                            "content_type": "text",
                            "parts": ["Hello, how are you?"]
                        },
                        "create_time": 1672531200.0
                    }
                },
                "msg2": {
                    "id": "msg2",
                    "parent": "msg1",
                    "children": [],
                    "message": {
                        "id": "msg2",
                        "role": "assistant",
                        "content": {
                            "content_type": "text",
                            "parts": ["I'm doing well, thank you! How can I help you today?"]
                        },
                        "create_time": 1672531250.0,
                        "metadata": {
                            "model_slug": "gpt-4"
                        }
                    }
                }
            }
        })
    }

    pub fn message_array_format() -> Value {
        json!({
            "title": "Simple Chat",
            "create_time": 1672531200.0,
            "messages": [
                {
                    "role": "user",
                    "content": "What is 2+2?",
                    "create_time": 1672531200.0
                },
                {
                    "role": "assistant",
                    "content": "2+2 equals 4.",
                    "create_time": 1672531250.0,
                    "metadata": {
                        "model_slug": "gpt-3.5-turbo"
                    }
                }
            ]
        })
    }

    pub fn conversations_json_format() -> Value {
        json!([
            simple_conversation(),
            message_array_format()
        ])
    }

    pub fn invalid_format() -> Value {
        json!({
            "not_a_chat": "invalid data",
            "random_field": 123
        })
    }

    pub fn empty_conversation() -> Value {
        json!({
            "title": "Empty Chat",
            "create_time": 1672531200.0,
            "mapping": {}
        })
    }
}

/// Claude test data samples
pub mod claude_samples {
    use super::*;

    pub fn simple_conversation() -> Value {
        json!({
            "uuid": "test-uuid-123",
            "name": "Test Claude Chat",
            "summary": "A test conversation",
            "created_at": "2023-01-01T12:00:00Z",
            "updated_at": "2023-01-01T12:05:00Z",
            "chat_messages": [
                {
                    "uuid": "msg-1",
                    "text": "Hello Claude!",
                    "sender": "human",
                    "created_at": "2023-01-01T12:00:00Z"
                },
                {
                    "uuid": "msg-2", 
                    "text": "Hello! How can I help you today?",
                    "sender": "assistant",
                    "created_at": "2023-01-01T12:00:30Z"
                }
            ]
        })
    }

    pub fn with_attachments() -> Value {
        json!({
            "uuid": "test-uuid-456",
            "name": "Chat with File",
            "created_at": "2023-01-01T12:00:00Z",
            "updated_at": "2023-01-01T12:05:00Z",
            "chat_messages": [
                {
                    "uuid": "msg-1",
                    "text": "Can you analyze this file?",
                    "sender": "human",
                    "created_at": "2023-01-01T12:00:00Z",
                    "attachments": [
                        {
                            "file_name": "document.pdf",
                            "file_type": "application/pdf",
                            "file_size": 1024000,
                            "extracted_content": "This is the extracted text from the PDF..."
                        }
                    ]
                }
            ]
        })
    }
}

/// Gemini test data samples
pub mod gemini_samples {
    use super::*;

    pub fn takeout_format() -> Value {
        json!([
            {
                "conversation_id": "test-conv-123",
                "conversation": {
                    "create_time": "2023-01-01T12:00:00.000Z",
                    "update_time": "2023-01-01T12:05:00.000Z"
                },
                "turns": [
                    {
                        "role": "user",
                        "parts": [
                            {
                                "text": "Hello Gemini!"
                            }
                        ],
                        "create_time": "2023-01-01T12:00:00.000Z"
                    },
                    {
                        "role": "model",
                        "parts": [
                            {
                                "text": "Hello! How can I assist you today?"
                            }
                        ],
                        "create_time": "2023-01-01T12:00:30.000Z"
                    }
                ]
            }
        ])
    }

    pub fn ai_studio_format() -> Value {
        json!({
            "history": [
                {
                    "role": "user", 
                    "parts": [{"text": "What is machine learning?"}]
                },
                {
                    "role": "model",
                    "parts": [{"text": "Machine learning is a subset of artificial intelligence..."}]
                }
            ],
            "systemInstruction": {
                "parts": [{"text": "You are a helpful AI assistant."}]
            }
        })
    }
}

/// Zed test data samples
pub mod zed_samples {
    use super::*;

    pub fn conversation_json() -> Value {
        json!({
            "id": "test-conv-789",
            "zed_version": "0.100.0",
            "messages": [
                {
                    "id": "msg-1",
                    "role": "user",
                    "content": "Help me debug this code",
                    "timestamp": "2023-01-01T12:00:00Z"
                },
                {
                    "id": "msg-2",
                    "role": "assistant", 
                    "content": "I'd be happy to help! Could you share the code you're working with?",
                    "timestamp": "2023-01-01T12:00:15Z"
                }
            ]
        })
    }

    pub fn with_context() -> Value {
        json!({
            "id": "test-conv-context",
            "zed_version": "0.100.0",
            "context": {
                "file_path": "/home/user/project/main.rs",
                "selected_text": "fn main() {\n    println!(\"Hello, world!\");\n}"
            },
            "messages": [
                {
                    "id": "msg-1",
                    "role": "user",
                    "content": "Explain this code",
                    "timestamp": "2023-01-01T12:00:00Z"
                }
            ]
        })
    }
}

/// Performance test helpers
pub mod performance {
    use std::time::{Duration, Instant};

    pub struct PerformanceTimer {
        start: Instant,
        name: String,
    }

    impl PerformanceTimer {
        pub fn new(name: &str) -> Self {
            Self {
                start: Instant::now(),
                name: name.to_string(),
            }
        }

        pub fn elapsed(&self) -> Duration {
            self.start.elapsed()
        }

        pub fn assert_under_ms(&self, max_ms: u64) {
            let elapsed = self.elapsed();
            assert!(
                elapsed.as_millis() < max_ms as u128,
                "{} took {}ms, expected < {}ms",
                self.name,
                elapsed.as_millis(),
                max_ms
            );
        }
    }

    /// Generate large test dataset
    pub fn generate_large_conversation(message_count: usize) -> serde_json::Value {
        use serde_json::json;
        
        let mut messages = Vec::new();
        for i in 0..message_count {
            let role = if i % 2 == 0 { "user" } else { "assistant" };
            let content = format!("This is test message number {} with some content to simulate real chat data.", i);
            messages.push(json!({
                "role": role,
                "content": content,
                "create_time": 1672531200.0 + (i as f64 * 30.0)
            }));
        }

        json!({
            "title": format!("Large Conversation with {} messages", message_count),
            "create_time": 1672531200.0,
            "messages": messages
        })
    }
}

/// Assertion helpers
pub fn assert_conversation_valid(conv: &llm_archive_backend::parsers::Conversation) {
    assert!(!conv.id.is_empty(), "Conversation ID should not be empty");
    assert!(!conv.title.is_empty(), "Conversation title should not be empty");
    assert!(!conv.provider.is_empty(), "Provider should not be empty");
    assert!(!conv.messages.is_empty(), "Conversation should have messages");
    assert!(conv.start_time <= conv.end_time, "Start time should be <= end time");
    
    // Validate messages are sorted by timestamp
    for window in conv.messages.windows(2) {
        assert!(
            window[0].timestamp <= window[1].timestamp,
            "Messages should be sorted by timestamp"
        );
    }
}