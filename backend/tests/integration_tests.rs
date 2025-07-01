// Integration tests for LLM Archive V2 API endpoints

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower::ServiceExt;
use llm_archive_backend::{create_app, AppState};
use test_utils::*;

async fn setup_test_app() -> (axum::Router, SqlitePool) {
    // Create in-memory database
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap();
    
    let state = AppState {
        db: pool.clone(),
    };
    
    let app = create_app(state);
    
    (app, pool)
}

async fn insert_test_conversation(pool: &SqlitePool) -> i64 {
    let result = sqlx::query!(
        r#"
        INSERT INTO conversations (id, title, provider, start_time, end_time, metadata)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        "test-conv-1",
        "Test Conversation",
        "chatgpt",
        "2023-01-01T12:00:00Z",
        "2023-01-01T12:05:00Z",
        "{}"
    )
    .execute(pool)
    .await
    .unwrap();
    
    result.last_insert_rowid()
}

mod health_tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let (app, _) = setup_test_app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["status"], "healthy");
        assert!(json["database"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let (app, _) = setup_test_app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }
}

mod conversation_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_conversations_empty() {
        let (app, _) = setup_test_app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/conversations")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["conversations"].as_array().unwrap().len(), 0);
        assert_eq!(json["total"], 0);
    }

    #[tokio::test]
    async fn test_list_conversations_with_data() {
        let (app, pool) = setup_test_app().await;
        
        // Insert test data
        insert_test_conversation(&pool).await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/conversations")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["conversations"].as_array().unwrap().len(), 1);
        assert_eq!(json["total"], 1);
        
        let conv = &json["conversations"][0];
        assert_eq!(conv["title"], "Test Conversation");
        assert_eq!(conv["provider"], "chatgpt");
    }

    #[tokio::test]
    async fn test_list_conversations_pagination() {
        let (app, pool) = setup_test_app().await;
        
        // Insert multiple conversations
        for i in 0..25 {
            sqlx::query!(
                r#"
                INSERT INTO conversations (id, title, provider, start_time, end_time)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                format!("test-conv-{}", i),
                format!("Test Conversation {}", i),
                "chatgpt",
                "2023-01-01T12:00:00Z",
                "2023-01-01T12:05:00Z"
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        
        // Test first page
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/conversations?page=1&per_page=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["conversations"].as_array().unwrap().len(), 10);
        assert_eq!(json["total"], 25);
        assert_eq!(json["page"], 1);
        assert_eq!(json["total_pages"], 3);
    }

    #[tokio::test]
    async fn test_get_conversation_not_found() {
        let (app, _) = setup_test_app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/conversations/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_performance_list_conversations() {
        let (app, pool) = setup_test_app().await;
        
        // Insert 1000 conversations
        for i in 0..1000 {
            sqlx::query!(
                r#"
                INSERT INTO conversations (id, title, provider, start_time, end_time)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                format!("test-conv-{}", i),
                format!("Test Conversation {}", i),
                "chatgpt",
                "2023-01-01T12:00:00Z",
                "2023-01-01T12:05:00Z"
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        
        let timer = performance::PerformanceTimer::new("List 1000 conversations");
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/conversations?per_page=50")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        timer.assert_under_ms(100);
    }
}

mod import_tests {
    use super::*;

    #[tokio::test]
    async fn test_import_chatgpt() {
        let (app, pool) = setup_test_app().await;
        
        let import_data = json!({
            "provider": "chatgpt",
            "data": chatgpt_samples::simple_conversation()
        });
        
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/import")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&import_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["imported_count"], 1);
        
        // Verify conversation was saved
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM conversations")
            .fetch_one(&pool)
            .await
            .unwrap();
        
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_import_multiple_providers() {
        let (app, pool) = setup_test_app().await;
        
        // Import from different providers
        let providers = vec![
            ("chatgpt", chatgpt_samples::simple_conversation()),
            ("claude", claude_samples::simple_conversation()),
            ("gemini", gemini_samples::takeout_format()),
            ("zed", zed_samples::conversation_json()),
        ];
        
        for (provider, data) in providers {
            let import_data = json!({
                "provider": provider,
                "data": data
            });
            
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/import")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&import_data).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            
            assert_eq!(response.status(), StatusCode::OK);
        }
        
        // Verify all were imported
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM conversations")
            .fetch_one(&pool)
            .await
            .unwrap();
        
        assert_eq!(count, 4);
    }

    #[tokio::test]
    async fn test_import_invalid_provider() {
        let (app, _) = setup_test_app().await;
        
        let import_data = json!({
            "provider": "invalid_provider",
            "data": {}
        });
        
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/import")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&import_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_import_duplicate_handling() {
        let (app, pool) = setup_test_app().await;
        
        let import_data = json!({
            "provider": "chatgpt",
            "data": chatgpt_samples::simple_conversation()
        });
        
        // Import same data twice
        for _ in 0..2 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/import")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&import_data).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            
            assert_eq!(response.status(), StatusCode::OK);
        }
        
        // Should only have one conversation (duplicate detection)
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM conversations")
            .fetch_one(&pool)
            .await
            .unwrap();
        
        assert_eq!(count, 1);
    }
}

mod search_tests {
    use super::*;

    #[tokio::test]
    async fn test_search_empty() {
        let (app, _) = setup_test_app().await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/search?q=test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["results"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_search_with_results() {
        let (app, pool) = setup_test_app().await;
        
        // Insert test data with searchable content
        let conv_id = insert_test_conversation(&pool).await;
        
        sqlx::query!(
            r#"
            INSERT INTO messages (conversation_id, role, content, timestamp)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            conv_id,
            "user",
            "Tell me about machine learning",
            "2023-01-01T12:00:00Z"
        )
        .execute(&pool)
        .await
        .unwrap();
        
        sqlx::query!(
            r#"
            INSERT INTO messages (conversation_id, role, content, timestamp)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            conv_id,
            "assistant",
            "Machine learning is a subset of artificial intelligence...",
            "2023-01-01T12:00:30Z"
        )
        .execute(&pool)
        .await
        .unwrap();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/search?q=machine+learning")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert!(json["results"].as_array().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_search_performance() {
        let (app, pool) = setup_test_app().await;
        
        // Insert 100 conversations with messages
        for i in 0..100 {
            let conv_id = sqlx::query!(
                r#"
                INSERT INTO conversations (id, title, provider, start_time, end_time)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                format!("test-conv-{}", i),
                format!("Conversation about topic {}", i),
                "chatgpt",
                "2023-01-01T12:00:00Z",
                "2023-01-01T12:05:00Z"
            )
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_rowid();
            
            // Add 10 messages per conversation
            for j in 0..10 {
                sqlx::query!(
                    r#"
                    INSERT INTO messages (conversation_id, role, content, timestamp)
                    VALUES (?1, ?2, ?3, ?4)
                    "#,
                    conv_id,
                    if j % 2 == 0 { "user" } else { "assistant" },
                    format!("This is message {} about topic {} with various keywords", j, i),
                    "2023-01-01T12:00:00Z"
                )
                .execute(&pool)
                .await
                .unwrap();
            }
        }
        
        let timer = performance::PerformanceTimer::new("Search 1000 messages");
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/search?q=topic")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        timer.assert_under_ms(100);
    }
}

mod export_tests {
    use super::*;

    #[tokio::test]
    async fn test_export_json() {
        let (app, pool) = setup_test_app().await;
        
        let conv_id = insert_test_conversation(&pool).await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/export/{}?format=json", conv_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["title"], "Test Conversation");
        assert_eq!(json["provider"], "chatgpt");
    }

    #[tokio::test]
    async fn test_export_markdown() {
        let (app, pool) = setup_test_app().await;
        
        let conv_id = insert_test_conversation(&pool).await;
        
        // Add messages
        sqlx::query!(
            r#"
            INSERT INTO messages (conversation_id, role, content, timestamp)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            conv_id,
            "user",
            "Hello!",
            "2023-01-01T12:00:00Z"
        )
        .execute(&pool)
        .await
        .unwrap();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/export/{}?format=markdown", conv_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let markdown = String::from_utf8(body.to_vec()).unwrap();
        
        assert!(markdown.contains("# Test Conversation"));
        assert!(markdown.contains("**User:**"));
        assert!(markdown.contains("Hello!"));
    }
}

mod stats_tests {
    use super::*;

    #[tokio::test]
    async fn test_stats_endpoint() {
        let (app, pool) = setup_test_app().await;
        
        // Insert test data
        insert_test_conversation(&pool).await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json["total_conversations"], 1);
        assert_eq!(json["providers"]["chatgpt"], 1);
    }
}