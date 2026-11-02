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

use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    pub embedder: Embedder,
    pub config: Arc<Config>,
}

#[shuttle_runtime::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

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
        .with_state(state)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(OpenApi)]
#[openapi(
    paths(routes::health, routes::embed_documents, routes::embed_query),
    components(schemas(
        routes::EmbedRequest, routes::EmbedResponse,
        routes::QueryRequest, routes::QueryResponse,
        error::ErrorBody
    )),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

// Adds the "Authorize" button for the x-api-key header
struct SecurityAddon;
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-api-key"))),
        );
    }
}