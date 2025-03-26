// You're thinking ahead wisely! For a URL shortener, the redirect endpoint (/{code}) is definitely public-facing and could be subject to high traffic or potential abuse. Let's implement a simple but effective rate limiting solution specifically for that route.

// Basic Rate Limiting for Public Routes
// Here's a lightweight approach using Actix Web middleware:

// 1. Add dependencies to Cargo.toml
// toml
// [
// dependencies
// ]
// # Existing dependencies
// actix-governor = "0.4.1"  # Rate limiting middleware for Actix Web
// 2. Implement simple per-IP rate limiting
// rust
// use actix_governor::{Governor, GovernorConfigBuilder};
// use std::time::Duration;

// // In your routes configuration or app setup
// fn configure_routes(cfg: &mut web::ServiceConfig) {
//     // Create a rate limiter for public routes
//     // Limit to 60 requests per minute per IP address
//     let public_route_limiter = Governor::new(
//         GovernorConfigBuilder::default()
//             .per_second(1)          // 1 request per second base rate
//             .burst_size(60)         // Allow bursts of up to 60 requests
//             .finish()
//             .unwrap()
//     );

//     // API routes (less restrictive or no rate limiting)
//     cfg.service(
//         web::scope("/api")
//             .service(
//                 web::scope("/urls")
//                     .route("", web::post().to(url_handlers::create_url))
//                     .route("/search", web::get().to(url_handlers::search_urls))
//                     .route("/{id}", web::get().to(url_handlers::get_url_by_id))
//                     .route("/{id}", web::put().to(url_handlers::update_url))
//                     .route("/{id}", web::delete().to(url_handlers::delete_url))
//             )
//     );

//     // Frontend UI route
//     cfg.service(web::resource("/").route(web::get().to(index_handler)));

//     // Redirect handler with rate limiting
//     // This is the public-facing route that could get hit a lot
//     cfg.service(
//         web::resource("/{code}")
//             .wrap(public_route_limiter)  // Apply rate limiting to this route only
//             .route(web::get().to(redirect_handler))
//     );
// }
// 3. Adding some customization for better user experience
// Let's improve the rate limiter with custom responses and headers:

// rust
// use actix_governor::{Governor, GovernorConfigBuilder, GovernorConfig};
// use actix_web::{HttpResponse, ResponseError, http::StatusCode};
// use std::fmt;

// // Custom error for rate limit exceeded
// #[derive(Debug)]
// struct RateLimitExceededError;

// impl fmt::Display for RateLimitExceededError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "Rate limit exceeded. Please try again later.")
//     }
// }

// impl ResponseError for RateLimitExceededError {
//     fn status_code(&self) -> StatusCode {
//         StatusCode::TOO_MANY_REQUESTS
//     }

//     fn error_response(&self) -> HttpResponse {
//         HttpResponse::TooManyRequests()
//             .content_type("text/html; charset=utf-8")
//             .body(
//                 r#"
// <html>
// <head>
// <title>
// Rate Limit Exceeded

// </title>
// </head>
// <body>
// <h1>
// 429 - Rate Limit Exceeded

// </h1>
// <p>
// You have made too many requests in a short period. Please try again later.

// </p>
// </body>
// </html>
//             "#
//         )
// }
// }

// // Custom configuration builder
// fn create_rate_limiter() -> Governor {
// // Create a custom config with error handling
// let config = GovernorConfigBuilder::default()
// .per_second(1)           // Base rate: 1 request per second
// .burst_size(60)          // Allow bursts of up to 60 requests
// .key_extractor(|req| {   // Extract key from request - we'll use IP address
// let connection_info = req.connection_info();
// let ip = connection_info.realip_remote_addr()
// .unwrap_or("unknown")
// .to_string();

//         // You could log the rate limiting here if desired
//         // log::debug!("Rate limiting request from IP: {}", ip);
        
//         Some(ip)
//     })
//     .error_handler(|_| {     // Custom error response
//         Box::new(RateLimitExceededError)
//     })
//     .finish()
//     .unwrap();

// Governor::new(config)
// }


// ### 4. Making it more configurable

// For better flexibility, let's make the rate limits configurable:

// ```rust
// // In your configuration module
// pub struct RateLimitConfig {
//     pub enabled: bool,
//     pub requests_per_second: u32,
//     pub burst_size: u32,
// }

// impl Default for RateLimitConfig {
//     fn default() -> Self {
//         Self {
//             enabled: true,
//             requests_per_second: 1,
//             burst_size: 60,
//         }
//     }
// }

// // In main application config
// pub struct Config {
//     // Other config sections...
//     pub rate_limit: RateLimitConfig,
// }

// // In your routes configuration
// fn configure_routes(cfg: &mut web::ServiceConfig, config: web::Data<Config>) {
//     if config.rate_limit.enabled {
//         let rate_limiter = Governor::new(
//             GovernorConfigBuilder::default()
//                 .per_second(config.rate_limit.requests_per_second)
//                 .burst_size(config.rate_limit.burst_size)
//                 .finish()
//                 .unwrap()
//         );
        
//         // Apply rate limiter to redirect route
//         cfg.service(
//             web::resource("/{code}")
//                 .wrap(rate_limiter)
//                 .route(web::get().to(redirect_handler))
//         );
//     } else {
//         // No rate limiting
//         cfg.service(
//             web::resource("/{code}")
//                 .route(web::get().to(redirect_handler))
//         );
//     }
    
//     // Other routes...
// }
// 5. More sophisticated rate limiting (optional)
// If you need more sophisticated rate limiting later, consider:

// rust
// use std::collections::HashMap;
// use std::sync::Arc;
// use tokio::sync::Mutex;
// use chrono::{Utc, DateTime, Duration};

// // A simple in-memory rate limiter for demonstration
// struct InMemoryRateLimiter {
//     // Maps IP address to list of request timestamps
//     requests: Mutex<HashMap<String, Vec<DateTime<Utc>>>>,
//     window_seconds: i64,
//     max_requests: usize,
// }

// impl InMemoryRateLimiter {
//     fn new(window_seconds: i64, max_requests: usize) -> Self {
//         Self {
//             requests: Mutex::new(HashMap::new()),
//             window_seconds,
//             max_requests,
//         }
//     }
    
//     async fn check(&self, ip: &str) -> bool {
//         let now = Utc::now();
//         let window_start = now - Duration::seconds(self.window_seconds);
        
//         let mut requests = self.requests.lock().await;
        
//         // Get or create entry for this IP
//         let timestamps = requests.entry(ip.to_string()).or_insert_with(Vec::new);
        
//         // Remove old timestamps
//         timestamps.retain(|timestamp| *timestamp >= window_start);
        
//         // Check if rate limit exceeded
//         if timestamps.len() >= self.max_requests {
//             return false;
//         }
        
//         // Record this request
//         timestamps.push(now);
//         true
//     }
// }

// // Create middleware
// struct RateLimitMiddleware {
//     limiter: Arc<InMemoryRateLimiter>,
// }

// impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
// where
//     S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
//     S::Future: 'static,
//     B: 'static,
// {
//     // Implementation details...
// }
// Lightweight Approach
// For the absolute lightest setup that still provides protection, you could also use an external solution:

// nginx rate limiting: If you're using nginx as a reverse proxy (common in production), you can configure rate limiting there:
// nginx
// http {
//     limit_req_zone $binary_remote_addr zone=redirect_limit:10m rate=1r/s;
    
//     server {
//         # Your other server config
        
//         # Protected redirect endpoint
//         location ~ ^/[a-zA-Z0-9]{6}$ {
//             limit_req zone=redirect_limit burst=60 nodelay;
//             proxy_pass http://localhost:8000;
//         }
        
//         # API endpoints don't need limiting
//         location /api/ {
//             proxy_pass http://localhost:8000;
//         }
//     }
// }
// Cloudflare rate limiting: If you're using Cloudflare in front of your service, you can enable their rate limiting rules:
// Rate Limiting Rule:
// - URL pattern: example.com/*
// - Threshold: 60 requests per minute
// - Action: Block
// - Bypass for logged in users: Yes (if applicable)
// Recommendation for Your Use Case
// Based on your requirements for a lightweight solution specific to the redirect route, I recommend the first approach using actix-governor. It's:

// Simple to implement: Just a few lines of code
// Low overhead: Minimal performance impact
// Targeted protection: Only applies to the routes that need it
// Configurable: Easy to adjust as your needs evolve
// Here's the minimal code you need:

// rust
// use actix_governor::{Governor, GovernorConfigBuilder};

// fn configure_routes(cfg: &mut web::ServiceConfig) {
//     // Create a basic rate limiter: 60 requests per minute per IP
//     let limiter = Governor::new(
//         GovernorConfigBuilder::default()
//             .per_second(1)  // 1 per second
//             .burst_size(60) // Allow bursts up to 60
//             .finish()
//             .unwrap()
//     );

//     // API routes (no rate limiting)
//     cfg.service(web::scope("/api").configure(api_routes));
    
//     // Frontend route (no rate limiting)
//     cfg.service(web::resource("/").route(web::get().to(index_handler)));
    
//     // Apply rate limiting to the redirect route
//     cfg.service(
//         web::resource("/{code}")
//             .wrap(limiter)
//             .route(web::get().to(redirect_handler))
//     );
// }
// This gives you immediate protection against basic abuse while being extremely lightweight and simple to implement. You can always enhance it later as your application scales.

// // In main.rs or app.rs
// use actix_web::{web, App, HttpServer};

// async fn main() -> std::io::Result<()> {
//     // Other initialization code...
    
//     HttpServer::new(move || {
//         App::new()
//             // Set maximum size limits for various types of requests
//             .app_data(web::JsonConfig::default()
//                 .limit(32 * 1024)  // Limit JSON payload to 32KB
//                 .error_handler(|err, _req| {
//                     // Custom error handler for oversized payloads
//                     actix_web::error::InternalError::from_response(
//                         err, 
//                         HttpResponse::BadRequest()
//                             .content_type("application/json")
//                             .body(r#"{"error":"JSON payload too large"}"#)
//                     ).into()
//                 }))
//             .app_data(web::PathConfig::default()
//                 .error_handler(|err, _req| {
//                     // Custom error handler for invalid path parameters
//                     actix_web::error::InternalError::from_response(
//                         err,
//                         HttpResponse::BadRequest()
//                             .content_type("application/json")
//                             .body(r#"{"error":"Invalid path parameter"}"#)
//                     ).into()
//                 }))
//             // Rest of your app configuration...
//     })
//     .bind(format!("{}:{}", config.server.host, config.server.port))?
//     .run()
//     .await
// }