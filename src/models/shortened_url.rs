// src/models/shortened_url.rs - Pure data structures
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use sqlx::FromRow;
use validator::Validate;

use crate::validations::{validate_custom_alias, validate_url};

// DTO for creating a new shortened URL
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateShortenedUrlDto {
    #[validate(custom(function = "validate_url"))]
    pub original_url: String,

    // #[validate(skip)]
    #[validate(custom(function = "validate_custom_alias"))]
    pub custom_alias: Option<String>,

    pub expires_at: Option<DateTime<Utc>>,

    #[validate(range(min = 0, max = 365, message = "Expiry days must be between 0 and 365"))]
    pub expires_in_days: Option<u32>,

    // validate custom metadata
    pub metadata: Option<JsonValue>,
}

/// Represents a shortened URL in the system
#[derive(Debug, Clone, Default, FromRow, Serialize, Deserialize)]
pub struct ShortenedUrl {
    /// The unique ID of the shortened URL
    pub id: Option<Uuid>,

    /// The original, long URL that was shortened
    pub original_url: String,

    /// The generated short code that identifies this URL
    pub short_code: String,

    /// When this shortened URL was created
    pub created_at: DateTime<Utc>,

    /// When this shortened URL was last accessed
    pub last_accessed: Option<DateTime<Utc>>,

    /// Number of times this shortened URL has been accessed
    pub access_count: i64,

    /// When this shortened URL expires (None means it never expires)
    pub expires_at: Option<DateTime<Utc>>,

    /// The identifier of the user or entity that created this shortened URL
    // pub created_by: Option<String>,

    /// Indicates whether the short code was custom or auto-generated
    pub is_custom_code: bool,

    /// Indicates whether the shortened URL is active or not
    pub is_active: bool,

    /// Additional metadata associated with the shortened URL
    pub metadata: Option<JsonValue>,
}

impl ShortenedUrl {
    /// Checks if the shortened URL has expired
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => Utc::now() > expiry,
            None => false,
        }
    }
    
    /// Convenience method to check if the URL is still valid (not expired)
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }
}

// DTO for response with shortened URL details
#[derive(Debug, Serialize, Deserialize)]
pub struct ShortenedUrlResponseDto {
    pub id: Option<Uuid>,
    pub is_active: bool,
    pub access_count: i64,
    pub short_code: String,
    pub original_url: String,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<JsonValue>,
    pub expires_at: Option<DateTime<Utc>>,
}

// Conversion functions between DTO and model
impl From<ShortenedUrl> for ShortenedUrlResponseDto {
    fn from(url: ShortenedUrl) -> Self {
        ShortenedUrlResponseDto {
            id: url.id,
            metadata: url.metadata,
            is_active: url.is_active,
            expires_at: url.expires_at,
            short_code: url.short_code,
            created_at: url.created_at,
            original_url: url.original_url,
            access_count: url.access_count,
        }
    }
}
