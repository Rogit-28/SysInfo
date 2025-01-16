//! GPU metrics endpoint handler.

use crate::api::AppState;
use crate::models::{ApiResponse, GpuInfo};
use axum::{extract::State, Json};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for GPU collection (5 seconds)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// GET /api/v1/gpu
///
/// Returns GPU metrics for all detected graphics cards including:
/// - GPU identification (name, vendor)
/// - VRAM usage (total, used)
/// - Utilization (GPU core, memory controller)
/// - Thermal data (temperature, hotspot temp)
/// - Power and fan information
/// - Clock speeds (graphics, memory)
pub async fn get_gpu(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<GpuInfo>>> {
    debug!("GPU metrics requested");

    let collectors = Arc::clone(&state.collectors);
    
    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(move || collectors.gpu.collect())
    ).await;

    let gpus = match result {
        Ok(Ok(gpus)) => gpus,
        Ok(Err(e)) => {
            warn!("GPU collection task failed: {:?}", e);
            Vec::new()
        }
        Err(_) => {
            warn!("GPU collection timed out after {:?}", COLLECTION_TIMEOUT);
            Vec::new()
        }
    };

    Json(ApiResponse::new(gpus))
}
