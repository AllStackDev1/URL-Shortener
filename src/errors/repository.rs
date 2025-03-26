use thiserror::Error;
use sqlx::Error as SqlxError;

#[derive(Error, Debug)]
pub enum RepositoryError {
    /// Database connection or query errors
    #[error("Database error: {0}")]
    Database(SqlxError),

    /// Entity not found
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Unique constraint violation
    #[error("Conflict error: {0}")]
    Conflict(String),

    /// Invalid input data
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

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
                        "23503" => {
                            return Self::InvalidData(
                                "Referenced resource does not exist".to_string(),
                            )
                        }
                        // Check constraint violation
                        "23514" => {
                            return Self::InvalidData("Data violates constraints".to_string())
                        }
                        _ => {}
                    }
                }
                Self::Database(SqlxError::Database(db_err))
            }
            _ => Self::Database(err),
        }
    }
}
