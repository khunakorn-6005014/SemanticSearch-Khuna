// src/handlers/reviews.rs

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use spfresh::Index;


use crate::{
    error::ApiError,
    store::{ReviewMeta, Store},
    embedder::Embedder,
    AppState,
};
use spfresh::Index;

/// Payload for single review
/// Request payload for a single review insert
#[derive(Debug, Deserialize)]
pub struct NewReview {
    pub review_title: String,
    pub review_body: String,
    pub product_id: String,
    pub review_rating: i32,
}

/// Payload for bulk reviews
// #[derive(Deserialize)]
// pub struct BulkReviewInput(pub Vec<ReviewInput>);

/// Response for single insert
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub id: usize,
    pub success: bool,
}


/// Response for bulk insert
#[derive(Debug, Serialize)]
pub struct BulkInsertResponse {
    pub inserted: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

/// Request payload for search
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub top_k: Option<usize>,
}

/// One search hit in the response
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub id: usize,
    pub score: f32,
    pub review: ReviewMeta,
}



/// Single insert (already in your code)
/// POST /reviews
pub async fn insert_review(
    // Extension(store): Extension<Arc<Store>>,
    // Extension(embedder): Extension<Embedder>,
    // Extension(index): Extension<Arc<Index>>,
    State(state): State<AppState>,
    Json(payload): Json<NewReview>,
) -> Result<(StatusCode, Json<ApiResponse>), ApiError> {
    // Validate
    if payload.review_title.trim().is_empty() || payload.review_body.trim().is_empty() {
        return Err(ApiError::ValidationError("title/body cannot be empty".into()));
    }
    if payload.product_id.trim().is_empty() {
        return Err(ApiError::ValidationError("product_id cannot be empty".into()));
    }
       // Generate  embed
     let meta = ReviewMeta {
        review_title: payload.review_title,
        review_body: payload.review_body,
        product_id: payload.product_id,
        review_rating: payload.review_rating,
    };
    // Embed the content
    let to_embed = format!("{}\n\n{}", meta.review_title, meta.review_body);
    let vec = state
        .embedder
        .embed(&to_embed)
        .map_err(ApiError::InternalError)?;
    // Append to index + metadata atomically
    let assigned_id = {
        let _commit_guard = state.commit_lock.lock().unwrap();
        let mut index = state.index.lock().unwrap();
        let id_index = index.append(&vec).map_err(ApiError::InternalError)?;
        let id_meta = state.meta.append(&meta).map_err(|e| {
         ApiError::InternalError(e.into())
        })?
         if id_index != id_meta {
            return Err(ApiError::InternalError(format!(
                "id mismatch: index={id_index} meta={id_meta}"
            )));
        }

        id_index
    };

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse {
            id: assigned_id,
            success: true,
        }),
    ))

/// POST /reviews/bulk
pub async fn insert_bulk_reviews(
    // Extension(store): Extension<Arc<Store>>,
    // Extension(embedder): Extension<Embedder>,
    // Extension(index): Extension<Arc<Index>>,
    State(state): State<AppState>,
    Json(items): Json<Vec<NewReview>>,
) -> Result<Json<BulkInsertResponse>, ApiError> {
    let mut inserted = 0;
    let mut failed = 0;
    let mut errors = Vec::new();


    for (i, item) in items.into_iter().enumerate() {
        // 1. Validate non-empty
        if item.review_title.trim().is_empty() || item.review_body.trim().is_empty() {
            failed += 1;
            errors.push(format!("item {}: empty title or body", i));
            continue;
        }

        // Build meta & vector
        let meta = ReviewMeta {
            review_title: item.review_title,
            review_body: item.review_body,
            product_id: item.product_id,
            review_rating: item.review_rating,
        };
        //Embed review text
        let to_embed = format!("{} \n\n{}", meta.review_title, meta.review_body);
        let vec = match state.embedder.embed(&to_embed) {
            Ok(v) => v,
            Err(e) => {
                failed += 1;
                errors.push(format!("item {}: embed error {}", i, e));
                continue;
            }
        };
        // Append to store
        let mut index = state.index.lock().unwrap();
        match index.append(&vec) {
            Ok(id_index) => {
                match state.meta.append(&meta) {
                    Ok(id_meta) if id_index == id_meta => {
                        inserted += 1;
                    }
                    Ok(id_meta) => {
                        failed += 1;
                        errors.push(format!(
                            "item {}: id mismatch (index={} meta={})",
                            i, id_index, id_meta
                        ));
                    }
                    Err(e) => {
                        failed += 1;
                        errors.push(format!("item {}: meta append error {}", i, e));
                    }
                }
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("item {}: index append error {}", i, e));
            }
        }
    }

    Ok(Json(BulkInsertResponse { inserted, failed, errors }))
}

// POST /search
pub async fn search_reviews(
    // Extension(store): Extension<Arc<Store>>,
    // Extension(embedder): Extension<Embedder>,
    // Extension(index): Extension<Arc<Index>>,
     State(state): State<AppState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<Vec<SearchHit>>, ApiError> {
    // Determine how many hits to return
    let k = req.top_k.unwrap_or(5);

    // 1. Embed the raw query
    let qvec = state.embedder
        .embed(&req.query)
        .map_err(ApiError::InternalError)?;
    // 2. Search the index
    let hits = {
        let index = state.index.lock().unwrap();
        index.search(&qvec, k).map_err(ApiError::InternalError)?
    };
    // 3. Map vector IDs back to metadata and build SearchResult
    let mut out = Vec::with_capacity(hits.len());
    for (id, score) in hits {
        let review = state.meta.get(id).map_err(ApiError::InternalError)?;
        out.push(SearchHit { id, score, review });
    }

    Ok(Json(out))
}



