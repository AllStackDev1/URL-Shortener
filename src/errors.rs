use std::{fmt, error::Error as StdError};

use sqlx::Error as SqlxError;

use crate::config::ConfigError;

// Define our own error type for better error messages
pub enum AppError {
    Server(std::io::Error),
    Config(String),
    Logger(String),
}

impl std::fmt::Display for AppError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
          AppError::Server(e) => write!(f, "Server error: {}", e),
          AppError::Config(e) => write!(f, "Configuration error: {}", e),
          AppError::Logger(e) => write!(f, "Logger error: {}", e),
      }
  }
}

impl From<std::io::Error> for AppError {
  fn from(error: std::io::Error) -> Self {
      AppError::Server(error)
  }
}

impl From<ConfigError> for AppError {
  fn from(error: ConfigError) -> Self {
      AppError::Config(error.to_string())
  }
}

/// Error type for repository operations
#[derive(Debug)]
pub enum RepositoryError {
    /// Database connection or query errors
    Database(SqlxError),
    
    /// Entity not found
    NotFound(String),
    
    /// Unique constraint violation
    Conflict(String),
    
    /// Invalid input data
    InvalidData(String),
    
    /// Unexpected error
    Unexpected(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(err) => write!(f, "Database error: {}", err),
            Self::NotFound(msg) => write!(f, "Entity not found: {}", msg),
            Self::Conflict(msg) => write!(f, "Conflict error: {}", msg),
            Self::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            Self::Unexpected(msg) => write!(f, "Unexpected error: {}", msg),
        }
    }
}

impl StdError for RepositoryError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Database(err) => Some(err),
            _ => None,
        }
    }
}

// Convenient conversion from SQLx errors
impl From<SqlxError> for RepositoryError {
    fn from(err: SqlxError) -> Self {
        match err {
            SqlxError::RowNotFound => Self::NotFound("Resource not found".to_string()),
            // Map database-specific errors to more meaningful application errors
            SqlxError::Database(db_err) => {
                // PostgreSQL error codes for common constraints
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        // Unique violation
                        "23505" => return Self::Conflict("Resource already exists".to_string()),
                        // Foreign key violation
                        "23503" => return Self::InvalidData("Referenced resource does not exist".to_string()),
                        // Check constraint violation
                        "23514" => return Self::InvalidData("Data violates constraints".to_string()),
                        _ => {},
                    }
                }
                Self::Database(SqlxError::Database(db_err))
            },
            _ => Self::Database(err),
        }
    }
}

/// Error type for service operations
#[derive(Debug)]
pub enum ServiceError {
    /// Input validation failed
    ValidationError(String),

    /// Resource already exists or conflict occurred
    Conflict(String),

    /// Resource was not found
    NotFound(String),

    /// Unrecoverable internal error
    InternalError(String),

    /// Wrapped repository error
    Repository(RepositoryError),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::Conflict(msg) => write!(f, "Conflict: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::InternalError(msg) => write!(f, "Internal error: {}", msg),
            Self::Repository(err) => write!(f, "Repository error: {}", err),
        }
    }
}

impl StdError for ServiceError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Repository(err) => Some(err),
            _ => None,
        }
    }
}

impl From<RepositoryError> for ServiceError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::NotFound(msg) => Self::NotFound(msg),
            RepositoryError::Conflict(msg) => Self::Conflict(msg),
            RepositoryError::InvalidData(msg) => Self::ValidationError(msg),
            RepositoryError::Unexpected(msg) => Self::InternalError(msg),
            RepositoryError::Database(_) => Self::InternalError("A database error occurred".to_string()),
        }
    }
}