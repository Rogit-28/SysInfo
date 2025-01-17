//! Network metrics endpoint handler.

use crate::api::AppState;
use crate::models::{ApiResponse, NetworkInfo};
use axum::{extract::State, Json};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for network collection (5 seconds)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// GET /api/v1/network
///
/// Returns network metrics for all detected interfaces including:
/// - Interface identification (name, MAC address)
/// - IP addresses (IPv4, IPv6)
/// - Interface status and speed
/// - Traffic statistics (bytes sent/received)
/// - Current transfer rates
pub async fn get_network(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<NetworkInfo>>> {
    debug!("Network metrics requested");

    let collectors = Arc::clone(&state.collectors);
    
    // Check if network collector is available
    let has_network = collectors.network.is_some();
    if !has_network {
        warn!("Network collector not available");
        return Json(ApiResponse::new(Vec::new()));
    }
    
    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            collectors.network.as_ref().map(|c| c.collect()).unwrap_or_default()
        })
    ).await;

    let network = match result {
        Ok(Ok(network)) => network,
        Ok(Err(e)) => {
            warn!("Network collection task failed: {:?}", e);
            Vec::new()
        }
        Err(_) => {
            warn!("Network collection timed out after {:?}", COLLECTION_TIMEOUT);
            Vec::new()
        }
    };

    Json(ApiResponse::new(network))
}
