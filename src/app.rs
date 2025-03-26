use std::time::Instant;

use actix_cors::Cors;
use actix_web::{
    http,
    middleware::{DefaultHeaders, Logger},
    web, App, HttpServer,
};

use env_logger::Env;
use log::{debug, error, info};

use crate::{
    config::{Config, Environment},
    db::{Database, DatabaseError},
    middleware::RequestLogger,
    routes,
    services,
    types::{Result as AppResult, AppState},
    AppError,
};

// Setup logging with custom format and configuration
fn setup_logging(config: &Config) -> Result<(), AppError> {
    // Configure log level based on environment and config
    let log_level = match config.app.environment {
        Environment::Development => config.app.log_level.clone(),
        Environment::Testing => "debug,actix_web=info".to_string(),
        Environment::Production => "info,actix_web=warn".to_string(),
    };

    let env = Env::default()
        .filter_or("RUST_LOG", log_level)
        .write_style_or("RUST_LOG_STYLE", "always");

    env_logger::try_init_from_env(env)
        .map_err(|e| AppError::Logger(format!("Failed to initialize logger: {}", e)))
}

pub async fn server() -> AppResult<()> {
    // Load application configuration
    let config = Config::load()?;

    // Setup enhanced logging based on configuration
    setup_logging(&config)?;

    // Capture start time for uptime calculation
    let start_time = Instant::now();

    // Log startup information
    info!(
        "Starting {} v{} in {:?} mode.",
        config.app.name, config.app.version, config.app.environment
    );

    if config.app.environment == Environment::Development {
        debug!("Debug logging enabled");
        debug!("Full configuration: {:?}", config);
    }

    // Determine if we should enable more verbose logging
    let enable_debug_logging = config.app.environment != Environment::Production;

    // Create a cloned config for the closure
    let app_config = config.clone();

    // Determine log format based on environment
    let log_format = if enable_debug_logging {
        // Simple format for production
        "%a \"%r\" %s %b %T"
    } else {
        // Detailed format for development/testing
        "%a \"%r\" %s %b %T \"%{Referer}i\" \"%{User-Agent}i\" %{X-Request-ID}i"
    };

    // Initialize database connection
    let db = match Database::connect(&config.db).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to initialize database: {}", e);

            // Provide helpful error messages based on error type
            match e {
                DatabaseError::ConnectionError(ref sqlx_err) => {
                    error!("Database connection error: {}", sqlx_err);
                    if let sqlx::Error::PoolTimedOut = sqlx_err {
                        error!("Hint: Check if the database server is running and accessible");
                    } else if let sqlx::Error::Database(db_err) = &sqlx_err {
                        error!(
                            "Database error: {} (code: {})",
                            db_err.message(),
                            db_err.code().unwrap_or_default()
                        );
                    }
                }
                DatabaseError::DatabaseNotFound(ref db_name) => {
                    error!("Database not found: {}", db_name);
                    error!(
                        "Hint: Create the database or enable create_database_if_missing in config"
                    );
                }
                DatabaseError::MigrationError(ref msg) => {
                    error!("Migration error: {}", msg);
                    error!("Hint: Check the migrations directory and migration files");
                }
                _ => {
                    error!("Other database error: {}", e);
                }
            }

            return Err(AppError::Server(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Database initialization failed: {}", e),
            )));
        }
    };

    // Get some information about the connected database
    if let Ok((db_name, db_version)) = db.get_db_info().await {
        info!("Connected to database: {} ({})", db_name, db_version);
    }

    // Create a shared database reference for shutdown handling
    let db_for_shutdown = db.clone();

    // Start the HTTP server
    let _server = HttpServer::new(move || {
        // Create a default CORS policy that is restrictive
        let cors = Cors::default()
            // Allow only your frontend origin in a production environment
            .allowed_origin("http://localhost:3000") // Replace with your frontend URL
            // For development environments, you might want to allow localhost with different ports
            .allowed_origin("http://127.0.0.1:3000")
            // Optionally allow development URLs conditionally
            .allowed_origin_fn(|origin, _req_head| {
                // In development, you might want to be more permissive
                if cfg!(debug_assertions) {
                    // Check if origin starts with http://localhost:
                    origin.as_bytes().starts_with(b"http://localhost:")
                } else {
                    // In production, be strict
                    false
                }
            })
            // Define which headers are allowed
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
            ])
            // Define which methods are allowed
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            // Allow credentials (cookies, authorization headers, TLS client certificates)
            .supports_credentials()
            // Set max age for preflight requests
            .max_age(3600); // 1 hour

        let app = App::new()
            // Register the CORS middleware
            .wrap(cors)
            .app_data(web::Data::new(AppState {
                start_time,
                db: db.clone(),
                version: app_config.app.version.clone(),
            }))
            // Make the full configuration available to handlers
            .app_data(web::Data::new(app_config.clone()))
            .wrap(Logger::new(log_format))
            // Add request tracking ID
            .wrap(DefaultHeaders::new().add(("X-Request-ID", uuid::Uuid::new_v4().to_string())))
            // Add middleware to log the beginning and end of each request (in debug mode)
            .wrap(RequestLogger::new(enable_debug_logging));

        // Configure routes
        app.configure(|cfg| {
                // Register services and routes 
                services::register(db.clone(), cfg);
                routes::configure_routes(cfg);
            }
        )
    })
    .workers(config.server.workers)
    .bind((config.server.host.to_string(), config.server.port))?
    .run();

    // Get the server handle to control shutdown
    let server_handle = _server.handle();

    // Spawn a task to handle graceful shutdown on signals
    tokio::spawn(async move {
        // Wait for SIGINT or SIGTERM
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for shutdown signal");
        info!("Shutdown signal received, starting graceful shutdown...");

        // Start graceful server shutdown
        server_handle.stop(true).await;
    });

    // Run the server
    let _ = _server.await;

    // Once the server has stopped, clean up the database connections
    info!("Web server stopped, cleaning up resources...");
    db_for_shutdown.shutdown().await;
    info!("All resources cleaned up, goodbye!");

    Ok(())
}
