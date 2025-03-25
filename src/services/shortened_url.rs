// src/services/shortened_url.rs - Business logic
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use validator::Validate;

use crate::errors::ServiceError;
use crate::models::{CreateShortenedUrlDto, ShortenedUrl, ShortenedUrlResponseDto};
use crate::repositories::ShortenedUrlRepositoryTrait;
use crate::utils::id_generator;

type Result<T> = std::result::Result<T, ServiceError>;

#[async_trait]
pub trait ShortenedUrlServiceTrait {
    async fn create(&self, dto: CreateShortenedUrlDto) -> Result<ShortenedUrlResponseDto>;
    // fn get_by_id(&self, id: Uuid) -> Result<ShortenedUrl>;
    // fn get_by_shortened_url(&self, shortened_url: String) -> Result<ShortenedUrl>;
    // fn update(&self, shortened_url: ShortenedUrl) -> Result<ShortenedUrl>;
    // fn delete(&self, id: Uuid) -> Result<()>;
}

pub struct ShortenedUrlService<T: ShortenedUrlRepositoryTrait> {
    repository: Arc<T>,
}

impl<T: ShortenedUrlRepositoryTrait> ShortenedUrlService<T> {
    pub fn new(repository: Arc<T>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<T: ShortenedUrlRepositoryTrait + Send + Sync> ShortenedUrlServiceTrait
    for ShortenedUrlService<T>
{
    async fn create(&self, dto: CreateShortenedUrlDto) -> Result<ShortenedUrlResponseDto> {
        if let Err(e) = dto.validate() {
            return Err(ServiceError::ValidationError(e.to_string()));
        }

        // Generate or use custom short code
        let (short_code, is_custom_code) = match dto.custom_alias {
            Some(code) if !code.trim().is_empty() => {
                // Check if custom code is already in use
                if (self.repository.find_by_code(&code).await?).is_some() {
                    return Err(ServiceError::ValidationError(format!(
                        "Custom short code '{}' is already in use",
                        code
                    )));
                }
                (code, true)
            }
            _ => {
                // Generate a unique short code
                let mut code = id_generator::generate_short_id(6);

                // Ensure the generated code is unique
                let mut attempts = 0;
                while (self.repository.find_by_code(&code).await?).is_some() {
                    code = id_generator::generate_short_id(6);
                    attempts += 1;

                    if attempts >= 5 {
                        return Err(ServiceError::InternalError(
                            "Failed to generate a unique short code after multiple attempts"
                                .to_string(),
                        ));
                    }
                }

                (code, false)
            }
        };

        // Create a new URL entity with basic info
        let mut shortened_url = ShortenedUrl {
            short_code,
            is_custom_code,
            original_url: dto.original_url,
            ..Default::default()
        };

        // Handle expiration logic (prioritize expires_at over expires_in_days)
        if let Some(expires_at) = dto.expires_at {
            // Validate that expiration is in the future
            if expires_at <= Utc::now() {
                return Err(ServiceError::ValidationError(
                    "Expiration date must be in the future".to_string(),
                ));
            }
            shortened_url.expires_at = Some(expires_at);
        } else if let Some(days) = dto.expires_in_days {
            if days == 0 {
                return Err(ServiceError::ValidationError(
                    "Expiration days must be positive".to_string(),
                ));
            }

            // Calculate expiration date based on days
            shortened_url.expires_at = Some(Utc::now() + Duration::days(days as i64));
        }

        // Set optional metadata if provided
        shortened_url.metadata = dto.metadata;

        // Save to repository
        let record = self.repository.save(&mut shortened_url).await?;
        let response_dto = ShortenedUrlResponseDto::from(record);

        Ok(response_dto)
    }
}
