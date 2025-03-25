// src/repositories/shortened_url.rs - Data access
use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::errors::RepositoryError;
use crate::models::ShortenedUrl;

type Result<T> = std::result::Result<T, RepositoryError>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl Default for OrderDirection {
    fn default() -> Self {
        OrderDirection::Desc
    }
}

impl fmt::Display for OrderDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderDirection::Asc => write!(f, "ASC"),
            OrderDirection::Desc => write!(f, "DESC"),
        }
    }
}

// Enum for allowed sort fields
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Id,
    ShortCode,
    OriginalUrl,
    CreatedAt,
    ExpiresAt,
    LastAccessed,
    AccessCount,
}

impl Default for SortField {
    fn default() -> Self {
        SortField::CreatedAt
    }
}

impl SortField {
    // Get database column name for this field
    pub fn as_column(&self) -> &'static str {
        match self {
            SortField::Id => "id",
            SortField::ShortCode => "short_code",
            SortField::OriginalUrl => "original_url",
            SortField::CreatedAt => "created_at",
            SortField::ExpiresAt => "expires_at",
            SortField::LastAccessed => "last_accessed",
            SortField::AccessCount => "access_count",
        }
    }

    // Check if a given column name is valid
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "id" => Some(Self::Id),
            "short_code" => Some(Self::ShortCode),
            "original_url" => Some(Self::OriginalUrl),
            "created_at" => Some(Self::CreatedAt),
            "expires_at" => Some(Self::ExpiresAt),
            "last_accessed" => Some(Self::LastAccessed),
            "access_count" => Some(Self::AccessCount),
            _ => None,
        }
    }
}

// Query parameters struct for the flexible find method
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct UrlQueryParams {
    pub id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub is_expired: Option<bool>,
    pub short_code: Option<String>,
    pub order_by: Option<SortField>,
    pub original_url: Option<String>,
    pub min_access_count: Option<i64>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub order_direction: Option<OrderDirection>,
}

#[async_trait]
pub trait ShortenedUrlRepositoryTrait {
    /// Saves a shortened URL to the database and assigns it a UUID
    ///
    /// # Arguments
    /// * `url` - The shortened URL to save, will be updated with the generated ID
    ///
    /// # Returns
    /// * `Result<Uuid>` - The UUID of the newly created record on success
    ///
    /// # Errors
    /// * `RepositoryError::Database` - If a database error occurs
    /// * `RepositoryError::Conflict` - If there's a constraint violation (e.g., duplicate short code)
    async fn save(&self, url: &mut ShortenedUrl) -> Result<ShortenedUrl>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ShortenedUrl>>;
    async fn find_by_code(&self, code: &str) -> Result<Option<ShortenedUrl>>;
    async fn find(&self, params: &UrlQueryParams) -> Result<Vec<ShortenedUrl>>;
    async fn find_all(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<ShortenedUrl>>;
    async fn update(&self, url: &ShortenedUrl) -> Result<()>;
    async fn update_access_stats(
        &self,
        id: Uuid,
        last_accessed: chrono::DateTime<chrono::Utc>,
        access_count: u64,
    ) -> Result<()>;
    async fn update_expiration(
        &self,
        id: Uuid,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()>;
    async fn delete(&self, id: Uuid, require_exists: bool) -> Result<bool>;
}

// Implementation using actual database
pub struct ShortenedUrlRepository {
    pool: PgPool,
}

impl ShortenedUrlRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // Helper method for transactions
    async fn begin_transaction(&self) -> Result<Transaction<'_, Postgres>> {
        self.pool.begin().await.map_err(|e| {
            log::error!("Failed to start database transaction: {}", e);
            RepositoryError::Database(e)
        })
    }
}

#[async_trait]
impl ShortenedUrlRepositoryTrait for ShortenedUrlRepository {
    async fn save(&self, url: &mut ShortenedUrl) -> Result<ShortenedUrl> {
        // Start a transaction so we can rollback if needed
        let mut tx = self.begin_transaction().await?;

        // Check if short_code already exists
        let exists = sqlx::query!(
            r#"
            SELECT EXISTS(SELECT 1 FROM shortened_urls WHERE short_code = $1) as exists
            "#,
            url.short_code
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            log::error!("Failed to check if short code exists: {}", e);
            RepositoryError::Database(e)
        })?;

        if exists.exists.unwrap_or(false) {
            // Rollback the transaction
            if let Err(e) = tx.rollback().await {
                log::warn!(
                    "Failed to rollback transaction after short code conflict: {}",
                    e
                );
            }

            return Err(RepositoryError::Conflict(format!(
                "Short code '{}' is already in use",
                url.short_code
            )));
        }

        // Insert the shortened URL
        let record = sqlx::query_as!(
            ShortenedUrl,
            r#"
                INSERT INTO shortened_urls 
                (original_url, short_code, created_at, last_accessed, access_count, expires_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id, original_url, short_code, created_at, expires_at, last_accessed, access_count, is_custom_code, is_active, metadata
            "#,
            url.original_url,
            url.short_code,
            url.created_at,
            url.last_accessed,
            url.access_count as i64,
            url.expires_at,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            // Check for specific PostgreSQL error codes
            if let sqlx::Error::Database(ref db_err) = e {
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        // Unique violation
                        "23505" => {
                            return RepositoryError::Conflict(format!(
                                "Conflict while saving URL: {}",
                                e
                            ))
                        }
                        // Check constraint violation
                        "23514" => {
                            return RepositoryError::InvalidData(format!(
                                "Invalid data for URL: {}",
                                e
                            ))
                        }
                        _ => {}
                    }
                }
            }

            log::error!("Failed to insert shortened URL: {}", e);
            RepositoryError::Database(e)
        })?;

        // Commit the transaction
        tx.commit().await.map_err(|e| {
            log::error!("Failed to commit transaction: {}", e);
            RepositoryError::Database(e)
        })?;

        Ok(record)
    }

    /// Finds some shortened URL by params
    ///
    /// # Arguments
    /// * `params` - UrlQueryParams
    ///
    /// # Returns
    /// * `Result<Vec<ShortenedUrl>>` - The list of shortened URL found
    ///
    /// # Errors
    /// * `RepositoryError::Database` - If a database error occurs
    /// * `RepositoryError::InvalidData` - If the database record cannot be mapped to a model
    async fn find(&self, params: &UrlQueryParams) -> Result<Vec<ShortenedUrl>> {
        // Use QueryBuilder instead of manual string manipulation
        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id, original_url, short_code, created_at, expires_at, last_accessed, access_count 
            FROM shortened_urls 
            WHERE 1=1"
        );

        // Add conditions based on provided parameters
        if let Some(code) = &params.short_code {
            query_builder.push(" AND short_code = ");
            query_builder.push_bind(code);
        }

        if let Some(url) = &params.original_url {
            query_builder.push(" AND original_url LIKE ");
            query_builder.push_bind(format!("%{}%", url));
        }

        if let Some(id) = params.id {
            query_builder.push(" AND id = ");
            query_builder.push_bind(id);
        }

        if let Some(after) = params.created_after {
            query_builder.push(" AND created_at >= ");
            query_builder.push_bind(after);
        }

        if let Some(before) = params.created_before {
            query_builder.push(" AND created_at <= ");
            query_builder.push_bind(before);
        }

        let now = Utc::now();
        if let Some(true) = params.is_expired {
            // URLs that have an expiration date in the past
            query_builder.push(" AND (expires_at IS NOT NULL AND expires_at < ");
            query_builder.push_bind(now);
            query_builder.push(")");
        } else if let Some(false) = params.is_expired {
            // URLs that either have no expiration or expiration in the future
            query_builder.push(" AND (expires_at IS NULL OR expires_at >= ");
            query_builder.push_bind(now);
            query_builder.push(")");
        }

        if let Some(min_count) = params.min_access_count {
            query_builder.push(" AND access_count >= ");
            query_builder.push_bind(min_count);
        }

        // Add order by with dynamic column and direction
        let order_by = params.order_by.unwrap_or_default();
        let direction = params.order_direction.unwrap_or_default();

        // Safely add the ORDER BY clause with the column name (not user input)
        query_builder.push(" ORDER BY ");
        query_builder.push(order_by.as_column());
        query_builder.push(" ");
        query_builder.push(direction.to_string());

        // Add limit and offset
        if let Some(limit) = params.limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }

        if let Some(offset) = params.offset {
            query_builder.push(" OFFSET ");
            query_builder.push_bind(offset);
        }

        // Build the final query
        let query = query_builder.build_query_as::<ShortenedUrl>();

        // Execute and return the results
        let results = query.fetch_all(&self.pool).await?;

        Ok(results)
    }

    // Implementation of find_by_id
    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<ShortenedUrl>> {
        sqlx::query_as!(
                ShortenedUrl,
                r#"
                SELECT id, original_url, short_code, created_at, expires_at, last_accessed, access_count, is_custom_code, is_active, metadata
                FROM shortened_urls
                WHERE id = $1
                "#,
                id
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(RepositoryError::Database)
    }

    async fn find_all(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<ShortenedUrl>> {
        // Create an empty query params object (no filters)
        let params = UrlQueryParams {
            limit,
            offset,
            ..Default::default()
        };
        
        // Use the existing find method
        self.find(&params).await
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<ShortenedUrl>> {
        let params = UrlQueryParams {
            short_code: Some(code.to_string()),
            ..Default::default()
        };

        self.find(&params)
        .await
        .map(|results| results.into_iter().next())
    }
    
    /// Updates a shortened URL in the database
    ///
    /// # Arguments
    /// * `url` - The shortened URL to update with new values
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Errors
    /// * `RepositoryError::NotFound` - If the URL doesn't exist
    /// * `RepositoryError::InvalidData` - If the URL has no ID
    /// * `RepositoryError::Database` - If a database error occurs
    async fn update(&self, url: &ShortenedUrl) -> Result<()> {
        // Ensure we have an ID to update
        let id = match url.id {
            Some(id) => id,
            None => {
                log::error!("Attempted to update URL without ID");
                return Err(RepositoryError::InvalidData(
                    "Cannot update URL without ID".to_string(),
                ));
            }
        };

        // Execute the update query
        let result = sqlx::query!(
            r#"
            UPDATE shortened_urls
            SET 
                last_accessed = $1,
                access_count = $2,
                expires_at = $3
            WHERE id = $4
            "#,
            url.last_accessed,
            url.access_count as i64, // Convert u64 to i64 for PostgreSQL
            url.expires_at,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            log::error!("Database error while updating URL with ID {}: {}", id, err);
            RepositoryError::Database(err)
        })?;

        // Check if any row was actually updated
        if result.rows_affected() == 0 {
            log::warn!("No URL found with ID {} during update", id);
            return Err(RepositoryError::NotFound(format!(
                "URL with ID {} not found",
                id
            )));
        }

        log::debug!("Updated URL with ID {}: {:?}", id, url);
        Ok(())
    }

    /// Updates only the access-related fields (last_accessed and access_count)
    /// This is a more efficient version when you only need to update these fields
    ///
    /// # Arguments
    /// * `id` - The UUID of the URL to update
    /// * `last_accessed` - The new last_accessed timestamp
    /// * `access_count` - The new access count
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    async fn update_access_stats(
        &self,
        id: Uuid,
        last_accessed: chrono::DateTime<chrono::Utc>,
        access_count: u64,
    ) -> Result<()> {
        let result = sqlx::query!(
            r#"
            UPDATE shortened_urls
            SET 
                last_accessed = $1,
                access_count = $2
            WHERE id = $3
            "#,
            last_accessed,
            access_count as i64,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            log::error!(
                "Database error while updating access stats for URL with ID {}: {}",
                id,
                err
            );
            RepositoryError::Database(err)
        })?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound(format!(
                "URL with ID {} not found",
                id
            )));
        }

        log::debug!(
            "Updated access stats for URL with ID {}: last_accessed={:?}, access_count={}",
            id,
            last_accessed,
            access_count
        );
        Ok(())
    }

    /// Updates only the expiration date of a URL
    ///
    /// # Arguments
    /// * `id` - The UUID of the URL to update
    /// * `expires_at` - The new expiration timestamp (None for no expiration)
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    async fn update_expiration(
        &self,
        id: Uuid,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        let result = sqlx::query!(
            r#"
            UPDATE shortened_urls
            SET expires_at = $1
            WHERE id = $2
            "#,
            expires_at,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(|err| {
            log::error!(
                "Database error while updating expiration for URL with ID {}: {}",
                id,
                err
            );
            RepositoryError::Database(err)
        })?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound(format!(
                "URL with ID {} not found",
                id
            )));
        }

        log::debug!(
            "Updated expiration for URL with ID {}: expires_at={:?}",
            id,
            expires_at
        );
        Ok(())
    }

    // Implementation of configurable delete
    async fn delete(&self, id: Uuid, require_exists: bool) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            DELETE FROM shortened_urls
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;
        
        let rows_deleted = result.rows_affected() > 0;
        
        // Check if we should require the record to exist
        if require_exists && !rows_deleted {
            return Err(RepositoryError::NotFound(
                format!("URL with ID {} not found", id)
            ));
        }
        
        // Return whether a row was actually deleted
        Ok(rows_deleted)
    }
}
