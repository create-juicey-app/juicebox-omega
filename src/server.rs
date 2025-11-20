use axum::{
    Router,
    routing::{get, post, delete},
    Extension,
};
use tower_http::{
    services::ServeDir,
    trace::TraceLayer,
    compression::CompressionLayer,
    limit::RequestBodyLimitLayer,
    cors::CorsLayer,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use crate::handlers::{
    batch_delete_files, delete_file, get_stats, health_check, list_files, upload_file,
    init_chunked_upload, upload_chunk, complete_chunked_upload,
};
use crate::middleware::{add_security_headers, validate_api_key};
use crate::state::AppState;
use crate::utils::shutdown_signal;
use crate::config::Config;

// build public router
pub fn build_public_router(files_dir: &PathBuf) -> Router {
    tracing::debug!("Building public router for directory: {:?}", files_dir);
    Router::new()
        .fallback_service(
            ServeDir::new(files_dir)
                .append_index_html_on_directories(true)
                .precompressed_gzip()
                .precompressed_br()
                .precompressed_deflate()
                .precompressed_zstd()
        )
        .layer(axum::middleware::from_fn(add_security_headers))
        .layer(CompressionLayer::new()
            .gzip(true)
            .br(true)
            .zstd(true)
        )
        .layer(TraceLayer::new_for_http())
}

/// build admin router
pub fn build_admin_router(state: Arc<AppState>, config: &Config) -> Router {
    tracing::debug!("Building admin router with max upload size: {} bytes", config.max_upload_size);
    
    // configure rate limiting
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2) // Burst size
            .burst_size(5)
            .finish()
            .unwrap(),
    );

    // configure cors
    let cors = CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
        ])
        .allow_origin(tower_http::cors::Any) // For development, should be stricter in prod
        .allow_headers(tower_http::cors::Any);
    // vroom vroom
    Router::new()
        .route("/admin/upload", post(upload_file))
        .route("/admin/upload/chunk/init", post(init_chunked_upload))
        .route("/admin/upload/chunk/:id/:num", post(upload_chunk))
        .route("/admin/upload/chunk/complete", post(complete_chunked_upload))
        .route("/admin/files", get(list_files))
        .route("/admin/files/:filename", delete(delete_file))
        .route("/admin/batch-delete", post(batch_delete_files))
        .route("/admin/stats", get(get_stats))
        .route("/admin/health", get(health_check))
        .layer(axum::middleware::from_fn(validate_api_key))
        .layer(Extension(config.api_key_hash.clone()))
        .layer(RequestBodyLimitLayer::new(config.max_upload_size))
        .layer(GovernorLayer { config: governor_conf })
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Start both public and admin servers
pub async fn start_servers(
    public_app: Router,
    admin_app: Router,
    public_addr: SocketAddr,
    admin_addr: SocketAddr,
) {
    tracing::info!("Starting servers...");
    
    // create listeners
    let public_listener = tokio::net::TcpListener::bind(public_addr)
        .await
        .expect("Failed to bind public server");
    
    let admin_listener = tokio::net::TcpListener::bind(admin_addr)
        .await
        .expect("Failed to bind admin server");

    tracing::debug!("Public listener bound to {}", public_addr);
    tracing::debug!("Admin listener bound to {}", admin_addr);

    // start servers
    let public_server = axum::serve(
        public_listener,
        public_app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .tcp_nodelay(true);

    let admin_server = axum::serve(
        admin_listener,
        admin_app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .tcp_nodelay(true);

    // run servers
    tracing::info!("Servers running and ready to accept connections");
    let _ = tokio::join!(
        async {
            if let Err(e) = public_server.await {
                tracing::error!("Public server error: {}", e);
            }
        },
        async {
            if let Err(e) = admin_server.await {
                tracing::error!("Admin server error: {}", e);
            }
        }
    );
}

/// print startup banner with server info
pub fn print_startup_banner(config: &Config) {
    tracing::info!("Juicebox-omega starting...");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("📡 PUBLIC FILE SERVER: http://{}:{}", config.public_host, config.public_port);
    tracing::info!("🔐 ADMIN API SERVER: http://{}:{}", config.admin_host, config.admin_port);
    tracing::info!("📁 Serving files from: {:?}", config.files_dir.canonicalize().unwrap_or(config.files_dir.clone()));
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

