use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a shortened URL in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortenedUrl {
    /// The unique ID of the shortened URL
    pub id: Option<i64>,

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
}

impl ShortenedUrl {
    /// Creates a new ShortenedUrl with default values
    pub fn new(original_url: String, short_code: String) -> Self {
        ShortenedUrl {
            id: None,               // No ID until stored in database
            original_url,           // Use the provided original URL
            short_code,             // Use the provided short code
            created_at: Utc::now(), // Set creation time to current time
            last_accessed: None,    // No access yet
            access_count: 0,        // Initialize access count to zero
            expires_at: None,       // No expiration by default
        }
    }

    /// Increments the access count and updates the last_accessed timestamp
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Some(Utc::now());
    }

    /// Checks if the shortened URL is valid (not expired)
    pub fn is_valid(&self) -> bool {
        match self.expires_at {
            Some(expiry) => Utc::now() < expiry,
            None => true, // URLs without expiration are always valid
        }
    }

    /// Creates a new ShortenedUrl with a specific expiration time
    pub fn with_expiration(
        original_url: String,
        short_code: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        ShortenedUrl {
            id: None,
            original_url,
            short_code,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            expires_at: Some(expires_at),
        }
    }

    /// Creates a new ShortenedUrl that expires after a specified duration from now
    pub fn with_duration(original_url: String, short_code: String, duration: Duration) -> Self {
        let expires_at = Utc::now() + duration;

        ShortenedUrl {
            id: None,
            original_url,
            short_code,
            created_at: Utc::now(),
            last_accessed: None,
            access_count: 0,
            expires_at: Some(expires_at),
        }
    }

    /// Removes the expiration time, making the URL never expire
    pub fn remove_expiration(&mut self) {
        self.expires_at = None;
    }
}
