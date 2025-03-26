use std::io::Error as IoError;

use actix_web::{
    http::StatusCode, 
    HttpResponse, ResponseError,
};
use serde_json::json;
use thiserror::Error;

pub mod config;
pub mod repository;

pub use config::ConfigError;
pub use repository::RepositoryError;

#[derive(Debug, Error)]
pub enum AppError {
    // Service-level domain errors
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Conflict error: {0}")]
    Conflict(String),
    #[error("Not found error: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
    /* #[error("Unauthorized")]
    Unauthorized, */
    // Infrastructure/system errors
    #[error("Server error: {0}")]
    Server(#[from] IoError),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Logger error: {0}")]
    Logger(String),
}

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        AppError::Config(e.to_string())
    }
}

impl From<RepositoryError> for AppError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::NotFound(msg) => AppError::NotFound(msg),
            RepositoryError::Conflict(msg) => AppError::Conflict(msg),
            RepositoryError::InvalidData(msg) => AppError::Validation(msg),
            RepositoryError::Database(mgs) => AppError::Internal(mgs.to_string()),
        }
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        // Flatten field errors into a single string
        let message = errors
            .field_errors()
            .iter()
            .map(|(field, errs)| {
                let reasons = errs
                    .iter()
                    .map(|e| e.message.clone().unwrap_or_else(|| "invalid".into()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}: {}", field, reasons)
            })
            .collect::<Vec<_>>()
            .join("; ");
        AppError::Validation(message)
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            // AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Internal(_)
            | AppError::Server(_)
            | AppError::Config(_)
            | AppError::Logger(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error_string = self.to_string();
        let (error_type, message) = error_string
        .split_once(":")
        .map(|(t, m)| (t.trim(), m.trim()))
        .unwrap_or(("Error", "An error occurred"));

        let error_message = if message.is_empty() {
            "An error occurred"
        } else {
            message
        };
        
        let code = self.status_code().as_u16();
        HttpResponse::build(self.status_code()).json(json!({
            "type": error_type.to_uppercase(),
            "message": error_message,
            "status_code": code,
        }))
    }
}
