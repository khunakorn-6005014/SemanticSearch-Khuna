// src/handlers/reviews.rs

use axum::{
    Extension,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use spfresh::Index;


use crate::{
    error::ApiError,
    store::{ReviewMeta, Store},
    embedder::Embedder,
};
/// Payload for single review
#[derive(Deserialize)]
pub struct ReviewInput {
    pub title: String,
    pub body: String,
}
/// Payload for bulk reviews
#[derive(Deserialize)]
pub struct BulkReviewInput(pub Vec<ReviewInput>);

/// Response for single insert
#[derive(Serialize)]
pub struct ApiResponse {
    pub id: usize,
    pub success: bool,
}

/// Response for bulk insert
#[derive(Serialize)]
pub struct BulkInsertResponse {
    pub inserted: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
/// Payload for search
#[derive(Deserialize)]
pub struct SearchInput {
    pub query: String,
    pub top_k: Option<usize>,
}
/// One search hit
#[derive(Serialize)]
pub struct SearchResult {
    pub id: usize,
    pub title: String,
    pub body: String,
    pub score: f32,
}


/// Single insert (already in your code)
/// POST /reviews
pub async fn insert_review(
    Extension(store): Extension<Arc<Store>>,
    Extension(embedder): Extension<Embedder>,
    //Extension(index): Extension<Arc<Index>>,
    Json(input): Json<ReviewInput>,
) -> Result<Json<ApiResponse>, ApiError> {
    // Basic validation if empty
    if input.title.trim().is_empty() || input.body.trim().is_empty() {
        return Err(ApiError::ValidationError("title/body cannot be empty".into()));
    }

       // Generate ID & embed
    let id = store.next_id();
    let meta = ReviewMeta {
        id,
        title: input.title.clone(),
        body: input.body.clone(),
    };
    let vector = embedder.embed(&format!("{} . {}", meta.title, meta.body));

    // Append and return
    store
        .append_review(&vector, &meta)
        .map_err(ApiError::InternalError)?;
    Ok(Json(ApiResponse { id, success: true }))
}


/// POST /reviews/bulk
pub async fn insert_bulk_reviews(
    Extension(store): Extension<Arc<Store>>,
    Extension(embedder): Extension<Embedder>,
    Json(BulkReviewInput(items)): Json<BulkReviewInput>,
) -> Result<Json<BulkInsertResponse>, ApiError> {
    let mut inserted = 0;
    let mut failed = 0;
    let mut errors = Vec::new();


    for (i, item) in items.into_iter().enumerate() {
        // Validation: non-empty
          if item.title.trim().is_empty() || item.body.trim().is_empty() {
            failed += 1;
            errors.push(format!("item {}: empty title or body", i));
            continue;
        }

        // Build meta & vector
        let id = store.next_id();
        let meta = ReviewMeta {
            id,
            title: item.title,
            body: item.body,
        };
        let vector = embedder.embed(&format!("{} . {}", meta.title, meta.body));

        // Append to store
        match store.append_review(&vector, &meta) {
            Ok(_) => inserted += 1,
            Err(e) => {
                failed += 1;
                errors.push(format!("item {}: {}", i, e));
            }
        }
    }

    Ok(Json(BulkInsertResponse { inserted, failed, errors }))
}


// POST /search
pub async fn search_reviews(
    Extension(store): Extension<Arc<Store>>,
    Extension(embedder): Extension<Embedder>,
    Extension(index): Extension<Arc<Index>>,
    Json(input): Json<SearchInput>,
) -> Result<Json<Vec<SearchResult>>, ApiError> {
    // Determine how many hits to return
    let k = input.top_k.unwrap_or(5);

    // 1. Embed the raw query
    let qvec = embedder.embed(&input.query);
    // 2. Search the index
    let hits = index
        .search(&qvec, k)
        .map_err(ApiError::InternalError)?;

    // 3. Map vector IDs back to metadata and build SearchResult
    let results: Vec<SearchResult> = hits
        .into_iter()
        .filter_map(|(vid, score)| {
            // load_meta returns Result<ReviewMeta, _>
            match store.load_meta(vid) {
                Ok(meta) => Some(SearchResult {
                    id: meta.id,
                    title: meta.title.clone(),
                    body: meta.body.clone(),
                    score,
                }),
                Err(_) => None,
            }
        })
        .collect();

    Ok(Json(results))
}

