use std::time::Duration;

use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use sqlx::migrate::MigrateDatabase;
use sqlx::{
    postgres::{PgPool, PgPoolOptions},
    Postgres,
};
use thiserror::Error;

use crate::config::DatabaseConfig;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    ConnectionError(#[from] sqlx::Error),

    #[error("Database migration error: {0}")]
    MigrationError(String),

    #[error("Database not found: {0}")]
    DatabaseNotFound(String),

    #[error("Failed to create database: {0}")]
    DatabaseCreationFailed(String),
}

pub type DbResult<T> = Result<T, DatabaseError>;

/// Represents an established database connection pool
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

/// Database health status
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DBHealthStatus {
    Healthy,
    Unhealthy,
}

/// Database information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DbInfo {
    pub name: Option<String>,
    pub version: Option<String>,
}

/// Complete database health check result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseHealth {
    pub status: DBHealthStatus,
    pub response_time_ms: u64,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_info: Option<DbInfo>,
}

impl Database {
    /// Create a new database connection pool from configuration
    pub async fn connect(config: &DatabaseConfig) -> DbResult<Self> {
        info!("Initializing database connection");
        debug!(
            "Database configuration: max_conn={}, min_conn={}, timeout={}s",
            config.max_connections, config.min_connections, config.connect_timeout_seconds
        );

        // First, check if the database exists
        if !config.skip_db_exists_check {
            Self::ensure_database_exists(config).await?;
        }

        // Create the connection pool
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .connect(&config.url)
            .await
            .map_err(|e| {
                warn!("Failed to connect to database: {}", e);
                DatabaseError::ConnectionError(e)
            })?;

        info!("Successfully connected to database");

        // Run migrations if enabled
        if config.use_migrations {
            Self::run_migrations(&pool).await?;
        }

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check if the database connection is healthy
    pub async fn health_check(&self) -> DbResult<DatabaseHealth> {
        // Measure query execution time
        let start = std::time::Instant::now();

        // Try a simple query to verify the connection is working
        let result = sqlx::query("SELECT 1 as result")
            .fetch_one(self.get_pool())
            .await;

        let elapsed = start.elapsed();

        match result {
            Ok(_) => {
                // Optionally get additional database information
                let db_info = match sqlx::query_as!(
                    DbInfo,
                    "SELECT current_database() as name, version() as version",
                )
                .fetch_one(self.get_pool())
                .await
                {
                    Ok(info) => Some(info),
                    Err(_) => None,
                };

                Ok(DatabaseHealth {
                    status: DBHealthStatus::Healthy,
                    response_time_ms: elapsed.as_millis() as u64,
                    message: None,
                    db_info,
                })
            }
            Err(e) => Ok(DatabaseHealth {
                status: DBHealthStatus::Unhealthy,
                response_time_ms: elapsed.as_millis() as u64,
                message: Some(format!("Database query failed: {}", e)),
                db_info: None,
            }),
        }
    }

    /// Get database server information
    pub async fn get_db_info(&self) -> DbResult<(String, String)> {
        let row = sqlx::query!(r#"SELECT current_database() as db_name, version() as db_version"#)
            .fetch_one(&self.pool)
            .await
            .map_err(DatabaseError::ConnectionError)?;

        Ok((
            row.db_name.expect("Database name not found"),
            row.db_version.expect("Database version not found"),
        ))
    }

    /// Ensure the target database exists, create it if necessary
    async fn ensure_database_exists(config: &DatabaseConfig) -> DbResult<()> {
        // Extract database name from connection URL
        let url = &config.url;
        let db_name = extract_db_name_from_url(url).ok_or_else(|| {
            DatabaseError::DatabaseNotFound(
                "Could not extract database name from connection string".to_string(),
            )
        })?;

        debug!("Checking if database '{}' exists", db_name);

        // Check if database exists
        let db_exists = Postgres::database_exists(url)
            .await
            .map_err(DatabaseError::ConnectionError)?;

        if !db_exists {
            if config.create_database_if_missing {
                info!("Database '{}' does not exist, creating it", db_name);

                // Get base URL (without database name)
                let base_url = url
                    .replace(&format!("/{}", db_name), "")
                    .replace(&format!("/{}/", db_name), "/postgres/");

                debug!(
                    "Using base connection for database creation: {}",
                    base_url.replace(
                        |c: char| c != ':' && c != '/' && !c.is_ascii_alphabetic(),
                        "*"
                    )
                );

                // Create database
                if let Err(err) = Postgres::create_database(&base_url).await {
                    return Err(DatabaseError::DatabaseCreationFailed(format!(
                        "Failed to create database '{}': {}",
                        db_name, err
                    )));
                }

                info!("Successfully created database '{}'", db_name);
            } else {
                return Err(DatabaseError::DatabaseNotFound(format!(
                    "Database '{}' does not exist",
                    db_name
                )));
            }
        } else {
            debug!("Database '{}' exists", db_name);
        }

        Ok(())
    }

    /// Run database migrations
    async fn run_migrations(pool: &PgPool) -> DbResult<()> {
        info!("Running database migrations");

        // can we check if a migration file has been modified, if so, drop the database and recreate it only in development
        

        match sqlx::migrate!("./migrations").run(pool).await {
            Ok(_) => {
                info!("Database migrations completed successfully");
                Ok(())
            }
            Err(e) => {
                warn!("Database migration error: {}", e);
                Err(DatabaseError::MigrationError(e.to_string()))
            }
        }
    }

    /// Gracefully close the database connection pool
    pub async fn shutdown(&self) {
        // Log the start of the shutdown process
        log::info!("Shutting down database connection pool...");

        // Get current connection count for reporting
        let used_connections = self.pool.size();
        let idle_connections = self.pool.num_idle();

        // Close the connection pool
        self.pool.close().await;

        // Log success with connection stats
        log::info!("Database connection pool successfully closed. Stats: {} active, {} idle connections released", 
            used_connections, idle_connections);
    }
}

/// Extract database name from a PostgreSQL connection string
fn extract_db_name_from_url(url: &str) -> Option<String> {
    // Split by '/' to get the path part
    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() < 4 {
        return None;
    }

    // The database name is the fourth part, potentially with query params
    let db_with_params = parts[3];

    // Remove query parameters if present
    let db_name = db_with_params.split('?').next()?;

    Some(db_name.to_string())
}
