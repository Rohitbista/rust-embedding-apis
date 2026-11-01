use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    embedder::{BGE_QUERY_PREFIX, DIMENSIONS, MODEL_NAME},
    error::AppError,
    AppState,
};

pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "model": MODEL_NAME, "dimensions": DIMENSIONS }))
}

#[derive(Deserialize)]
pub struct EmbedRequest {
    pub texts: Vec<String>,
}

#[derive(Serialize)]
pub struct EmbedResponse {
    pub model: &'static str,
    pub dimensions: usize,
    pub embeddings: Vec<Vec<f32>>,
}

/// POST /embed: documents / notes (no prefix)
pub async fn embed_documents(
    State(state): State<AppState>,
    Json(req): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>, AppError> {
    if req.texts.is_empty() {
        return Err(AppError::BadRequest("`texts` must not be empty".into()));
    }
    if req.texts.len() > state.config.max_batch {
        return Err(AppError::BadRequest(format!(
            "batch too large (max {})",
            state.config.max_batch
        )));
    }

    let embeddings = state.embedder.embed(req.texts).await?;
    Ok(Json(EmbedResponse { model: MODEL_NAME, dimensions: DIMENSIONS, embeddings }))
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub text: String,
}

#[derive(Serialize)]
pub struct QueryResponse {
    pub model: &'static str,
    pub dimensions: usize,
    pub embedding: Vec<f32>,
}

/// POST /embed/query: search queries (BGE query prefix added server-side)
pub async fn embed_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    if req.text.trim().is_empty() {
        return Err(AppError::BadRequest("`text` must not be empty".into()));
    }

    let prefixed = format!("{BGE_QUERY_PREFIX}{}", req.text);
    let mut out = state.embedder.embed(vec![prefixed]).await?;
    let embedding = out
        .pop()
        .ok_or_else(|| AppError::Internal("no embedding returned".into()))?;

    Ok(Json(QueryResponse { model: MODEL_NAME, dimensions: DIMENSIONS, embedding }))
}