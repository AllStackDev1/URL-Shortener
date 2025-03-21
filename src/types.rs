use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Serialize, Deserialize)]
pub struct ResponsePayload {
    pub status: i32,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

// Define an AppState struct to hold shared application state
pub struct AppState {
    pub start_time: Instant,
    pub version: String,
}