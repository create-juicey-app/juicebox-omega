use std::path::PathBuf;
use sha2::{Sha256, Digest};

/// application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    /// directory to serve files from
    pub files_dir: PathBuf,
    /// public server address (file serving)
    pub public_host: String,
    /// public server port
    pub public_port: u16,
    /// admin api address
    pub admin_host: String,
    /// admin api port
    pub admin_port: u16,
    /// maximum upload size in bytes
    pub max_upload_size: usize,
    /// number of tokio worker threads
    pub worker_threads: usize,
    /// api key for admin authentication (hashed)
    pub api_key_hash: String,
    /// cors allowed origins (comma-separated)
    pub cors_origins: Vec<String>,
    /// rate limit: requests per minute
    pub rate_limit_per_minute: u64,
}

impl Config {
    /// load configuration from environment variables with defaults
    pub fn from_env() -> Self {
        // get api key from env and hash it
        let api_key = std::env::var("ADMIN_API_KEY")
            .unwrap_or_else(|_| {
                tracing::warn!("⚠️  No ADMIN_API_KEY set! Using default 'changeme' - CHANGE THIS IN PRODUCTION!");
                "changeme".to_string()
            });
        
        let api_key_hash = Self::hash_api_key(&api_key);
        
        // parse cors origins
        let cors_origins = std::env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000,http://127.0.0.1:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        Self {
            files_dir: std::env::var("FILES_DIR")
                .unwrap_or_else(|_| "./files".to_string())
                .into(),
            public_host: std::env::var("PUBLIC_HOST")
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
            public_port: std::env::var("PUBLIC_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(4848),
            admin_host: std::env::var("ADMIN_HOST")
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
            admin_port: std::env::var("ADMIN_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(4849),
            max_upload_size: std::env::var("MAX_UPLOAD_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10 * 1024 * 1024 * 1024), // 10GB default
            worker_threads: std::env::var("WORKER_THREADS")
                .ok()
                .and_then(|t| t.parse().ok())
                .unwrap_or(8),
            api_key_hash,
            cors_origins,
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .ok()
                .and_then(|r| r.parse().ok())
                .unwrap_or(60),
        }
    }
    
    // hash api key using sha256
    pub fn hash_api_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hex::encode(hasher.finalize())
    }
}

