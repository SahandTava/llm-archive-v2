// Performance regression tests for LLM Archive V2

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use llm_archive_backend::parsers::{chatgpt, claude, gemini, zed};
use std::time::Duration;
use test_utils::*;

fn benchmark_chatgpt_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("chatgpt_parser");
    group.measurement_time(Duration::from_secs(10));
    
    // Small conversation (2 messages)
    group.bench_function("small_conversation", |b| {
        let data = chatgpt_samples::simple_conversation();
        b.iter(|| {
            let _ = chatgpt::parse_chatgpt_json(black_box(&data));
        });
    });
    
    // Medium conversation (100 messages)
    group.bench_function("medium_conversation", |b| {
        let data = performance::generate_large_conversation(100);
        b.iter(|| {
            let _ = chatgpt::parse_chatgpt_json(black_box(&data));
        });
    });
    
    // Large conversation (1000 messages)
    group.bench_function("large_conversation", |b| {
        let data = performance::generate_large_conversation(1000);
        b.iter(|| {
            let _ = chatgpt::parse_chatgpt_json(black_box(&data));
        });
    });
    
    // Multiple conversations
    group.bench_function("multiple_conversations", |b| {
        let mut conversations = Vec::new();
        for _ in 0..50 {
            conversations.push(chatgpt_samples::simple_conversation());
        }
        let data = serde_json::Value::Array(conversations);
        
        b.iter(|| {
            let _ = chatgpt::parse_chatgpt_json(black_box(&data));
        });
    });
    
    group.finish();
}

fn benchmark_claude_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("claude_parser");
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("simple_conversation", |b| {
        let data = claude_samples::simple_conversation();
        b.iter(|| {
            let _ = claude::parse_claude_json(black_box(&data));
        });
    });
    
    group.bench_function("with_attachments", |b| {
        let data = claude_samples::with_attachments();
        b.iter(|| {
            let _ = claude::parse_claude_json(black_box(&data));
        });
    });
    
    group.finish();
}

fn benchmark_gemini_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("gemini_parser");
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("takeout_format", |b| {
        let data = gemini_samples::takeout_format();
        b.iter(|| {
            let _ = gemini::parse_gemini_json(black_box(&data));
        });
    });
    
    group.bench_function("ai_studio_format", |b| {
        let data = gemini_samples::ai_studio_format();
        b.iter(|| {
            let _ = gemini::parse_gemini_json(black_box(&data));
        });
    });
    
    group.finish();
}

fn benchmark_zed_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("zed_parser");
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("conversation_json", |b| {
        let data = zed_samples::conversation_json();
        b.iter(|| {
            let _ = zed::parse_zed_json(black_box(&data));
        });
    });
    
    group.bench_function("with_context", |b| {
        let data = zed_samples::with_context();
        b.iter(|| {
            let _ = zed::parse_zed_json(black_box(&data));
        });
    });
    
    group.finish();
}

fn benchmark_message_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_extraction");
    group.measurement_time(Duration::from_secs(10));
    
    // Benchmark extracting messages from different formats
    group.bench_function("chatgpt_mapping_traversal", |b| {
        let data = chatgpt_samples::simple_conversation();
        b.iter(|| {
            if let Some(mapping) = data.get("mapping") {
                let mut count = 0;
                for (_, node) in mapping.as_object().unwrap() {
                    if node.get("message").is_some() {
                        count += 1;
                    }
                }
                black_box(count);
            }
        });
    });
    
    group.bench_function("message_array_iteration", |b| {
        let data = chatgpt_samples::message_array_format();
        b.iter(|| {
            if let Some(messages) = data.get("messages") {
                let count = messages.as_array().unwrap().len();
                black_box(count);
            }
        });
    });
    
    group.finish();
}

fn benchmark_timestamp_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("timestamp_parsing");
    
    group.bench_function("unix_timestamp", |b| {
        b.iter(|| {
            let ts = 1672531200.0;
            let dt = chrono::DateTime::from_timestamp(ts as i64, 0).unwrap();
            black_box(dt.to_rfc3339());
        });
    });
    
    group.bench_function("iso8601_parsing", |b| {
        b.iter(|| {
            let ts = "2023-01-01T12:00:00Z";
            let dt = chrono::DateTime::parse_from_rfc3339(ts).unwrap();
            black_box(dt.timestamp());
        });
    });
    
    group.finish();
}

fn benchmark_json_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_operations");
    
    // Benchmark JSON parsing
    group.bench_function("parse_small_json", |b| {
        let json_str = serde_json::to_string(&chatgpt_samples::simple_conversation()).unwrap();
        b.iter(|| {
            let _: serde_json::Value = serde_json::from_str(black_box(&json_str)).unwrap();
        });
    });
    
    // Benchmark JSON serialization
    group.bench_function("serialize_conversation", |b| {
        let conv = chatgpt_samples::simple_conversation();
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&conv)).unwrap();
        });
    });
    
    group.finish();
}

// Performance regression tests
#[cfg(test)]
mod regression_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_parser_performance_targets() {
        // Test that parsers meet <100ms target for standard files
        
        // ChatGPT: 100 messages should parse in <100ms
        let start = Instant::now();
        let data = performance::generate_large_conversation(100);
        let _ = chatgpt::parse_chatgpt_json(&data).unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 100, "ChatGPT parser took {}ms", elapsed.as_millis());
        
        // Claude: 50 conversations should parse in <100ms
        let start = Instant::now();
        let mut conversations = Vec::new();
        for i in 0..50 {
            let mut conv = claude_samples::simple_conversation();
            conv["uuid"] = serde_json::Value::String(format!("conv-{}", i));
            conversations.push(conv);
        }
        let data = serde_json::Value::Array(conversations);
        let _ = claude::parse_claude_json(&data).unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 100, "Claude parser took {}ms", elapsed.as_millis());
    }

    #[test]
    fn test_memory_usage() {
        // Ensure parsers don't use excessive memory
        // This is a simple test - in production, use valgrind or similar
        
        let data = performance::generate_large_conversation(10000);
        let json_size = serde_json::to_string(&data).unwrap().len();
        
        // Parse and measure
        let conversations = chatgpt::parse_chatgpt_json(&data).unwrap();
        
        // Rough estimate: parsed data shouldn't be more than 2x the JSON size
        let estimated_size = conversations.len() * std::mem::size_of::<llm_archive_backend::parsers::Conversation>();
        assert!(estimated_size < json_size * 2, "Excessive memory usage detected");
    }
}

criterion_group!(
    benches,
    benchmark_chatgpt_parser,
    benchmark_claude_parser,
    benchmark_gemini_parser,
    benchmark_zed_parser,
    benchmark_message_extraction,
    benchmark_timestamp_parsing,
    benchmark_json_operations
);

criterion_main!(benches);