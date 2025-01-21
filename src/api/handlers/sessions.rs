//! Sessions endpoint handler.
//!
//! Returns active user sessions.

use crate::api::AppState;
use crate::collectors::sessions;
use crate::models::{ApiResponse, SessionInfo};
use axum::{extract::State, Json};
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for sessions collection (15 seconds)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(15);

/// GET /api/v1/sessions
///
/// Returns active user sessions (console, RDP, etc.).
pub async fn get_sessions(State(_state): State<AppState>) -> Json<ApiResponse<Vec<SessionInfo>>> {
    debug!("Sessions endpoint called");

    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(sessions::collect)
    ).await;

    let sessions = match result {
        Ok(Ok(sessions)) => sessions,
        Ok(Err(e)) => {
            warn!("Sessions collection task failed: {:?}", e);
            Vec::new()
        }
        Err(_) => {
            warn!("Sessions collection timed out after {:?}", COLLECTION_TIMEOUT);
            Vec::new()
        }
    };

    debug!("Returning {} sessions", sessions.len());
    Json(ApiResponse::new(sessions))
}
