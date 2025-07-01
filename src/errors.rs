use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

/// Application error type
#[derive(Debug)]
pub enum AppError {
    /// Database errors
    Database(sqlx::Error),
    
    /// Not found errors
    NotFound(String),
    
    /// Bad request errors
    BadRequest(String),
    
    /// Internal server errors
    Internal(anyhow::Error),
    
    /// Import errors
    ImportError(String),
    
    /// Search errors
    SearchError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Internal(e) => write!(f, "Internal error: {}", e),
            AppError::ImportError(msg) => write!(f, "Import error: {}", msg),
            AppError::SearchError(msg) => write!(f, "Search error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred")
            }
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, msg.as_str())
            }
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg.as_str())
            }
            AppError::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::ImportError(msg) => {
                tracing::error!("Import error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str())
            }
            AppError::SearchError(msg) => {
                tracing::error!("Search error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.as_str())
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

/// Result type alias using AppError
pub type AppResult<T> = Result<T, AppError>;