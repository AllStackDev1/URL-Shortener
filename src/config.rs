use std::{env, net::IpAddr, str::FromStr};

use dotenvy::dotenv;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::errors::ConfigError;

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
            _ => Err(format!(
                "Invalid environment: {}. Must be one of: development, testing, production",
                s
            )),
        }
    }
}

// Result type for configuration functions
type ConfigResult<T> = Result<T, ConfigError>;

// Database Config
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub use_migrations: bool,
    pub skip_db_exists_check: bool,
    pub connect_timeout_seconds: u64,
    pub create_database_if_missing: bool,
}

// Config struct that matches our environment variables
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub app: AppConfig,
    pub db: DatabaseConfig,
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
        let version = option_env!("CARGO_PKG_VERSION")
            .unwrap_or("0.1.0")
            .to_string();

        // Create the app config
        let app = AppConfig {
            name: get_env_or_default("APP_NAME", "url-shortener")?,
            version: env::var("APP_VERSION").unwrap_or(version),
            environment: get_env_or_default("APP_ENVIRONMENT", "development")?,
            log_level: get_env_or_default("RUST_LOG", "info")?,
        };

        // Database config
        let db = DatabaseConfig {
            url: get_env_or_default(
                "DATABASE_URL",
                "postgres://MrCEO:postgres@localhost:5432/kick-shortener",
            )?,
            max_connections: get_env_or_default("DATABASE_MAX_CONNECTIONS", "10")?,
            min_connections: get_env_or_default("DATABASE_MIN_CONNECTIONS", "5")?,
            connect_timeout_seconds: get_env_or_default("DATABASE_CONNECT_TIMEOUT_SECONDS", "5")?,
            skip_db_exists_check: get_env_or_default("DATABASE_SKIP_DB_EXISTS_CHECK", "false")?,
            use_migrations: get_env_or_default("DATABASE_USE_MIGRATIONS", "true")?,
            create_database_if_missing: get_env_or_default(
                "DATABASE_CREATE_DATABASE_IF_MISSING",
                "true",
            )?,
        };

        let config = Config { db, app, server };
        info!("Configuration loaded successfully");
        debug!("Loaded config: {:?}", config);

        Ok(config)
    }
}

/// Helper function to get an env variable with a default value
fn get_env_or_default<T: std::str::FromStr>(key: &str, default: &str) -> ConfigResult<T>
where
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(val) => val
            .parse::<T>()
            .map_err(|e| ConfigError::ParseError(format!("Could not parse {}: {}", key, e))),
        Err(env::VarError::NotPresent) => {
            debug!("{} not set, using default: {}", key, default);
            default.parse::<T>().map_err(|e| {
                ConfigError::ParseError(format!("Could not parse default for {}: {}", key, e))
            })
        }
        Err(e) => Err(ConfigError::EnvVarError(e)),
    }
}


// pub struct CorsConfig {
//     pub allowed_origins: Vec<String>,
//     pub allowed_methods: Vec<String>,
//     pub max_age: u32,
// }

// pub struct Config {
//     pub database: DbConfig,
//     pub server: ServerConfig,
//     pub cors: CorsConfig,
//     // Other config sections
// }

// // Then in your app configuration
// let cors = Cors::default();
// let cors = config.cors.allowed_origins.iter().fold(cors, |cors, origin| {
//     cors.allowed_origin(origin)
// });
// let cors = cors
//     .allowed_methods(config.cors.allowed_methods.iter().map(|m| m.as_str()))
//     .max_age(config.cors.max_age);

// src/config/cors.rs
// use actix_cors::Cors;
// use actix_web::http;

// // Enum to represent different environments
// pub enum Environment {
//     Development,
//     Staging,
//     Production,
// }

// impl Environment {
//     pub fn from_str(s: &str) -> Self {
//         match s.to_lowercase().as_str() {
//             "development" | "dev" => Self::Development,
//             "staging" | "stage" => Self::Staging,
//             "production" | "prod" => Self::Production,
//             _ => Self::Development, // Default to development
//         }
//     }
// }

// // Configure CORS based on the environment
// pub fn configure_cors(environment: Environment) -> Cors {
//     match environment {
//         Environment::Development => {
//             // In development, be more permissive
//             Cors::default()
//                 .allow_any_origin()  // Allow any origin in development
//                 .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
//                 .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT, http::header::CONTENT_TYPE])
//                 .supports_credentials()
//                 .max_age(3600)
//         },
//         Environment::Staging => {
//             // In staging, allow specific origins
//             Cors::default()
//                 .allowed_origin("https://staging.yourapp.com")
//                 .allowed_origin("http://localhost:3000")  // For testing
//                 .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
//                 .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT, http::header::CONTENT_TYPE])
//                 .supports_credentials()
//                 .max_age(3600)
//         },
//         Environment::Production => {
//             // In production, be very restrictive
//             Cors::default()
//                 .allowed_origin("https://yourapp.com")
//                 .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
//                 .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT, http::header::CONTENT_TYPE])
//                 .supports_credentials()
//                 .max_age(3600)
//         }
//     }
// }