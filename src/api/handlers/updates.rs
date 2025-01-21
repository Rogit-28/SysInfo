//! Updates endpoint handler.
//!
//! Returns Windows Update status.

use crate::api::AppState;
use crate::collectors::updates;
use crate::models::{ApiResponse, UpdateSummary};
use axum::{extract::State, Json};
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for updates collection (60 seconds - Windows Update queries are slow)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(60);

/// GET /api/v1/updates
///
/// Returns Windows Update status including pending updates and reboot status.
pub async fn get_updates(State(_state): State<AppState>) -> Json<ApiResponse<UpdateSummary>> {
    debug!("Updates endpoint called");

    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(updates::collect)
    ).await;

    let summary = match result {
        Ok(Ok(summary)) => summary,
        Ok(Err(e)) => {
            warn!("Updates collection task failed: {:?}", e);
            UpdateSummary::default()
        }
        Err(_) => {
            warn!("Updates collection timed out after {:?}", COLLECTION_TIMEOUT);
            UpdateSummary::default()
        }
    };

    debug!(
        "Returning update summary: {} pending, reboot_pending={}",
        summary.pending_count, summary.reboot_pending
    );
    Json(ApiResponse::new(summary))
}
