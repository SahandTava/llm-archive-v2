use sqlx::{SqliteConnection, Row};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::cache::{SearchCache, SearchResult};

/// Enhanced search with incremental results and smart ranking
pub struct EnhancedSearch {
    search_cache: Arc<SearchCache>,
    prepared_statements: Arc<RwLock<PreparedStatements>>,
}

struct PreparedStatements {
    fts_search: Option<String>,
    prefix_search: Option<String>,
    fuzzy_search: Option<String>,
}

impl EnhancedSearch {
    pub fn new(search_cache: Arc<SearchCache>) -> Self {
        Self {
            search_cache,
            prepared_statements: Arc::new(RwLock::new(PreparedStatements {
                fts_search: None,
                prefix_search: None,
                fuzzy_search: None,
            })),
        }
    }

    /// Incremental search that returns results as you type
    pub async fn incremental_search(
        &self,
        conn: &mut SqliteConnection,
        query: &str,
        limit: i32,
    ) -> Result<Vec<SearchResult>, String> {
        // Return cached results if available
        if let Some(cached) = self.search_cache.get_results(query).await {
            return Ok(cached);
        }

        let results = if query.len() < 3 {
            // For short queries, use prefix matching
            self.prefix_search(conn, query, limit).await?
        } else {
            // For longer queries, use FTS5
            self.fts_search(conn, query, limit).await?
        };

        // Cache the results
        self.search_cache.cache_results(query, results.clone()).await;

        Ok(results)
    }

    /// Full-text search using FTS5
    async fn fts_search(
        &self,
        conn: &mut SqliteConnection,
        query: &str,
        limit: i32,
    ) -> Result<Vec<SearchResult>, String> {
        let sql = r#"
            SELECT 
                c.id,
                c.title,
                snippet(messages_fts, 1, '<mark>', '</mark>', '...', 30) as snippet,
                rank as score
            FROM messages_fts
            JOIN conversations c ON c.id = messages_fts.conversation_id
            WHERE messages_fts MATCH ?
            ORDER BY rank
            LIMIT ?
        "#;

        let rows = sqlx::query(sql)
            .bind(query)
            .bind(limit)
            .fetch_all(conn)
            .await
            .map_err(|e| format!("FTS search failed: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|row| SearchResult {
                conversation_id: row.get("id"),
                title: row.get("title"),
                snippet: row.get("snippet"),
                score: row.get::<f32, _>("score").abs(), // SQLite FTS5 rank is negative
            })
            .collect())
    }

    /// Prefix search for short queries
    async fn prefix_search(
        &self,
        conn: &mut SqliteConnection,
        query: &str,
        limit: i32,
    ) -> Result<Vec<SearchResult>, String> {
        let pattern = format!("{}%", query);
        
        let sql = r#"
            SELECT DISTINCT
                c.id,
                c.title,
                substr(m.content, 1, 150) as snippet,
                1.0 as score
            FROM conversations c
            JOIN messages m ON c.id = m.conversation_id
            WHERE 
                c.title LIKE ? OR
                m.content LIKE ?
            ORDER BY 
                CASE 
                    WHEN c.title LIKE ? THEN 0
                    ELSE 1
                END,
                c.id DESC
            LIMIT ?
        "#;

        let rows = sqlx::query(sql)
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(conn)
            .await
            .map_err(|e| format!("Prefix search failed: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|row| SearchResult {
                conversation_id: row.get("id"),
                title: row.get("title"),
                snippet: row.get("snippet"),
                score: row.get("score"),
            })
            .collect())
    }

    /// Advanced search with DSL support
    pub async fn advanced_search(
        &self,
        conn: &mut SqliteConnection,
        dsl_query: &SearchDSL,
    ) -> Result<Vec<SearchResult>, String> {
        let mut sql = String::from(
            "SELECT DISTINCT c.id, c.title, substr(m.content, 1, 150) as snippet, 1.0 as score
             FROM conversations c
             JOIN messages m ON c.id = m.conversation_id
             WHERE 1=1"
        );
        
        let mut bindings = vec![];

        // Build dynamic query based on DSL
        if let Some(text) = &dsl_query.text {
            sql.push_str(" AND m.content LIKE ?");
            bindings.push(format!("%{}%", text));
        }

        if let Some(provider) = &dsl_query.provider {
            sql.push_str(" AND c.provider = ?");
            bindings.push(provider.clone());
        }

        if let Some(role) = &dsl_query.role {
            sql.push_str(" AND m.role = ?");
            bindings.push(role.clone());
        }

        if let Some(after) = dsl_query.after_timestamp {
            sql.push_str(" AND c.timestamp > ?");
            bindings.push(after.to_string());
        }

        if let Some(before) = dsl_query.before_timestamp {
            sql.push_str(" AND c.timestamp < ?");
            bindings.push(before.to_string());
        }

        sql.push_str(" ORDER BY c.id DESC LIMIT ?");
        bindings.push(dsl_query.limit.unwrap_or(50).to_string());

        // Execute dynamic query
        let mut query = sqlx::query(&sql);
        for binding in &bindings {
            query = query.bind(binding);
        }

        let rows = query
            .fetch_all(conn)
            .await
            .map_err(|e| format!("Advanced search failed: {}", e))?;

        Ok(rows
            .into_iter()
            .map(|row| SearchResult {
                conversation_id: row.get("id"),
                title: row.get("title"),
                snippet: row.get("snippet"),
                score: row.get("score"),
            })
            .collect())
    }
}

/// Search DSL for advanced queries
#[derive(Default)]
pub struct SearchDSL {
    pub text: Option<String>,
    pub provider: Option<String>,
    pub role: Option<String>,
    pub after_timestamp: Option<i64>,
    pub before_timestamp: Option<i64>,
    pub limit: Option<i32>,
}

impl SearchDSL {
    /// Parse DSL from string like "provider:chatgpt role:user after:2024-01-01"
    pub fn parse(query: &str) -> Self {
        let mut dsl = Self::default();
        let parts: Vec<&str> = query.split_whitespace().collect();
        let mut text_parts = vec![];

        for part in parts {
            if let Some((key, value)) = part.split_once(':') {
                match key {
                    "provider" => dsl.provider = Some(value.to_string()),
                    "role" => dsl.role = Some(value.to_string()),
                    "after" => {
                        // Parse date to timestamp
                        if let Ok(ts) = parse_date(value) {
                            dsl.after_timestamp = Some(ts);
                        }
                    }
                    "before" => {
                        if let Ok(ts) = parse_date(value) {
                            dsl.before_timestamp = Some(ts);
                        }
                    }
                    _ => text_parts.push(part),
                }
            } else {
                text_parts.push(part);
            }
        }

        if !text_parts.is_empty() {
            dsl.text = Some(text_parts.join(" "));
        }

        dsl
    }
}

fn parse_date(date_str: &str) -> Result<i64, String> {
    // Simple date parser - would use chrono in production
    // For now, assume YYYY-MM-DD format
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() == 3 {
        // Rough timestamp calculation
        let year: i32 = parts[0].parse().unwrap_or(2024);
        let month: i32 = parts[1].parse().unwrap_or(1);
        let day: i32 = parts[2].parse().unwrap_or(1);
        
        // Approximate timestamp (would use proper date library)
        let timestamp = ((year - 1970) as i64 * 365 * 24 * 60 * 60) +
                       ((month - 1) as i64 * 30 * 24 * 60 * 60) +
                       ((day - 1) as i64 * 24 * 60 * 60);
        Ok(timestamp)
    } else {
        Err("Invalid date format".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsl_parsing() {
        let query = "rust programming provider:chatgpt role:user after:2024-01-01";
        let dsl = SearchDSL::parse(query);
        
        assert_eq!(dsl.text, Some("rust programming".to_string()));
        assert_eq!(dsl.provider, Some("chatgpt".to_string()));
        assert_eq!(dsl.role, Some("user".to_string()));
        assert!(dsl.after_timestamp.is_some());
    }
}