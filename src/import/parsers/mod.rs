/// Native Rust parsers for various LLM export formats
/// These will gradually replace the Python parsers for better performance

pub mod chatgpt;
pub mod claude;

// Common parsing utilities
use serde_json::Value;
use chrono::{DateTime, Utc};

/// Parse a timestamp from various formats
pub fn parse_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::String(s) => {
            // Try RFC3339 first
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Some(dt.with_timezone(&Utc));
            }
            
            // Try Unix timestamp
            if let Ok(ts) = s.parse::<i64>() {
                return DateTime::from_timestamp(ts, 0);
            }
            
            // Try float Unix timestamp
            if let Ok(ts) = s.parse::<f64>() {
                return DateTime::from_timestamp(ts as i64, ((ts.fract() * 1_000_000_000.0) as u32));
            }
            
            None
        }
        Value::Number(n) => {
            if let Some(ts) = n.as_i64() {
                DateTime::from_timestamp(ts, 0)
            } else if let Some(ts) = n.as_f64() {
                DateTime::from_timestamp(ts as i64, ((ts.fract() * 1_000_000_000.0) as u32))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract string value from JSON
pub fn get_string(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(|s| s.to_string())
}

/// Extract f32 value from JSON
pub fn get_f32(value: &Value, key: &str) -> Option<f32> {
    match value.get(key)? {
        Value::Number(n) => n.as_f64().map(|f| f as f32),
        _ => None,
    }
}

/// Extract i32 value from JSON
pub fn get_i32(value: &Value, key: &str) -> Option<i32> {
    match value.get(key)? {
        Value::Number(n) => n.as_i64().map(|i| i as i32),
        _ => None,
    }
}