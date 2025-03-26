use actix_web::{http::header::LOCATION, web, HttpResponse, Responder};
use chrono::Utc;
use log::{debug, info};
use serde_json::json;
use uuid::Uuid;

use crate::{
    errors::AppError,
    types::Result,
    models::{CreateShortenedUrlDto, ShortenedUrlQueryParams, ShortenedUrlUpdateParams},
    repositories::ShortenedUrlRepository,
    services::{ShortenedUrlService, ShortenedUrlServiceTrait},
};

pub type ShortenedUrlServiceType = ShortenedUrlService<ShortenedUrlRepository>;

/// Create shortened URL route handler
pub async fn create_handler(
    dto: web::Json<CreateShortenedUrlDto>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    let url = service.create(dto.into_inner()).await?;
    Ok(HttpResponse::Created().json(json!({
        "data": url,
        "message": "Successfully created URL",
    })))
}

/// Get all URLs route handler
pub async fn get_all_handler(
    query: web::Query<ShortenedUrlQueryParams>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    let urls = service.get_all(query.limit, query.offset).await?;
    Ok(HttpResponse::Ok().json(json!({
        "data": urls,
        "message": "Successfully retrieved URLs",
    })))
}

/// Get URLs by query route handler
pub async fn get_by_query_handler(
    query: web::Query<ShortenedUrlQueryParams>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    let urls = service.get_by_query(&query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(json!({
        "data": urls,
        "message": "Successfully retrieved URLs",
    })))
}

/// Get URL by ID route handler
pub async fn get_by_id_handler(
    id: web::Path<Uuid>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    let url = service.get_by_id(&id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(json!({
        "data": url,
        "message": "Successfully retrieved URL",
    })))
}

/// Update URL route handler
pub async fn update_handler(
    id: web::Path<Uuid>,
    params: web::Json<ShortenedUrlUpdateParams>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    let url = service.update(&id.into_inner(), params.into_inner()).await?;
    Ok(HttpResponse::Ok().json(json!({
        "data": url,
        "message": "Successfully retrieved URL",
    })))
}

/// Delete URL route handler
pub async fn delete_handler(
    id: web::Path<Uuid>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    let id = id.into_inner();
    let _ = service.delete(&id).await?;
    Ok(HttpResponse::Ok().json(json!({
        "deleted_id": &id,
        "message": format!("Successfully deleted URL with ID '{}'", id),
    })))
}

/// Redirect route handler
pub async fn redirect_handler(
    path: web::Path<String>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    let short_code = path.into_inner();
    debug!("Redirect requested for code: {}", short_code);

    // Find the URL by short code, it should fail if not found
    let url = service.get_by_code(&short_code).await?;

    // Check if URL is still valid
    if url.is_valid() {
        info!("URL with code '{}' has expired", short_code);
        return Err(AppError::Validation(format!(
            "URL with code '{}' has expired",
            short_code
        )));
    }

    // Increment access count (don't wait for the result to avoid delaying the redirect)
    let params = ShortenedUrlUpdateParams {
        access_count: url.access_count + 1,
        last_accessed: Some(Utc::now()),
        metadata: Some(format!("Last accessed at: {}", Utc::now()).into()),
        ..Default::default()
    };
    let _ = service.update(&url.id, params).await;

    // Log the successful redirect
    info!("Redirecting '{}' to '{}'", short_code, url.original_url);

    // Return redirect response
    Ok(HttpResponse::TemporaryRedirect()
        .insert_header((LOCATION, url.original_url.clone()))
        .finish())
}
