use crate::storage::{UrlStorage, StorageError};
use crate::models::ShortenedUrl;
use crate::utils::id_generator;
use std::time::Duration;

const MAX_RETRIES: usize = 5;
const DEFAULT_CODE_LENGTH: usize = 6;

pub struct UrlGeneratorService {
    storage: UrlStorage,
    code_length: usize,
}

impl UrlGeneratorService {
    pub fn new(storage: UrlStorage) -> Self {
        Self {
            storage,
            code_length: DEFAULT_CODE_LENGTH,
        }
    }
    
    pub fn with_code_length(storage: UrlStorage, code_length: usize) -> Self {
        Self {
            storage,
            code_length,
        }
    }
    
    // Generate a unique short URL with collision avoidance
    pub async fn generate(
        &self, 
        original_url: String, 
        force_new: bool,
        expiry_duration: Option<Duration>
    ) -> Result<ShortenedUrl, StorageError> {
        // Check for existing URL first, unless force_new is true
        if !force_new {
            if let Some(existing) = self.storage.find_by_original_url(&original_url).await? {
                return Ok(existing);
            }
        }
        
        // Try generating a unique code
        for attempt in 0..MAX_RETRIES {
            let short_code = id_generator::generate_short_id(self.code_length);
            
            // Check if this short code already exists
            if !self.storage.exists(&short_code).await? {
                // Create the new shortened URL
                let mut url = if let Some(duration) = expiry_duration {
                    ShortenedUrl::with_duration(original_url.clone(), short_code, duration)
                } else {
                    ShortenedUrl::new(original_url.clone(), short_code)
                };
                
                // Store it and return
                self.storage.store(url.clone()).await?;
                return Ok(url);
            }
            
            // If we're on the last attempt and still have collisions,
            // try increasing the code length
            if attempt == MAX_RETRIES - 1 {
                let longer_code = id_generator::generate_short_id(self.code_length + 2);
                let mut url = if let Some(duration) = expiry_duration {
                    ShortenedUrl::with_duration(original_url.clone(), longer_code, duration)
                } else {
                    ShortenedUrl::new(original_url.clone(), longer_code)
                };
                
                self.storage.store(url.clone()).await?;
                return Ok(url);
            }
        }
        
        // This should technically never be reached due to the fallback above
        Err(StorageError::InternalError("Failed to generate unique short code".to_string()))
    }
}