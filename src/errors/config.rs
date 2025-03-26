use std::env::VarError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    /// Represents an error related to environment variables.
    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] VarError),

    /// Represents an error related to parsing configuration data.
    #[error("Parse error: {0}")]
    ParseError(String),
}
