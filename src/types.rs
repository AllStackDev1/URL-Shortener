use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::{db::{Database, DatabaseHealth}, errors::AppError};

#[derive(Serialize, Deserialize)]
pub struct ResponsePayload {
    pub status: i32,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub db_health: Option<DatabaseHealth>,
    pub uptime_seconds: u64,
}

pub struct AppState {
    pub start_time: Instant,
    pub db: Database,
    pub version: String,
}

pub type Result<T> = std::result::Result<T, AppError>;