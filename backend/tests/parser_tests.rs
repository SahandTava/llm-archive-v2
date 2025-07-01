// Parser unit tests for LLM Archive V2

use llm_archive_backend::parsers::{chatgpt, claude, gemini, zed, Conversation};
use test_utils::*;
use tokio;

mod chatgpt_parser_tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_simple_conversation() {
        let data = chatgpt_samples::simple_conversation();
        let conversations = chatgpt::parse_chatgpt_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        assert_conversation_valid(conv);
        assert_eq!(conv.title, "Test ChatGPT Conversation");
        assert_eq!(conv.provider, "chatgpt");
        assert_eq!(conv.messages.len(), 2);
        
        // Check messages
        assert_eq!(conv.messages[0].role, "user");
        assert_eq!(conv.messages[0].content, "Hello, how are you?");
        assert_eq!(conv.messages[1].role, "assistant");
        assert_eq!(conv.messages[1].content, "I'm doing well, thank you! How can I help you today?");
        assert_eq!(conv.messages[1].model.as_ref().unwrap(), "gpt-4");
    }

    #[tokio::test]
    async fn test_parse_message_array_format() {
        let data = chatgpt_samples::message_array_format();
        let conversations = chatgpt::parse_chatgpt_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        assert_eq!(conv.messages.len(), 2);
        assert_eq!(conv.messages[0].content, "What is 2+2?");
        assert_eq!(conv.messages[1].content, "2+2 equals 4.");
        assert_eq!(conv.messages[1].model.as_ref().unwrap(), "gpt-3.5-turbo");
    }

    #[tokio::test]
    async fn test_parse_conversations_json() {
        let data = chatgpt_samples::conversations_json_format();
        let conversations = chatgpt::parse_chatgpt_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 2);
        assert_conversation_valid(&conversations[0]);
        assert_conversation_valid(&conversations[1]);
    }

    #[tokio::test]
    async fn test_parse_invalid_format() {
        let data = chatgpt_samples::invalid_format();
        let result = chatgpt::parse_chatgpt_json(&data);
        
        assert!(result.is_err(), "Should fail to parse invalid format");
    }

    #[tokio::test]
    async fn test_parse_empty_conversation() {
        let data = chatgpt_samples::empty_conversation();
        let conversations = chatgpt::parse_chatgpt_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].messages.len(), 0);
    }

    #[tokio::test]
    async fn test_performance_large_conversation() {
        let timer = performance::PerformanceTimer::new("ChatGPT parser - 1000 messages");
        let data = performance::generate_large_conversation(1000);
        
        let _ = chatgpt::parse_chatgpt_json(&data).unwrap();
        
        timer.assert_under_ms(100); // Should parse under 100ms
    }
}

mod claude_parser_tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_simple_conversation() {
        let data = claude_samples::simple_conversation();
        let conversations = claude::parse_claude_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        assert_conversation_valid(conv);
        assert_eq!(conv.title, "Test Claude Chat");
        assert_eq!(conv.provider, "claude");
        assert_eq!(conv.messages.len(), 2);
        
        assert_eq!(conv.messages[0].role, "user");
        assert_eq!(conv.messages[0].content, "Hello Claude!");
        assert_eq!(conv.messages[1].role, "assistant");
        assert_eq!(conv.messages[1].content, "Hello! How can I help you today?");
    }

    #[tokio::test]
    async fn test_parse_with_attachments() {
        let data = claude_samples::with_attachments();
        let conversations = claude::parse_claude_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        assert_eq!(conv.messages.len(), 1);
        let msg = &conv.messages[0];
        
        // Should include attachment info in content
        assert!(msg.content.contains("Can you analyze this file?"));
        assert!(msg.content.contains("document.pdf"));
        assert!(msg.content.contains("extracted text from the PDF"));
    }

    #[tokio::test]
    async fn test_performance_multiple_conversations() {
        let timer = performance::PerformanceTimer::new("Claude parser - 50 conversations");
        
        let mut conversations = Vec::new();
        for i in 0..50 {
            let mut conv = claude_samples::simple_conversation();
            conv["uuid"] = serde_json::Value::String(format!("conv-{}", i));
            conversations.push(conv);
        }
        
        let data = serde_json::Value::Array(conversations);
        let _ = claude::parse_claude_json(&data).unwrap();
        
        timer.assert_under_ms(100);
    }
}

mod gemini_parser_tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_takeout_format() {
        let data = gemini_samples::takeout_format();
        let conversations = gemini::parse_gemini_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        assert_conversation_valid(conv);
        assert_eq!(conv.provider, "gemini");
        assert_eq!(conv.messages.len(), 2);
        
        assert_eq!(conv.messages[0].role, "user");
        assert_eq!(conv.messages[0].content, "Hello Gemini!");
        assert_eq!(conv.messages[1].role, "assistant");
        assert_eq!(conv.messages[1].content, "Hello! How can I assist you today?");
    }

    #[tokio::test]
    async fn test_parse_ai_studio_format() {
        let data = gemini_samples::ai_studio_format();
        let conversations = gemini::parse_gemini_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        assert_eq!(conv.messages.len(), 3); // System + 2 messages
        
        // Check system message
        assert_eq!(conv.messages[0].role, "system");
        assert_eq!(conv.messages[0].content, "You are a helpful AI assistant.");
        
        // Check conversation
        assert_eq!(conv.messages[1].role, "user");
        assert_eq!(conv.messages[1].content, "What is machine learning?");
        assert_eq!(conv.messages[2].role, "assistant");
        assert!(conv.messages[2].content.contains("Machine learning"));
    }

    #[tokio::test]
    async fn test_performance_large_takeout() {
        let timer = performance::PerformanceTimer::new("Gemini parser - 100 conversations");
        
        let mut conversations = Vec::new();
        for i in 0..100 {
            let mut conv = gemini_samples::takeout_format();
            if let serde_json::Value::Array(ref mut arr) = conv {
                arr[0]["conversation_id"] = serde_json::Value::String(format!("conv-{}", i));
            }
            conversations.extend(conv.as_array().unwrap().clone());
        }
        
        let data = serde_json::Value::Array(conversations);
        let _ = gemini::parse_gemini_json(&data).unwrap();
        
        timer.assert_under_ms(100);
    }
}

mod zed_parser_tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_conversation_json() {
        let data = zed_samples::conversation_json();
        let conversations = zed::parse_zed_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        assert_conversation_valid(conv);
        assert_eq!(conv.provider, "zed");
        assert_eq!(conv.messages.len(), 2);
        
        assert_eq!(conv.messages[0].role, "user");
        assert_eq!(conv.messages[0].content, "Help me debug this code");
        assert_eq!(conv.messages[1].role, "assistant");
    }

    #[tokio::test]
    async fn test_parse_with_context() {
        let data = zed_samples::with_context();
        let conversations = zed::parse_zed_json(&data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        // Should include context in title or metadata
        assert!(conv.title.contains("main.rs") || conv.messages[0].content.contains("Explain this code"));
    }

    #[tokio::test]
    async fn test_performance_directory_scan() {
        let timer = performance::PerformanceTimer::new("Zed directory scan");
        
        // Simulate scanning a directory with many conversation files
        let temp_dir = create_temp_dir();
        
        // Create 50 conversation files
        for i in 0..50 {
            let mut conv = zed_samples::conversation_json();
            conv["id"] = serde_json::Value::String(format!("conv-{}", i));
            
            let file_path = temp_dir.path().join(format!("conversation-{}.json", i));
            std::fs::write(&file_path, serde_json::to_string(&conv).unwrap()).unwrap();
        }
        
        // Test directory scanning performance
        let files = zed::find_zed_files(temp_dir.path()).unwrap();
        assert_eq!(files.len(), 50);
        
        timer.assert_under_ms(100);
    }
}

mod cross_parser_tests {
    use super::*;

    #[tokio::test]
    async fn test_all_parsers_consistency() {
        // Test that all parsers produce valid conversations
        let chatgpt_convs = chatgpt::parse_chatgpt_json(&chatgpt_samples::simple_conversation()).unwrap();
        let claude_convs = claude::parse_claude_json(&claude_samples::simple_conversation()).unwrap();
        let gemini_convs = gemini::parse_gemini_json(&gemini_samples::takeout_format()).unwrap();
        let zed_convs = zed::parse_zed_json(&zed_samples::conversation_json()).unwrap();
        
        // All should produce valid conversations
        assert_conversation_valid(&chatgpt_convs[0]);
        assert_conversation_valid(&claude_convs[0]);
        assert_conversation_valid(&gemini_convs[0]);
        assert_conversation_valid(&zed_convs[0]);
        
        // Check provider tags
        assert_eq!(chatgpt_convs[0].provider, "chatgpt");
        assert_eq!(claude_convs[0].provider, "claude");
        assert_eq!(gemini_convs[0].provider, "gemini");
        assert_eq!(zed_convs[0].provider, "zed");
    }

    #[tokio::test]
    async fn test_timestamp_normalization() {
        // All parsers should normalize timestamps to RFC3339
        let chatgpt_convs = chatgpt::parse_chatgpt_json(&chatgpt_samples::simple_conversation()).unwrap();
        let claude_convs = claude::parse_claude_json(&claude_samples::simple_conversation()).unwrap();
        let gemini_convs = gemini::parse_gemini_json(&gemini_samples::takeout_format()).unwrap();
        let zed_convs = zed::parse_zed_json(&zed_samples::conversation_json()).unwrap();
        
        // Check timestamp format
        for conv in [&chatgpt_convs[0], &claude_convs[0], &gemini_convs[0], &zed_convs[0]] {
            // Should parse as valid RFC3339
            chrono::DateTime::parse_from_rfc3339(&conv.start_time).unwrap();
            chrono::DateTime::parse_from_rfc3339(&conv.end_time).unwrap();
            
            for msg in &conv.messages {
                chrono::DateTime::parse_from_rfc3339(&msg.timestamp).unwrap();
            }
        }
    }
}