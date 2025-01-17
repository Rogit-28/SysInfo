//! Processes endpoint handler.
//!
//! Returns top N processes by CPU or memory usage.

use crate::api::AppState;
use crate::models::{ApiResponse, ProcessInfo};
use axum::{extract::State, Json};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for process collection (10 seconds)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(10);

/// GET /api/v1/processes
///
/// Returns top N processes sorted by CPU or memory usage.
/// Configuration is read from settings (processes.top_n, processes.sort_by).
pub async fn get_processes(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ProcessInfo>>> {
    debug!("Processes endpoint called");

    let top_n = state.config.processes.top_n;
    let sort_by = state.config.processes.sort_by.clone();
    let collectors = Arc::clone(&state.collectors);

    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            collectors.processes.collect(top_n, &sort_by)
        })
    ).await;

    let processes = match result {
        Ok(Ok(processes)) => processes,
        Ok(Err(e)) => {
            warn!("Process collection task failed: {:?}", e);
            Vec::new()
        }
        Err(_) => {
            warn!("Process collection timed out after {:?}", COLLECTION_TIMEOUT);
            Vec::new()
        }
    };

    debug!("Returning {} processes", processes.len());
    Json(ApiResponse::new(processes))
}
