use axum::http::StatusCode;
use axum_test::TestServer;
use sqlx::SqlitePool;
use std::time::Instant;

#[tokio::test]
async fn test_health_endpoint() {
    let app = llm_archive_v2::create_app(":memory:").await;
    let server = TestServer::new(app).unwrap();
    
    let response = server.get("/health").await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.text(), "OK");
}

#[tokio::test]
async fn test_search_performance() {
    let app = llm_archive_v2::create_app(":memory:").await;
    let server = TestServer::new(app).unwrap();
    
    // Seed some test data
    seed_test_data(&server).await;
    
    // Test search performance
    let start = Instant::now();
    let response = server.get("/api/search?q=rust").await;
    let elapsed = start.elapsed();
    
    assert_eq!(response.status_code(), StatusCode::OK);
    // Must respond in under 100ms
    assert!(elapsed.as_millis() < 100, "Search took {:?}", elapsed);
    
    let json: serde_json::Value = response.json();
    assert!(json["results"].is_array());
}

#[tokio::test]
async fn test_conversations_pagination() {
    let app = llm_archive_v2::create_app(":memory:").await;
    let server = TestServer::new(app).unwrap();
    
    seed_test_data(&server).await;
    
    // Test first page
    let response = server.get("/api/conversations?page=1&per_page=10").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let json: serde_json::Value = response.json();
    assert_eq!(json["conversations"].as_array().unwrap().len(), 10);
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 10);
}

#[tokio::test]
async fn test_conversation_messages() {
    let app = llm_archive_v2::create_app(":memory:").await;
    let server = TestServer::new(app).unwrap();
    
    seed_test_data(&server).await;
    
    // Get first conversation
    let response = server.get("/api/conversations/1/messages").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let json: serde_json::Value = response.json();
    let messages = json["messages"].as_array().unwrap();
    
    // Verify message structure
    assert!(!messages.is_empty());
    for msg in messages {
        assert!(msg["role"].is_string());
        assert!(msg["content"].is_string());
        assert!(msg["timestamp"].is_number());
    }
}

#[tokio::test]
async fn test_import_chatgpt() {
    let app = llm_archive_v2::create_app(":memory:").await;
    let server = TestServer::new(app).unwrap();
    
    let chatgpt_data = r#"{
        "conversations": [{
            "id": "test-import",
            "title": "Import Test",
            "create_time": 1704103330,
            "mapping": {
                "msg1": {
                    "message": {
                        "author": {"role": "user"},
                        "content": {"parts": ["Test import"]},
                        "create_time": 1704103330
                    }
                }
            }
        }]
    }"#;
    
    let response = server
        .post("/api/import")
        .json(&serde_json::json!({
            "provider": "chatgpt",
            "data": chatgpt_data
        }))
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let json: serde_json::Value = response.json();
    assert_eq!(json["imported"], 1);
    assert_eq!(json["provider"], "chatgpt");
}

#[tokio::test]
async fn test_concurrent_requests() {
    let app = llm_archive_v2::create_app(":memory:").await;
    let server = TestServer::new(app).unwrap();
    
    seed_test_data(&server).await;
    
    // Test concurrent requests don't degrade performance
    let start = Instant::now();
    
    let mut handles = vec![];
    for _ in 0..10 {
        let server_clone = server.clone();
        let handle = tokio::spawn(async move {
            server_clone.get("/api/search?q=test").await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let response = handle.await.unwrap();
        assert_eq!(response.status_code(), StatusCode::OK);
    }
    
    let elapsed = start.elapsed();
    // 10 concurrent requests should complete in under 200ms
    assert!(elapsed.as_millis() < 200, "Concurrent requests took {:?}", elapsed);
}

// Helper function to seed test data
async fn seed_test_data(server: &TestServer) {
    // Insert test providers
    for provider in ["chatgpt", "claude", "gemini"] {
        let data = format!(r#"{{
            "conversations": [
                {{
                    "id": "{}-1",
                    "title": "{} Conversation 1",
                    "create_time": 1704103330,
                    "mapping": {{}}
                }},
                {{
                    "id": "{}-2", 
                    "title": "{} Conversation 2 about Rust",
                    "create_time": 1704103340,
                    "mapping": {{}}
                }}
            ]
        }}"#, provider, provider, provider, provider);
        
        server.post("/api/import")
            .json(&serde_json::json!({
                "provider": provider,
                "data": data
            }))
            .await;
    }
}