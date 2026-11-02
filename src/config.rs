use shuttle_runtime::SecretStore;

#[derive(Clone)]
pub struct Config {
    pub api_key: Option<String>,
    pub cache_dir: String,
    pub max_batch: usize,
}

impl Config {
    pub fn from_secrets(secrets: &SecretStore) -> Self {
        Self {
            api_key: secrets.get("API_KEY").filter(|k| !k.is_empty()),
            cache_dir: secrets
                .get("MODEL_CACHE_DIR")
                .unwrap_or_else(|| "./models".into()),
            max_batch: secrets
                .get("MAX_BATCH")
                .and_then(|p| p.parse().ok())
                .unwrap_or(128),
        }
    }
}