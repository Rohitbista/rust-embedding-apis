mod auth;
mod config;
mod embedder;
mod error;
mod routes;

use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::{config::Config, embedder::Embedder};

#[derive(Clone)]
pub struct AppState {
    pub embedder: Embedder,
    pub config: Arc<Config>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Arc::new(Config::from_env());
    if config.api_key.is_none() {
        tracing::warn!("API_KEY not set: endpoints are UNPROTECTED");
    }

    tracing::info!("loading model (first run downloads it)...");
    let embedder = {
        let dir = config.cache_dir.clone();
        tokio::task::spawn_blocking(move || Embedder::new(&dir)).await??
    };
    tracing::info!("model ready");

    let state = AppState { embedder, config: config.clone() };

    let protected = Router::new()
        .route("/embed", post(routes::embed_documents))
        .route("/embed/query", post(routes::embed_query))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::require_api_key));

    let app = Router::new()
        .route("/health", get(routes::health))
        .merge(protected)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}