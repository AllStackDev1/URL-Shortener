// src/repositories/shortened_url.rs - Data access
use async_trait::async_trait;
use chrono::Utc;
use log::debug;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::db::Database;
use crate::errors::RepositoryError;
use crate::models::{ShortenedUrl, ShortenedUrlQueryParams, ShortenedUrlUpdateParams};

type Result<T> = std::result::Result<T, RepositoryError>;

#[async_trait]
pub trait ShortenedUrlRepositoryTrait {
    /// Saves a shortened URL to the database and assigns it a UUID
    ///
    /// ### Arguments
    /// * `url` - The shortened URL to save, will be updated with the generated ID
    ///
    /// ### Returns
    /// * `Result<Uuid>` - The UUID of the newly created record on success
    ///
    /// ### Errors
    /// * `RepositoryError::Database` - If a database error occurs
    /// * `RepositoryError::Conflict` - If there's a constraint violation (e.g., duplicate short code)
    async fn save(&self, url: &ShortenedUrl) -> Result<ShortenedUrl>;

    /// Finds some shortened URL by params
    ///
    /// ### Arguments
    /// * `params` - ShortenedUrlQueryParams object with filters
    ///
    /// ### Returns
    /// * `Result<Vec<ShortenedUrl>>` - The list of shortened URL found
    ///
    /// ### Errors
    /// * `RepositoryError::Database` - If a database error occurs
    /// * `RepositoryError::InvalidData` - If the database record cannot be mapped to a model
    async fn find(&self, params: &ShortenedUrlQueryParams) -> Result<Vec<ShortenedUrl>>;

    /// Finds a shortened URL by its unique identifier (UUID)
    ///
    /// ### Arguments
    /// * `id` - The UUID of the shortened URL to find
    ///
    /// ### Returns
    /// * `Result<Option<ShortenedUrl>>` - The shortened URL if found, or `None` if not found
    ///
    /// ### Errors
    /// * `RepositoryError::Database` - If a database error occurs
    /// * `RepositoryError::InvalidData` - If the database record cannot be mapped to a model
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ShortenedUrl>>;

    /// Finds a shortened URL by its unique short code
    ///
    /// ### Arguments
    /// * `code` - The short code of the shortened URL to find
    ///
    /// ### Returns
    /// * `Result<Option<ShortenedUrl>>` - The shortened URL if found, or `None` if not found
    ///
    /// ### Errors
    /// * `RepositoryError::Database` - If a database error occurs
    /// * `RepositoryError::InvalidData` - If the database record cannot be mapped to a model
    async fn find_by_code(&self, code: &str) -> Result<Option<ShortenedUrl>>;

    /// Finds all shortened URLs with optional pagination
    ///
    /// ### Arguments
    /// * `limit` - The maximum number of records to return (optional)
    /// * `offset` - The number of records to skip before starting to return results (optional)
    ///
    /// ### Returns
    /// * `Result<Vec<ShortenedUrl>>` - A list of shortened URLs
    ///
    /// ### Errors
    /// * `RepositoryError::Database` - If a database error occurs
    async fn find_all(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<ShortenedUrl>>;

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
    async fn update(&self, id: &Uuid, params: &ShortenedUrlUpdateParams) -> Result<u64>;

    /// Deletes a shortened URL by its unique identifier (UUID)
    ///
    /// ### Arguments
    /// * `id` - The UUID of the shortened URL to delete
    /// * `require_exists` - If `true`, an error will be returned if the URL does not exist
    ///
    /// ### Returns
    /// * `Result<u64>` - `number` number of rows affected
    ///
    /// ### Errors
    /// * `RepositoryError::NotFound` - If the URL doesn't exist and `require_exists` is `true`
    /// * `RepositoryError::Database` - If a database error occurs
    async fn delete(&self, id: &Uuid, require_exists: bool) -> Result<bool>;
}

// Implementation using actual database
pub struct ShortenedUrlRepository {
    pool: PgPool,
}

impl ShortenedUrlRepository {
    pub fn new(db: Database) -> Self {
        Self { pool: db.get_pool().clone() }
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
    async fn save(&self, url: &ShortenedUrl) -> Result<ShortenedUrl> {
        // Start a transaction so we can rollback if needed
        let mut tx = self.begin_transaction().await?;

        // Insert the shortened URL
        let record = sqlx::query_as!(
            ShortenedUrl,
            r#"
                INSERT INTO shortened_urls 
                (original_url, short_code, last_accessed, access_count, expires_at, is_custom_code, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING *
            "#,
            url.original_url,
            url.short_code,
            url.last_accessed,
            url.access_count as i64,
            url.expires_at,
            url.is_custom_code,
            url.metadata
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            log::error!("Failed to insert shortened URL: {}", e);
            RepositoryError::from(e)
        })?;

        // Commit the transaction
        tx.commit().await.map_err(|e| {
            log::error!("Failed to commit transaction: {}", e);
            RepositoryError::Database(e)
        })?;

        Ok(record)
    }

    async fn find(&self, params: &ShortenedUrlQueryParams) -> Result<Vec<ShortenedUrl>> {
        // Use QueryBuilder instead of manual string manipulation
        let mut query_builder = QueryBuilder::new(
            "SELECT * 
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

        if let Some(is_active) = params.is_active {
            query_builder.push(" AND is_active = ");
            query_builder.push_bind(is_active);
        }

        if let Some(is_custom_code) = params.is_custom_code {
            query_builder.push(" AND is_custom_code = ");
            query_builder.push_bind(is_custom_code);
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

    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ShortenedUrl>> {
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
        let params = ShortenedUrlQueryParams {
            limit,
            offset,
            ..Default::default()
        };

        // Use the existing find method
        self.find(&params).await
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<ShortenedUrl>> {
        let params = ShortenedUrlQueryParams {
            short_code: Some(code.to_string()),
            ..Default::default()
        };

        self.find(&params)
            .await
            .map(|results| results.into_iter().next())
    }

    async fn update(&self, id: &Uuid, params: &ShortenedUrlUpdateParams) -> Result<u64> {
        debug!("Updating URL with id: {} and params: {:?}", id, params);

        let mut builder = QueryBuilder::new("UPDATE shortened_urls SET ");
        let mut separated = builder.separated(", ");

        if let Some(url) = &params.original_url {
            separated.push("original_url = ").push_bind(url);
        }

        if let Some(is_active) = &params.is_active {
            if *is_active {
                separated.push("expires_at = NULL");
            } else {
                separated.push("expires_at = ").push_bind(Utc::now());
            }
        }

        separated.push("updated_at = ").push_bind(Utc::now());

        // Add the WHERE clause
        builder.push(" WHERE id = ").push_bind(id);

        // Optional: RETURNING if you want the updated row back
        // builder.push(" RETURNING *");

        let query = builder.build();

        // Execute it
        let result = query.execute(&self.pool).await?;
        let affected = result.rows_affected();

        debug!("Updated URL with ID {}: {:?}", id, result);
        Ok(affected)
    }

    async fn delete(&self, id: &Uuid, require_exists: bool) -> Result<bool> {
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

        let is_rows_deleted = result.rows_affected() > 0;

        // Check if we should require the record to exist
        if require_exists && !is_rows_deleted {
            return Err(RepositoryError::NotFound(format!(
                "URL with ID {} not found",
                id
            )));
        }

        // Return whether a row was actually deleted
        Ok(is_rows_deleted)
    }
}
