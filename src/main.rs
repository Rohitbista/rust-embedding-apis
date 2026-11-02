use shuttle_runtime::{CustomError, SecretStore};

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> shuttle_axum::ShuttleAxum {
    let config = Arc::new(Config::from_secrets(&secrets));
    if config.api_key.is_none() {
        tracing::warn!("API_KEY not set: endpoints are UNPROTECTED");
    }

    tracing::info!("loading model (first run downloads it)...");
    let embedder = {
        let dir = config.cache_dir.clone();
        tokio::task::spawn_blocking(move || Embedder::new(&dir))
            .await
            .map_err(CustomError::new)?
            .map_err(CustomError::new)?
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

    Ok(app.into())
}