use actix_web::{web, HttpResponse, Responder};

use crate::{db::{DatabaseHealth, DBHealthStatus}, types::{AppState, HealthStatus, ResponsePayload}};

// Handler function for the root route "/"
async fn index() -> impl Responder {
    let welcome_message = ResponsePayload {
        status: 200,
        message: String::from("Welcome and have a great time!"),
    };

    // Return the struct as JSON
    HttpResponse::Ok().json(welcome_message)
}

// Handler function for the health check endpoint
async fn health_check(data: web::Data<AppState>) -> impl Responder {
    // Calculate uptime in seconds
    let uptime = data.start_time.elapsed().as_secs();

    // Check database health
    let db_health = match data.db.health_check().await {
        Ok(health) => health,
        Err(e) => DatabaseHealth {
            status: DBHealthStatus::Unhealthy,
            response_time_ms: 0,
            message: Some(format!("Error performing health check: {}", e)),
            db_info: None,
        },
    };

    let status = HealthStatus {
        status: String::from("OK"),
        db_health: Some(db_health),
        version: data.version.clone(),
        uptime_seconds: uptime,
    };

    // Return the status as JSON
    HttpResponse::Ok().json(status)
}

// Configure all routes function
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Register routes from individual modules
    cfg.route("/", web::get().to(index));
    cfg.route("/health", web::get().to(health_check));
    // index::configure(cfg);
    // health_check::configure(cfg);
}
