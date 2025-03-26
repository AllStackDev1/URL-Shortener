mod shortened_url;

use actix_web::{web, HttpResponse, Responder};

use crate::{
    db::{DBHealthStatus, DatabaseHealth},
    handlers::{redirect_handler, ShortenedUrlServiceType},
    types::{AppState, HealthStatus, ResponsePayload, Result},
};

// Handler function for the root route "/"
async fn index_url() -> impl Responder {
    let welcome_message = ResponsePayload {
        status: 200,
        message: String::from("Welcome and have a great time!"),
    };

    // Return the struct as JSON
    HttpResponse::Ok().json(welcome_message)
}

// Handler function for the health check endpoint
async fn health_check_url(data: web::Data<AppState>) -> impl Responder {
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

// Redirect to original URL route handler
async fn redirect_url(
    path: web::Path<String>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    redirect_handler(path, service).await
}

// Configure all routes function
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Register routes from individual modules
    cfg.route("/", web::get().to(index_url))
        .route("/health", web::get().to(health_check_url))
        .route("/{code}", web::get().to(redirect_url))
        .configure(shortened_url::configure_routes);
}
