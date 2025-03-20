
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
