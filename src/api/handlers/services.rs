//! Services endpoint handler.
//!
//! Returns Windows services status.

use crate::api::AppState;
use crate::models::{ApiResponse, ServiceInfo};
use axum::{extract::State, Json};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for services collection (30 seconds - PowerShell can be slow)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(30);

/// GET /api/v1/services
///
/// Returns Windows services status.
/// Configuration: services.watchlist (empty = all), services.include_stopped
pub async fn get_services(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ServiceInfo>>> {
    debug!("Services endpoint called");

    let watchlist = state.config.services.watchlist.clone();
    let include_stopped = state.config.services.include_stopped;
    let collectors = Arc::clone(&state.collectors);

    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            collectors.services.collect(&watchlist, include_stopped)
        })
    ).await;

    let services = match result {
        Ok(Ok(services)) => services,
        Ok(Err(e)) => {
            warn!("Services collection task failed: {:?}", e);
            Vec::new()
        }
        Err(_) => {
            warn!("Services collection timed out after {:?}", COLLECTION_TIMEOUT);
            Vec::new()
        }
    };

    debug!("Returning {} services", services.len());
    Json(ApiResponse::new(services))
}
