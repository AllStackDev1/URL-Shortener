use dotenv::dotenv;
use log::{info, warn, debug};
use serde::Deserialize;
use std::env;
use std::net::IpAddr;
use std::str::FromStr;


// Define a configuration error type
#[derive(Debug)]
pub enum ConfigError {
    EnvVarError(env::VarError),
    ParseError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::EnvVarError(e) => write!(f, "Environment variable error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<env::VarError> for ConfigError {
    fn from(error: env::VarError) -> Self {
        ConfigError::EnvVarError(error)
    }
}

// Result type for configuration functions
type ConfigResult<T> = Result<T, ConfigError>;

// Config struct that matches our environment variables
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub app: AppConfig,
}

// Server-specific configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub workers: usize,
}

// Application-specific configuration
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    pub environment: Environment,
    pub log_level: String,
}

// Environment enum for different deployment environments
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Testing,
    Production,
}

// Implement FromStr trait for Environment enum to enable parsing from string
impl FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "testing" | "test" => Ok(Environment::Testing),
            "production" | "prod" => Ok(Environment::Production),
            _ => Err(format!("Invalid environment: {}. Must be one of: development, testing, production", s)),
        }
    }
}

impl Config {
    // Load configuration from environment variables
    pub fn load() -> ConfigResult<Self> {
        // Load .env file if it exists
        match dotenv() {
            Ok(_) => debug!(".env file loaded successfully"),
            Err(e) => warn!("Could not load .env file: {}", e),
        }

        // Create the server config
        let server = ServerConfig {
            host: get_env_or_default("SERVER_HOST", "127.0.0.1")?,
            port: get_env_or_default("SERVER_PORT", "8000")?,
            workers: get_env_or_default("SERVER_WORKERS", "4")?,
        };

        // Get version from Cargo.toml or environment
        let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.1.0").to_string();
        
        // Create the app config
        let app = AppConfig {
            name: get_env_or_default("APP_NAME", "url-shortener")?,
            version: env::var("APP_VERSION").unwrap_or(version),
            environment: get_env_or_default("APP_ENVIRONMENT", "development")?,
            log_level: get_env_or_default("RUST_LOG", "info")?,
        };

        let config = Config { server, app };
        info!("Configuration loaded successfully");
        debug!("Loaded config: {:?}", config);

        Ok(config)
    }
}

// Helper function to get an env variable with a default value
fn get_env_or_default<T: std::str::FromStr>(key: &str, default: &str) -> ConfigResult<T> 
where
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(val) => val.parse::<T>().map_err(|e| {
            ConfigError::ParseError(format!("Could not parse {}: {}", key, e))
        }),
        Err(env::VarError::NotPresent) => {
            debug!("{} not set, using default: {}", key, default);
            default.parse::<T>().map_err(|e| {
                ConfigError::ParseError(format!("Could not parse default for {}: {}", key, e))
            })
        },
        Err(e) => Err(ConfigError::EnvVarError(e)),
    }
}