//! HTTP server setup and configuration.
//!
//! Contains the application state and server initialization logic.

use crate::collectors::CollectorRegistry;
use crate::config::Settings;
use crate::persistence::InfluxDbWriter;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Shared application state accessible by all request handlers.
#[derive(Clone)]
pub struct AppState {
    /// Application configuration
    pub config: Arc<Settings>,

    /// Registry of hardware data collectors
    /// Wrapped in Arc directly, collectors themselves handle internal mutability if needed
    pub collectors: Arc<CollectorRegistry>,

    /// InfluxDB client for data persistence (optional)
    pub influxdb: Arc<InfluxDbWriter>,

    /// Server start time for uptime calculation
    pub start_time: Instant,
}

impl AppState {
    /// Create a new application state.
    pub fn new(
        config: Arc<Settings>,
        collectors: Arc<CollectorRegistry>,
        influxdb: Arc<InfluxDbWriter>,
    ) -> Self {
        Self {
            config,
            collectors,
            influxdb,
            start_time: Instant::now(),
        }
    }

    /// Get the server uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Start the HTTP server with the given application state.
///
/// This function will block until the server is shut down.
pub async fn start_server(state: AppState) -> anyhow::Result<()> {
    let addr: SocketAddr = format!("{}:{}", state.config.server.host, state.config.server.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid server address: {}", e))?;

    let router = super::create_router(state);

    info!("Starting HTTP server on {}", addr);

    let listener = TcpListener::bind(addr).await.map_err(|e| {
        error!("Failed to bind to {}: {}", addr, e);
        anyhow::anyhow!("Failed to bind to {}: {}", addr, e)
    })?;

    info!("Server listening on http://{}", addr);
    info!("API endpoints available at http://{}/api/v1/", addr);
    
    // Set up graceful shutdown
    let graceful_shutdown = async {
        if let Ok(_) = tokio::signal::ctrl_c().await {
            info!("Received shutdown signal. Stopping server...");
        }
    };

    axum::serve(listener, router)
        .with_graceful_shutdown(graceful_shutdown)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
