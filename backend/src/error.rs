// src/error.rs

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;
use serde_json::json;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("invalid input: {0}")]
    ValidationError(String),

    #[error("internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let code = match &self {
            ApiError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ApiError::InternalError(_)   => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(json!({ "error": self.to_string() }));
        (code, body).into_response()
    }
}