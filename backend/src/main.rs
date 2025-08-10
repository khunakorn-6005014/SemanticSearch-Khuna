//backend/src/main.rs
mod error;
mod handlers;
mod store;
mod embedder;

use axum::{
    routing::{post, get},
    Router,
    Extension,
};
use spfresh::Index;

use std::sync::Arc;
use crate::error::ApiError;
use crate::store::Store;
use crate::embedder::Embedder;
use crate::handlers::reviews::{insert_review, insert_bulk_reviews, search_reviews};


#[tokio::main]
async fn main() -> Result<(), ApiError> {
    // Initialize shared components
    let index = Arc::new(Index::open("data/reviews.index")?);
    let store = Arc::new(Store::open("data")?);
    let embedder = Embedder::new().map_err(ApiError::InternalError)?;

    / Build router
    let app = Router::new()
        .route("/reviews", post(insert_review))
        .route("/reviews/bulk", post(insert_bulk_reviews))
        .route("/search", post(search_reviews))
        .route("/health", get(|| async { "OK" }))
        // Inject shared state into handlers
        .layer(Extension(index))
        .layer(Extension(store))
        .layer(Extension(embedder));

   // Start server
    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .map_err(ApiError::InternalError)?;

    Ok(())
}

