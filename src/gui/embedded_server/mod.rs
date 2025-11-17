// Embedded Web Server module for EGUI integration

pub mod server_handle;
pub mod metrics;
pub mod middleware;
pub mod config_export;
pub mod system_monitor;
pub mod network_utils;

pub use server_handle::ServerHandle;
pub use metrics::{ServerMetrics, MetricsSnapshot, ResponseTimeRecord, ErrorLog};
pub use middleware::{metrics_middleware, logging_middleware, RequestLogger, RequestLog};
pub use config_export::{ConfigExportResponse, ConfigExportQuery, export_config_handler};
pub use system_monitor::{SystemMonitor, SystemInfo};
pub use network_utils::{get_local_ip, get_all_local_ips, format_server_address};

use axum::Router;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Start the embedded web server in a background task
pub async fn start_embedded_server(
    metrics: Arc<ServerMetrics>,
    logger: Arc<RequestLogger>,
    shutdown_rx: oneshot::Receiver<()>,
    host: String,
    port: u16,
) -> anyhow::Result<()> {
    use axum::routing::get;
    use tower_http::cors::{Any, CorsLayer};
    
    // Create the router with middleware
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .layer(axum::middleware::from_fn(move |req, next| {
            let metrics = metrics.clone();
            async move {
                middleware::metrics_middleware(req, next, metrics).await
            }
        }))
        .layer(axum::middleware::from_fn(move |req, next| {
            let logger = logger.clone();
            async move {
                middleware::logging_middleware(req, next, logger).await
            }
        }))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );
    
    // Bind to address
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    log::info!("Embedded web server listening on {}", addr);
    
    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_rx.await.ok();
            log::info!("Shutting down embedded web server");
        })
        .await?;
    
    Ok(())
}
