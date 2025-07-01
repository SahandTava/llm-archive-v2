// parsers/common.rs - Common utilities for all parsers

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use mime_guess::from_path;
use regex::Regex;
use serde_json::Value;
use std::path::Path;

use super::{ParserError, ParserResult};

/// Parse timestamp from various formats
pub fn parse_timestamp(input: &str) -> ParserResult<DateTime<Utc>> {
    // Try ISO 8601 with Z suffix
    if let Ok(dt) = DateTime::parse_from_rfc3339(&input.replace("Z", "+00:00")) {
        return Ok(dt.with_timezone(&Utc));
    }
    
    // Try ISO 8601 without timezone
    if let Ok(dt) = NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(Utc.from_utc_datetime(&dt));
    }
    
    // Try common formats
    let formats = [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
        "%m/%d/%Y %H:%M:%S",
    ];
    
    for format in formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(input, format) {
            return Ok(Utc.from_utc_datetime(&dt));
        }
    }
    
    Err(ParserError::InvalidTimestamp(input.to_string()))
}

/// Parse timestamp from numeric value (unix timestamp in seconds or milliseconds)
pub fn parse_timestamp_numeric(value: f64) -> DateTime<Utc> {
    // If value is very large, assume milliseconds
    if value > 1e11 {
        DateTime::from_timestamp_millis(value as i64).unwrap_or_else(|| Utc::now())
    } else {
        DateTime::from_timestamp(value as i64, 0).unwrap_or_else(|| Utc::now())
    }
}

/// Parse timestamp from JSON value (string or number)
pub fn parse_timestamp_json(value: &Value) -> ParserResult<DateTime<Utc>> {
    match value {
        Value::String(s) => parse_timestamp(s),
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(parse_timestamp_numeric(f))
            } else {
                Err(ParserError::InvalidTimestamp(value.to_string()))
            }
        }
        _ => Err(ParserError::InvalidTimestamp(value.to_string())),
    }
}

/// Extract text content from various JSON structures
pub fn extract_text_content(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Object(obj) => {
            // Try common text field names
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                return text.to_string();
            }
            if let Some(content) = obj.get("content") {
                return extract_text_content(content);
            }
            if let Some(parts) = obj.get("parts").and_then(|v| v.as_array()) {
                return extract_text_from_parts(parts);
            }
            String::new()
        }
        Value::Array(arr) => extract_text_from_parts(arr),
        _ => String::new(),
    }
}

/// Extract text from array of content parts
pub fn extract_text_from_parts(parts: &[Value]) -> String {
    parts
        .iter()
        .map(|part| {
            if let Value::String(s) = part {
                s.clone()
            } else if let Value::Object(obj) = part {
                if obj.get("type").and_then(|v| v.as_str()) == Some("text") {
                    obj.get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Detect MIME type from file path
pub fn detect_mime_type(path: &Path) -> Option<String> {
    from_path(path).first_raw().map(|s| s.to_string())
}

/// Extract media references from text using regex
pub fn find_media_references(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    
    // Markdown image/link pattern
    let md_pattern = Regex::new(r"\[.*?\]\((.*?)\)").unwrap();
    for cap in md_pattern.captures_iter(text) {
        if let Some(url) = cap.get(1) {
            refs.push(url.as_str().to_string());
        }
    }
    
    // Direct URL pattern for common media extensions
    let url_pattern = Regex::new(
        r"https?://\S+\.(?:jpg|jpeg|png|gif|webp|mp3|mp4|wav|pdf|txt|md)\b"
    ).unwrap();
    for mat in url_pattern.find_iter(text) {
        refs.push(mat.as_str().to_string());
    }
    
    refs
}

/// Sanitize and validate conversation title
pub fn sanitize_title(title: &str, default: &str) -> String {
    let cleaned = title.trim();
    if cleaned.is_empty() {
        default.to_string()
    } else {
        // Truncate very long titles
        if cleaned.len() > 200 {
            format!("{}...", &cleaned[..197])
        } else {
            cleaned.to_string()
        }
    }
}

/// Generate a unique conversation ID from various inputs
pub fn generate_conversation_id(provider: &str, file_path: &Path, index: usize) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    provider.hash(&mut hasher);
    file_path.to_string_lossy().hash(&mut hasher);
    index.hash(&mut hasher);
    
    format!("{}_{}_{:x}", provider, index, hasher.finish())
}

/// Check if a JSON file is valid
pub fn is_valid_json_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    
    match std::fs::File::open(path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            serde_json::from_reader::<_, Value>(reader).is_ok()
        }
        Err(_) => false,
    }
}

/// Extract metadata from JSON object, preserving only safe types
pub fn extract_metadata(obj: &serde_json::Map<String, Value>) -> serde_json::Map<String, Value> {
    let mut metadata = serde_json::Map::new();
    
    // List of keys to exclude from metadata (already extracted elsewhere)
    let exclude_keys = [
        "messages", "chat_messages", "transcript", "mapping",
        "title", "name", "created_at", "updated_at", "uuid",
        "conversation_id", "id", "text", "content", "role",
        "sender", "timestamp", "create_time", "update_time"
    ];
    
    for (key, value) in obj {
        if exclude_keys.contains(&key.as_str()) {
            continue;
        }
        
        // Only include simple types in metadata
        match value {
            Value::String(_) | Value::Number(_) | Value::Bool(_) => {
                metadata.insert(key.clone(), value.clone());
            }
            Value::Array(arr) if arr.len() < 10 => {
                // Only include small arrays of simple types
                if arr.iter().all(|v| matches!(v, Value::String(_) | Value::Number(_) | Value::Bool(_))) {
                    metadata.insert(key.clone(), value.clone());
                }
            }
            _ => {} // Skip complex objects and large arrays
        }
    }
    
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_timestamp_parsing() {
        // ISO 8601 with Z
        let ts = parse_timestamp("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:30:00+00:00");
        
        // ISO 8601 without timezone
        let ts = parse_timestamp("2024-01-15T10:30:00").unwrap();
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:30:00+00:00");
        
        // Common format
        let ts = parse_timestamp("2024-01-15 10:30:00").unwrap();
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:30:00+00:00");
    }
    
    #[test]
    fn test_numeric_timestamp() {
        // Unix seconds
        let ts = parse_timestamp_numeric(1705316400.0);
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:20:00+00:00");
        
        // Unix milliseconds
        let ts = parse_timestamp_numeric(1705316400000.0);
        assert_eq!(ts.to_rfc3339(), "2024-01-15T10:20:00+00:00");
    }
    
    #[test]
    fn test_text_extraction() {
        // Simple string
        let json = serde_json::json!("Hello world");
        assert_eq!(extract_text_content(&json), "Hello world");
        
        // Object with text field
        let json = serde_json::json!({"text": "Hello world"});
        assert_eq!(extract_text_content(&json), "Hello world");
        
        // Array of parts
        let json = serde_json::json!([
            {"type": "text", "text": "Hello"},
            {"type": "text", "text": "world"}
        ]);
        assert_eq!(extract_text_content(&json), "Hello\nworld");
    }
    
    #[test]
    fn test_media_references() {
        let text = "Check out this image: ![alt](https://example.com/image.png) and [link](doc.pdf)";
        let refs = find_media_references(text);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"https://example.com/image.png".to_string()));
        assert!(refs.contains(&"doc.pdf".to_string()));
    }
}