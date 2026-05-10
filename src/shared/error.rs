use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid configuration: {0}")]
    Config(String),
    #[error("startup failed: {0}")]
    Startup(String),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    DateTime(#[from] chrono::ParseError),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Config(message)
            | AppError::Startup(message)
            | AppError::Upstream(message) => (StatusCode::SERVICE_UNAVAILABLE, message),
            AppError::DateTime(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            AppError::Io(err) => (StatusCode::SERVICE_UNAVAILABLE, err.to_string()),
            AppError::Serde(err) => (StatusCode::SERVICE_UNAVAILABLE, err.to_string()),
            AppError::Join(err) => (StatusCode::SERVICE_UNAVAILABLE, err.to_string()),
        };
        (status, message).into_response()
    }
}
