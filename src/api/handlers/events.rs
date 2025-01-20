//! Events endpoint handler.
//!
//! Returns recent Windows Event Log entries.

use crate::api::AppState;
use crate::collectors::events;
use crate::models::{ApiResponse, EventInfo};
use axum::{extract::State, Json};
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for events collection (30 seconds - Get-WinEvent can be slow)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(30);

/// GET /api/v1/events
///
/// Returns recent Windows Event Log entries.
/// Configuration: events.max_age_minutes, events.max_count, events.severity
pub async fn get_events(State(state): State<AppState>) -> Json<ApiResponse<Vec<EventInfo>>> {
    debug!("Events endpoint called");

    let config = state.config.clone();

    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(move || events::collect(&config))
    ).await;

    let events = match result {
        Ok(Ok(events)) => events,
        Ok(Err(e)) => {
            warn!("Events collection task failed: {:?}", e);
            Vec::new()
        }
        Err(_) => {
            warn!("Events collection timed out after {:?}", COLLECTION_TIMEOUT);
            Vec::new()
        }
    };

    debug!("Returning {} events", events.len());
    Json(ApiResponse::new(events))
}
