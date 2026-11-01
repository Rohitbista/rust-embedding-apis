use std::env;

#[derive(Clone)]
pub struct Config {
    pub port: u16,
    pub api_key: Option<String>,
    pub cache_dir: String,
    pub max_batch: usize,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080),
            api_key: env::var("API_KEY").ok().filter(|k| !k.is_empty()),
            cache_dir: env::var("MODEL_CACHE_DIR").unwrap_or_else(|_| "./models".into()),
            max_batch: env::var("MAX_BATCH").ok().and_then(|p| p.parse().ok()).unwrap_or(128),
        }
    }
}