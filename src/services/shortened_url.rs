// src/services/shortened_url.rs - Business logic
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::AppError,
    models::{
        CreateShortenedUrlDto, ShortenedUrl, ShortenedUrlQueryParams, ShortenedUrlResponseDto,
        ShortenedUrlUpdateParams,
    },
    repositories::ShortenedUrlRepositoryTrait,
    types::Result,
    utils::id_generator,
};

#[async_trait]
pub trait ShortenedUrlServiceTrait {
    async fn create(&self, dto: CreateShortenedUrlDto) -> Result<ShortenedUrlResponseDto>;
    async fn get_by_id(&self, id: &Uuid) -> Result<ShortenedUrl>;
    async fn get_by_query(&self, params: &ShortenedUrlQueryParams) -> Result<Vec<ShortenedUrl>>;
    async fn get_all(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<ShortenedUrl>>;
    async fn get_by_code(&self, code: &str) -> Result<ShortenedUrl>;
    async fn update(&self, id: &Uuid, params: ShortenedUrlUpdateParams) -> Result<u64>;
    async fn delete(&self, id: &Uuid) -> Result<bool>;
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
        dto.validate()?;

        // Generate or use custom short code
        let (short_code, is_custom_code) = match dto.custom_alias {
            Some(code) if !code.trim().is_empty() => {
                // Check if custom code is already in use
                if (self.repository.find_by_code(&code).await?).is_some() {
                    return Err(AppError::Validation(format!(
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
                        return Err(AppError::Internal(
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
                return Err(AppError::Validation(
                    "Expiration date must be in the future".to_string(),
                ));
            }
            shortened_url.expires_at = Some(expires_at);
        } else if let Some(days) = dto.expires_in_days {
            if days == 0 {
                return Err(AppError::Validation(
                    "Expiration days must be positive".to_string(),
                ));
            }

            // Calculate expiration date based on days
            shortened_url.expires_at = Some(Utc::now() + Duration::days(days as i64));
        }

        // Set optional metadata if provided
        shortened_url.metadata = dto.metadata;

        // Save to repository
        let record = self.repository.save(&shortened_url).await?;
        let response_dto = ShortenedUrlResponseDto::from(record);

        Ok(response_dto)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<ShortenedUrl> {
        match self.repository.find_by_id(id).await? {
            Some(url) => Ok(url),
            None => Err(AppError::NotFound(format!(
                "URL with ID '{}' not found",
                id
            ))),
        }
    }

    async fn get_by_code(&self, code: &str) -> Result<ShortenedUrl> {
        match self.repository.find_by_code(code).await? {
            Some(url) => Ok(url),
            None => Err(AppError::NotFound(format!(
                "URL with code '{}' not found",
                code
            ))),
        }
    }

    async fn get_by_query(&self, params: &ShortenedUrlQueryParams) -> Result<Vec<ShortenedUrl>> {
        print!("params: {:?}", params);
        let urls = self.repository.find(params).await?;
        Ok(urls)
    }

    async fn get_all(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<ShortenedUrl>> {
        let urls = self.repository.find_all(limit, offset).await?;
        Ok(urls)
    }

    async fn update(&self, id: &Uuid, dto: ShortenedUrlUpdateParams) -> Result<u64> {
        dto.validate()?;

        let rows = self.repository.update(id, &dto).await?;
        Ok(rows)
    }

    async fn delete(&self, id: &Uuid) -> Result<bool> {
        let is_rows_deleted = self.repository.delete(id, false).await?;
        Ok(is_rows_deleted)
    }
}
