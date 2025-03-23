use std::time::Instant;

use actix_web::{
    middleware::{DefaultHeaders, Logger},
    web, App, HttpServer,
};
use env_logger::Env;
use log::{debug, error, info};

use crate::{
    config::{Config, Environment},
    db::{Database, DatabaseError},
    error::AppError,
    routes,
    types::AppState,
};

// Custom result type for the application
pub type AppResult<T> = Result<T, AppError>;

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
    info!("Starting {} v{}", config.app.name, config.app.version);
    info!("Environment: {:?}", config.app.environment);
    info!(
        "Binding to {}:{} with {} workers",
        config.server.host, config.server.port, config.server.workers
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
    info!("Initializing database connection");
    let db = match Database::connect(&config.db).await {
        Ok(db) => {
            info!("Database connected successfully");
            db
        }
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
        let app = App::new()
            .app_data(web::Data::new(AppState {
                start_time,
                db: db.clone(),
                version: app_config.app.version.clone(),
            }))
            // Make the full configuration available to handlers
            .app_data(web::Data::new(app_config.clone()))
            .wrap(Logger::new(log_format))
            // Add request tracking ID
            .wrap(DefaultHeaders::new().add(("X-Request-ID", uuid::Uuid::new_v4().to_string())));

        // Add middleware to log the beginning and end of each request (in debug mode)
        // if enable_debug_logging {
        //     app = app.wrap_fn(move |req, srv| {
        //         let path = req.path().to_owned();
        //         let method = req.method().clone();

        //         debug!("Processing request: {} {}", method, path);

        //         let fut = srv.call(req);
        //         async move {
        //             let res = fut.await?;
        //             debug!("Response: {} {} - status: {}", method, path, res.status());
        //             Ok(res)
        //         }
        //     });
        // }

        // Configure routes
        app.configure(routes::configure_routes)
    })
    .workers(config.server.workers)
    .bind((config.server.host.to_string(), config.server.port))?
    .run();
    // .await?;

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
