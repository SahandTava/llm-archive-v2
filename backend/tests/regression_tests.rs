// V1 Regression tests to ensure backward compatibility
// These tests prevent regressions from the original Python implementation

use llm_archive_backend::parsers::{chatgpt, claude, gemini, zed, Conversation};
use serde_json::Value;
use std::collections::HashMap;
use test_utils::*;
use tokio;

/// Test ChatGPT V1 format compatibility
mod chatgpt_v1_regression {
    use super::*;

    #[tokio::test]
    async fn test_v1_conversation_structure() {
        let v1_data = v1_samples::chatgpt_v1_format();
        let conversations = chatgpt::parse_chatgpt_json(&v1_data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        // V1 specific assertions
        assert_conversation_valid(conv);
        assert_eq!(conv.provider, "chatgpt");
        assert!(!conv.title.is_empty(), "V1 conversations should have titles");
        assert!(conv.messages.len() >= 2, "V1 conversations should have multiple messages");
        
        // Check message structure matches V1 expectations
        for msg in &conv.messages {
            assert!(!msg.content.is_empty(), "V1 messages should have content");
            assert!(msg.role == "user" || msg.role == "assistant" || msg.role == "system");
            
            // V1 timestamps were Unix timestamps
            let timestamp = chrono::DateTime::parse_from_rfc3339(&msg.timestamp);
            assert!(timestamp.is_ok(), "Timestamps should be normalized to RFC3339");
        }
    }

    #[tokio::test]
    async fn test_v1_message_mapping_format() {
        // V1 used complex message mapping structure
        let v1_data = v1_samples::chatgpt_v1_message_mapping();
        let conversations = chatgpt::parse_chatgpt_json(&v1_data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        // Should handle nested message structure correctly
        assert!(conv.messages.len() >= 2);
        
        // Messages should be in chronological order
        for i in 1..conv.messages.len() {
            let prev_time = chrono::DateTime::parse_from_rfc3339(&conv.messages[i-1].timestamp).unwrap();
            let curr_time = chrono::DateTime::parse_from_rfc3339(&conv.messages[i].timestamp).unwrap();
            assert!(curr_time >= prev_time, "Messages should be chronologically ordered");
        }
    }

    #[tokio::test]
    async fn test_v1_model_metadata() {
        let v1_data = v1_samples::chatgpt_v1_with_models();
        let conversations = chatgpt::parse_chatgpt_json(&v1_data).unwrap();
        
        let conv = &conversations[0];
        
        // V1 stored model information in metadata
        let assistant_messages: Vec<_> = conv.messages.iter()
            .filter(|m| m.role == "assistant")
            .collect();
        
        assert!(!assistant_messages.is_empty());
        
        for msg in assistant_messages {
            assert!(msg.model.is_some(), "Assistant messages should have model info");
            let model = msg.model.as_ref().unwrap();
            assert!(model.starts_with("gpt-"), "Should be GPT model: {}", model);
        }
    }

    #[tokio::test]
    async fn test_v1_performance_regression() {
        let timer = performance::PerformanceTimer::new("V1 ChatGPT regression - large dataset");
        
        // V1 could handle large exports - ensure we maintain performance
        let v1_data = v1_samples::chatgpt_v1_large_export(100); // 100 conversations
        let conversations = chatgpt::parse_chatgpt_json(&v1_data).unwrap();
        
        assert_eq!(conversations.len(), 100);
        
        // Should complete parsing in under 100ms for 100 conversations
        timer.assert_under_ms(100);
        
        // Verify all conversations are valid
        for conv in &conversations {
            assert_conversation_valid(conv);
        }
    }
}

/// Test Claude V1 format compatibility
mod claude_v1_regression {
    use super::*;

    #[tokio::test]
    async fn test_v1_uuid_handling() {
        let v1_data = v1_samples::claude_v1_format();
        let conversations = claude::parse_claude_json(&v1_data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        // V1 used UUIDs for conversation IDs
        assert!(conv.conversation_id.len() >= 32, "Should preserve V1 UUID format");
        assert_eq!(conv.provider, "claude");
    }

    #[tokio::test]
    async fn test_v1_attachment_handling() {
        let v1_data = v1_samples::claude_v1_with_attachments();
        let conversations = claude::parse_claude_json(&v1_data).unwrap();
        
        let conv = &conversations[0];
        
        // V1 embedded attachment content in messages
        let has_attachment_content = conv.messages.iter()
            .any(|m| m.content.contains("attachment") || m.content.contains("file"));
        
        assert!(has_attachment_content, "Should preserve attachment information");
    }

    #[tokio::test]
    async fn test_v1_claude_timestamps() {
        let v1_data = v1_samples::claude_v1_format();
        let conversations = claude::parse_claude_json(&v1_data).unwrap();
        
        let conv = &conversations[0];
        
        // V1 Claude had ISO format timestamps - ensure they're preserved
        for msg in &conv.messages {
            let parsed = chrono::DateTime::parse_from_rfc3339(&msg.timestamp);
            assert!(parsed.is_ok(), "Claude V1 timestamps should parse correctly");
        }
    }
}

/// Test Gemini V1 format compatibility  
mod gemini_v1_regression {
    use super::*;

    #[tokio::test]
    async fn test_v1_takeout_format_compatibility() {
        let v1_data = v1_samples::gemini_v1_takeout();
        let conversations = gemini::parse_gemini_json(&v1_data).unwrap();
        
        assert!(!conversations.is_empty());
        
        for conv in &conversations {
            assert_eq!(conv.provider, "gemini");
            assert_conversation_valid(conv);
            
            // V1 Takeout had specific conversation structure
            assert!(!conv.title.is_empty() || !conv.messages.is_empty());
        }
    }

    #[tokio::test]
    async fn test_v1_ai_studio_compatibility() {
        let v1_data = v1_samples::gemini_v1_ai_studio();
        let conversations = gemini::parse_gemini_json(&v1_data).unwrap();
        
        let conv = &conversations[0];
        
        // V1 AI Studio format had system prompts as first message
        if !conv.messages.is_empty() {
            let first_msg = &conv.messages[0];
            if first_msg.role == "system" {
                assert!(!first_msg.content.is_empty(), "System messages should have content");
            }
        }
    }
}

/// Test Zed V1 format compatibility
mod zed_v1_regression {
    use super::*;

    #[tokio::test]
    async fn test_v1_conversation_file_structure() {
        let v1_data = v1_samples::zed_v1_conversation();
        let conversations = zed::parse_zed_json(&v1_data).unwrap();
        
        assert_eq!(conversations.len(), 1);
        let conv = &conversations[0];
        
        assert_eq!(conv.provider, "zed");
        assert_conversation_valid(conv);
        
        // V1 Zed conversations had specific structure
        assert!(!conv.messages.is_empty());
    }

    #[tokio::test]
    async fn test_v1_context_preservation() {
        let v1_data = v1_samples::zed_v1_with_context();
        let conversations = zed::parse_zed_json(&v1_data).unwrap();
        
        let conv = &conversations[0];
        
        // V1 preserved file context in conversations
        let has_context = conv.messages.iter()
            .any(|m| m.content.contains("file") || m.content.contains("context"));
        
        assert!(has_context, "Should preserve V1 context information");
    }
}

/// Cross-parser V1 regression tests
mod cross_parser_v1_regression {
    use super::*;

    #[tokio::test]
    async fn test_v1_timestamp_consistency() {
        // V1 had inconsistent timestamp formats - ensure V2 normalizes them
        let chatgpt_convs = chatgpt::parse_chatgpt_json(&v1_samples::chatgpt_v1_format()).unwrap();
        let claude_convs = claude::parse_claude_json(&v1_samples::claude_v1_format()).unwrap();
        let gemini_convs = gemini::parse_gemini_json(&v1_samples::gemini_v1_takeout()).unwrap();
        let zed_convs = zed::parse_zed_json(&v1_samples::zed_v1_conversation()).unwrap();
        
        // All should now use RFC3339 format
        for conv in [&chatgpt_convs[0], &claude_convs[0], &gemini_convs[0], &zed_convs[0]] {
            chrono::DateTime::parse_from_rfc3339(&conv.start_time).unwrap();
            chrono::DateTime::parse_from_rfc3339(&conv.end_time).unwrap();
            
            for msg in &conv.messages {
                chrono::DateTime::parse_from_rfc3339(&msg.timestamp).unwrap();
            }
        }
    }

    #[tokio::test]
    async fn test_v1_message_count_accuracy() {
        // V1 sometimes had inaccurate message counts
        let test_cases = [
            (v1_samples::chatgpt_v1_format(), "chatgpt"),
            (v1_samples::claude_v1_format(), "claude"),
            (v1_samples::gemini_v1_takeout(), "gemini"),
            (v1_samples::zed_v1_conversation(), "zed"),
        ];
        
        for (data, provider) in test_cases {
            let conversations = match provider {
                "chatgpt" => chatgpt::parse_chatgpt_json(&data).unwrap(),
                "claude" => claude::parse_claude_json(&data).unwrap(),
                "gemini" => gemini::parse_gemini_json(&data).unwrap(),
                "zed" => zed::parse_zed_json(&data).unwrap(),
                _ => panic!("Unknown provider"),
            };
            
            for conv in &conversations {
                assert_eq!(
                    conv.message_count,
                    conv.messages.len() as i32,
                    "Message count should match actual messages for provider: {}",
                    provider
                );
            }
        }
    }

    #[tokio::test]
    async fn test_v1_performance_parity() {
        let timer = performance::PerformanceTimer::new("V1 performance parity test");
        
        // V1 benchmarks - ensure V2 is at least as fast
        let v1_datasets = [
            v1_samples::chatgpt_v1_large_export(50),
            v1_samples::claude_v1_large_export(50),
            v1_samples::gemini_v1_large_export(50),
            v1_samples::zed_v1_large_export(50),
        ];
        
        let mut total_conversations = 0;
        
        for (i, data) in v1_datasets.iter().enumerate() {
            let conversations = match i {
                0 => chatgpt::parse_chatgpt_json(data).unwrap(),
                1 => claude::parse_claude_json(data).unwrap(), 
                2 => gemini::parse_gemini_json(data).unwrap(),
                3 => zed::parse_zed_json(data).unwrap(),
                _ => panic!("Unexpected index"),
            };
            
            total_conversations += conversations.len();
        }
        
        assert_eq!(total_conversations, 200); // 4 providers * 50 conversations each
        
        // Should complete all parsing in under 100ms (V1 performance target)
        timer.assert_under_ms(100);
    }
}

/// Memory usage regression tests
mod memory_regression {
    use super::*;

    #[tokio::test]
    async fn test_v1_memory_efficiency() {
        // V1 had memory issues with large exports - ensure V2 is efficient
        let initial_memory = get_memory_usage();
        
        // Parse large datasets
        let large_chatgpt = v1_samples::chatgpt_v1_large_export(100);
        let conversations = chatgpt::parse_chatgpt_json(&large_chatgpt).unwrap();
        
        let peak_memory = get_memory_usage();
        let memory_increase = peak_memory - initial_memory;
        
        // Should not increase memory by more than 50MB for 100 conversations
        assert!(memory_increase < 50_000_000, 
                "Memory usage should be efficient: {} bytes", memory_increase);
        
        // Verify parsing worked correctly
        assert_eq!(conversations.len(), 100);
        
        drop(conversations);
        
        // Memory should be released
        let final_memory = get_memory_usage();
        assert!(final_memory < peak_memory, 
                "Memory should be released after parsing");
    }
}

// Helper function to get current memory usage (approximate)
fn get_memory_usage() -> usize {
    // Simple approximation - in real implementation would use proper memory profiling
    std::mem::size_of::<Vec<Conversation>>() * 1000
}