//! Memory metrics endpoint handler.

use crate::api::AppState;
use crate::models::{ApiResponse, MemoryInfo};
use axum::{extract::State, Json};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for memory collection (5 seconds)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// GET /api/v1/memory
///
/// Returns system memory metrics including:
/// - Total physical memory (GB)
/// - Available memory (GB)
/// - Used memory (GB)
/// - Usage percentage
pub async fn get_memory(
    State(state): State<AppState>,
) -> Json<ApiResponse<MemoryInfo>> {
    debug!("Memory metrics requested");

    let collectors = Arc::clone(&state.collectors);
    
    // Check if memory collector is available
    let has_memory = collectors.memory.is_some();
    if !has_memory {
        warn!("Memory collector not available");
        return Json(ApiResponse::new(MemoryInfo::default()));
    }
    
    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            collectors.memory.as_ref().map(|c| c.collect()).unwrap_or_default()
        })
    ).await;

    let memory = match result {
        Ok(Ok(memory)) => memory,
        Ok(Err(e)) => {
            warn!("Memory collection task failed: {:?}", e);
            MemoryInfo::default()
        }
        Err(_) => {
            warn!("Memory collection timed out after {:?}", COLLECTION_TIMEOUT);
            MemoryInfo::default()
        }
    };

    Json(ApiResponse::new(memory))
}
