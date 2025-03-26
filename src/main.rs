use std::process;

use log::error;

mod app;
mod config;
mod db;
mod errors;
mod handlers;
mod middleware;
mod models;
mod repositories;
mod routes;
mod services;
mod telemetry;
mod types;
mod utils;
mod validations;

use errors::AppError;

#[actix_web::main]
async fn main() {
    // Run the server with error handling for critical failures
    if let Err(err) = app::server().await {
        match err {
            AppError::Server(e) => {
                error!("Critical server error: {}", e);
                process::exit(1);
            }
            AppError::Config(e) => {
                error!("Critical configuration error: {}", e);
                process::exit(2);
            }
            AppError::Logger(e) => {
                error!("Critical logger error: {}", e);
                process::exit(3);
            }
            _ => {
                // Log unexpected errors, but don't exit
                error!("Unexpected error: {}", err);
            }
        }
    }
}
