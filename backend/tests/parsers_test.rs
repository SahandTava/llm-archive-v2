use llm_archive_v2::parsers::{chatgpt, claude, ChatProvider, Conversation, Message};
use std::fs;
use std::path::Path;

#[test]
fn test_chatgpt_parser() {
    let sample_data = r#"{
        "conversations": [{
            "id": "test-123",
            "title": "Test Conversation",
            "create_time": 1704103330.123456,
            "mapping": {
                "msg1": {
                    "message": {
                        "author": {"role": "system"},
                        "content": {"parts": ["You are a helpful assistant."]},
                        "create_time": 1704103330.123456
                    },
                    "children": ["msg2"]
                },
                "msg2": {
                    "message": {
                        "author": {"role": "user"},
                        "content": {"parts": ["Hello!"]},
                        "create_time": 1704103331.123456
                    },
                    "children": ["msg3"]
                },
                "msg3": {
                    "message": {
                        "author": {"role": "assistant"},
                        "content": {"parts": ["Hi! How can I help you today?"]},
                        "create_time": 1704103332.123456
                    }
                }
            }
        }]
    }"#;

    let parser = chatgpt::ChatGPTParser::new();
    let result = parser.parse(sample_data.as_bytes()).unwrap();
    
    assert_eq!(result.conversations.len(), 1);
    let conv = &result.conversations[0];
    assert_eq!(conv.title, "Test Conversation");
    assert_eq!(conv.messages.len(), 3);
    
    // Check system prompt
    assert_eq!(conv.messages[0].role, "system");
    assert_eq!(conv.messages[0].content, "You are a helpful assistant.");
    
    // Check user message
    assert_eq!(conv.messages[1].role, "user");
    assert_eq!(conv.messages[1].content, "Hello!");
    
    // Check assistant response
    assert_eq!(conv.messages[2].role, "assistant");
    assert_eq!(conv.messages[2].content, "Hi! How can I help you today?");
}

#[test]
fn test_claude_parser() {
    let sample_data = r#"{
        "conversations": [{
            "uuid": "claude-123",
            "name": "Claude Test",
            "created_at": "2024-01-01T10:00:00.000Z",
            "chat_messages": [
                {
                    "sender": "human",
                    "text": "What is Rust?",
                    "created_at": "2024-01-01T10:00:00.000Z"
                },
                {
                    "sender": "assistant",
                    "text": "Rust is a systems programming language...",
                    "created_at": "2024-01-01T10:00:01.000Z"
                }
            ]
        }]
    }"#;

    let parser = claude::ClaudeParser::new();
    let result = parser.parse(sample_data.as_bytes()).unwrap();
    
    assert_eq!(result.conversations.len(), 1);
    let conv = &result.conversations[0];
    assert_eq!(conv.title, "Claude Test");
    assert_eq!(conv.messages.len(), 2);
    
    assert_eq!(conv.messages[0].role, "user");
    assert_eq!(conv.messages[0].content, "What is Rust?");
    
    assert_eq!(conv.messages[1].role, "assistant");
}

#[test]
fn test_timestamp_parsing() {
    use llm_archive_v2::parsers::common::parse_timestamp;
    
    // Test various timestamp formats
    assert!(parse_timestamp("2024-01-01T10:00:00Z").is_ok());
    assert!(parse_timestamp("2024-01-01T10:00:00.000Z").is_ok());
    assert!(parse_timestamp("1704103330").is_ok());
    assert!(parse_timestamp("1704103330.123456").is_ok());
}

#[test]
fn test_performance_chatgpt_large() {
    // Test parsing performance with larger dataset
    let start = std::time::Instant::now();
    
    // Create a large test dataset
    let mut conversations = vec![];
    for i in 0..100 {
        conversations.push(format!(r#"{{
            "id": "conv-{}",
            "title": "Conversation {}",
            "create_time": 1704103330,
            "mapping": {{}}
        }}"#, i, i));
    }
    
    let sample_data = format!(r#"{{"conversations": [{}]}}"#, conversations.join(","));
    
    let parser = chatgpt::ChatGPTParser::new();
    let result = parser.parse(sample_data.as_bytes()).unwrap();
    
    let elapsed = start.elapsed();
    
    assert_eq!(result.conversations.len(), 100);
    // Should parse 100 conversations in under 100ms
    assert!(elapsed.as_millis() < 100, "Parsing took {:?}", elapsed);
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_real_chatgpt_export() {
        let test_file = "../test-data/chatgpt-sample.json";
        if Path::new(test_file).exists() {
            let data = fs::read(test_file).unwrap();
            let parser = chatgpt::ChatGPTParser::new();
            let result = parser.parse(&data).unwrap();
            
            assert!(!result.conversations.is_empty());
            for conv in &result.conversations {
                assert!(!conv.title.is_empty());
                assert!(!conv.messages.is_empty());
                
                // Verify all messages have timestamps
                for msg in &conv.messages {
                    assert!(msg.timestamp > 0);
                }
            }
        }
    }
    
    #[test]
    fn test_real_claude_export() {
        let test_file = "../test-data/claude-sample.json";
        if Path::new(test_file).exists() {
            let data = fs::read(test_file).unwrap();
            let parser = claude::ClaudeParser::new();
            let result = parser.parse(&data).unwrap();
            
            assert!(!result.conversations.is_empty());
        }
    }
}