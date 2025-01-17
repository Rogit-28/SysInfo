//! Storage metrics endpoint handler.

use crate::api::AppState;
use crate::models::{ApiResponse, StorageInfo};
use axum::{extract::State, Json};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for storage collection (5 seconds)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// GET /api/v1/storage
///
/// Returns storage metrics for all detected drives/volumes including:
/// - Device identification (name, mount point, type)
/// - Capacity information (total, used, available)
/// - Usage percentage
/// - Performance data (read/write speeds) if available
/// - Temperature if available via SMART
pub async fn get_storage(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<StorageInfo>>> {
    debug!("Storage metrics requested");

    let collectors = Arc::clone(&state.collectors);
    
    // Check if storage collector is available
    let has_storage = collectors.storage.is_some();
    if !has_storage {
        warn!("Storage collector not available");
        return Json(ApiResponse::new(Vec::new()));
    }
    
    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            collectors.storage.as_ref().map(|c| c.collect()).unwrap_or_default()
        })
    ).await;

    let storage = match result {
        Ok(Ok(storage)) => storage,
        Ok(Err(e)) => {
            warn!("Storage collection task failed: {:?}", e);
            Vec::new()
        }
        Err(_) => {
            warn!("Storage collection timed out after {:?}", COLLECTION_TIMEOUT);
            Vec::new()
        }
    };

    Json(ApiResponse::new(storage))
}
