//! CPU metrics endpoint handler.

use crate::api::AppState;
use crate::models::{ApiResponse, CpuInfo};
use axum::{extract::State, Json};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for CPU collection (5 seconds)
const COLLECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// GET /api/v1/cpu
///
/// Returns current CPU metrics including:
/// - CPU identification (name, vendor, cores, threads)
/// - Usage statistics (total and per-core)
/// - Frequency information (base and per-core current)
/// - Thermal data (temperature, power)
pub async fn get_cpu(
    State(state): State<AppState>,
) -> Json<ApiResponse<CpuInfo>> {
    debug!("CPU metrics requested");

    let collectors = Arc::clone(&state.collectors);
    
    // Check if CPU collector is available
    let has_cpu = collectors.cpu.is_some();
    if !has_cpu {
        warn!("CPU collector not available");
        return Json(ApiResponse::new(CpuInfo::default()));
    }
    
    let result = tokio::time::timeout(
        COLLECTION_TIMEOUT,
        tokio::task::spawn_blocking(move || {
            collectors.cpu.as_ref().map(|c| c.collect()).unwrap_or_default()
        })
    ).await;

    let cpu = match result {
        Ok(Ok(cpu)) => cpu,
        Ok(Err(e)) => {
            warn!("CPU collection task failed: {:?}", e);
            CpuInfo::default()
        }
        Err(_) => {
            warn!("CPU collection timed out after {:?}", COLLECTION_TIMEOUT);
            CpuInfo::default()
        }
    };

    Json(ApiResponse::new(cpu))
}
