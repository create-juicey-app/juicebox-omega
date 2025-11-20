use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::net::SocketAddr;
use std::sync::Arc;

use juicebox_omega::config::Config;
use juicebox_omega::state::AppState;
use juicebox_omega::server::{build_admin_router, build_public_router, print_startup_banner, start_servers};

// use mimalloc as the global allocator 
// 10-20% faster than system allocator
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    // load .env file if it exists (fails silently if not found)
    let _ = dotenvy::dotenv();

    // load configuration from environment variables
    let config = Config::from_env();

    // build tokio runtime with configured worker threads
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads)
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");

    runtime.block_on(async {
        // initialize tracing for performance monitoring
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        // create the directory if it doesn't exist
        if !config.files_dir.exists() {
            std::fs::create_dir_all(&config.files_dir).expect("Failed to create files directory");
            tracing::info!("Created files directory at: {:?}", config.files_dir);
        }

        // create shared state
        let state = Arc::new(AppState::new(config.files_dir.clone()));

        // build routers
        let public_app = build_public_router(&config.files_dir);
        let admin_app = build_admin_router(state, &config);

        // define addresses from config
        let public_addr = SocketAddr::from((
            config.public_host.parse::<std::net::IpAddr>()
                .expect("Invalid PUBLIC_HOST"),
            config.public_port
        ));
        let admin_addr = SocketAddr::from((
            config.admin_host.parse::<std::net::IpAddr>()
                .expect("Invalid ADMIN_HOST"),
            config.admin_port
        ));
        
        // print startup information
        print_startup_banner(&config);

        // start both serverssss
        start_servers(public_app, admin_app, public_addr, admin_addr).await;
    });
}
