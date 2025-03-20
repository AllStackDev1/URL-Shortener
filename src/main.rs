use std::process;

use log::{error, info};

mod app;
mod config;
mod error;
mod middleware;
mod models;
mod repositories;
mod routes;
mod services;
mod telemetry;
mod types;
mod utils;

use app::server;
use error::AppError;

#[actix_web::main]
async fn main() {
    // Run the server with error handling
    match server().await {
        Ok(_) => {
            info!("Server shutdown gracefully");
        },
        Err(AppError::Server(e)) => {
            error!("Server error: {}", e);
            process::exit(1);
        },
        Err(AppError::Config(e)) => {
            error!("Configuration error: {}", e);
            process::exit(2);
        },
        Err(AppError::Logger(_)) => {
            error!("Logger error");
            process::exit(3);
        }
    }
}
