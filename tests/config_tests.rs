use juicebox_omega::config::Config;
use std::env;

// helper to clear env vars
fn clear_env() {
    env::remove_var("FILES_DIR");
    env::remove_var("PUBLIC_HOST");
    env::remove_var("PUBLIC_PORT");
    env::remove_var("ADMIN_HOST");
    env::remove_var("ADMIN_PORT");
    env::remove_var("MAX_UPLOAD_SIZE");
    env::remove_var("WORKER_THREADS");
    env::remove_var("ADMIN_API_KEY");
    env::remove_var("CORS_ORIGINS");
    env::remove_var("RATE_LIMIT_PER_MINUTE");
}

#[test]
fn test_hash_api_key() {
    let key = "secret";
    let hash = Config::hash_api_key(key);
    // sha256 hex string is 64 chars
    assert_eq!(hash.len(), 64);
    
    // deterministic
    assert_eq!(hash, Config::hash_api_key(key));
    
    // different keys produce different hashes
    assert_ne!(hash, Config::hash_api_key("other"));
}

#[test]
fn test_config_behavior() {
    // Run these sequentially to avoid race conditions with environment variables
    
    // 1. Test Defaults
    clear_env();
    
    let config = Config::from_env();
    
    assert_eq!(config.files_dir.to_str().unwrap(), "./files");
    assert_eq!(config.public_host, "127.0.0.1");
    assert_eq!(config.public_port, 4848);
    assert_eq!(config.admin_port, 4849);
    assert_eq!(config.worker_threads, 8);
    assert_eq!(config.rate_limit_per_minute, 60);
    
    let expected_hash = Config::hash_api_key("changeme");
    assert_eq!(config.api_key_hash, expected_hash);

    // 2. Test From Env
    clear_env();
    
    env::set_var("FILES_DIR", "/tmp/test_files");
    env::set_var("PUBLIC_PORT", "9090");
    env::set_var("WORKER_THREADS", "4");
    env::set_var("ADMIN_API_KEY", "supersecret");
    
    let config = Config::from_env();
    
    assert_eq!(config.files_dir.to_str().unwrap(), "/tmp/test_files");
    assert_eq!(config.public_port, 9090);
    assert_eq!(config.worker_threads, 4);
    
    let expected_hash = Config::hash_api_key("supersecret");
    assert_eq!(config.api_key_hash, expected_hash);
    
    // Cleanup
    clear_env();
}
