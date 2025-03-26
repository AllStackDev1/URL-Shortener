use std::sync::Arc;

use actix_web::web;

mod shortened_url;

pub use shortened_url::{ShortenedUrlService, ShortenedUrlServiceTrait};

use crate::{db::Database, repositories::ShortenedUrlRepository};

/// Service Register
pub fn register(db: Database, cfg: &mut web::ServiceConfig) {
    // create repository
    let shortened_url_repository = ShortenedUrlRepository::new(db.clone());
    let shortened_url_service = ShortenedUrlService::new(Arc::new(shortened_url_repository));
    cfg.app_data(web::Data::new(shortened_url_service));
}
