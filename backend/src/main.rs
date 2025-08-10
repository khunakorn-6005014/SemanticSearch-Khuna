//backend/src/main.rs
mod error;
mod handlers;
mod store;  //src/store.rs
mod embedder;

use axum::{
    routing::{get, post},
    Router,
    extract::State,
};

use std::sync::{Arc, Mutex};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

use crate::error::ApiError;
use crate::store::Store;
use crate::embedder::Embedder;
use crate::handlers::reviews::{insert_review, insert_bulk_reviews, search_reviews};
use spfresh::Index;

/// Centralized state 
#[derive(Clone)]
struct AppState {
    index: Arc<Index>,
    store: Arc<Store>,
    embedder: Arc<Embedder>,
}

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    // Init logging/tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("info,axum=info")
        }))
        .init();
    // Initialize shared components
    let index = Arc::new(Index::open("data/reviews.index")?);
    let store = Arc::new(Store::open("data")?);
    let embedder = Embedder::new().map_err(ApiError::InternalError)?;
     let state = AppState { index, store, embedder };

    // Build router
    let app = Router::new()
        .route("/reviews", post(insert_review))
        .route("/reviews/bulk", post(insert_bulk_reviews))
        .route("/search", post(search_reviews))
        .route("/health", get(|| async { "OK" }))
        // Inject shared state into handlers
        .with_state(state);
    //      .layer(Extension(index))
    // .layer(Extension(store))
    // .layer(Extension(embedder));

   // Start server
    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .map_err(ApiError::InternalError)?;

    Ok(())
}

