use actix_web::{web, Responder};
use uuid::Uuid;

use crate::{
    handlers::{
        create_handler, delete_handler, get_all_handler, get_by_id_handler, get_by_query_handler,
        update_handler, ShortenedUrlServiceType,
    },
    models::{CreateShortenedUrlDto, ShortenedUrlQueryParams, ShortenedUrlUpdateParams},
    types::Result,
};

// Create shortened URL route handler
async fn create_url(
    dto: web::Json<CreateShortenedUrlDto>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    create_handler(dto, service).await
}

// Get all URLs route handler
async fn get_all_url(
    query: web::Query<ShortenedUrlQueryParams>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    get_all_handler(query, service).await
}

// Get URLs by query route handler
async fn get_all_url_by_query(
    query: web::Query<ShortenedUrlQueryParams>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    log::info!("query 0: {:?}", query);
    get_by_query_handler(query, service).await
}

// Get URL by ID route handler
async fn get_url_by_id(
    id: web::Path<Uuid>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    get_by_id_handler(id, service).await
}

// Update URL by ID route handler
async fn update_url(
    id: web::Path<Uuid>,
    param: web::Json<ShortenedUrlUpdateParams>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    update_handler(id, param, service).await
}

// Delete URL by ID route handler
async fn delete_url(
    id: web::Path<Uuid>,
    service: web::Data<ShortenedUrlServiceType>,
) -> Result<impl Responder> {
    delete_handler(id, service).await
}

// Configure all routes function
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/urls")
            .route("", web::post().to(create_url))
            .route("", web::get().to(get_all_url))
            .route("", web::patch().to(update_url))
            .route("", web::delete().to(delete_url))
            .route("/search", web::get().to(get_all_url_by_query))
            .route("/{id}", web::get().to(get_url_by_id)),
        // add more routes here
    );
}
