use std::sync::{Arc, Mutex};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::error::AppError;

pub const MODEL_NAME: &str = "BAAI/bge-small-en-v1.5";
pub const DIMENSIONS: usize = 384;
pub const BGE_QUERY_PREFIX: &str = "Represent this sentence for searching relevant passages: ";

/// `embed` needs `&mut self`, so the model lives behind a Mutex.
/// Inference is CPU-bound, so it runs on the blocking thread pool.
#[derive(Clone)]
pub struct Embedder {
    model: Arc<Mutex<TextEmbedding>>,
}

impl Embedder {
    pub fn new(cache_dir: &str) -> anyhow::Result<Self> {
        let mut model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15)
                .with_cache_dir(cache_dir.into())
                .with_show_download_progress(true),
        )?;

        // Warmup so the ONNX session is fully initialised before serving traffic.
        model.embed(vec!["warmup"], None)?;

        Ok(Self { model: Arc::new(Mutex::new(model)) })
    }

    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        let model = self.model.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = model
                .lock()
                .map_err(|_| AppError::Internal("model mutex poisoned".into()))?;
            guard
                .embed(texts, Some(32))
                .map_err(|e| AppError::Internal(e.to_string()))
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
    }
}