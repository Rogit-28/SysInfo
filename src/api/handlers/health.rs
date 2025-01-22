//! Health check endpoint handler.
//!
//! Provides service health status including uptime, collector status,
//! and InfluxDB connection status.

use crate::api::AppState;
use crate::models::{ApiResponse, CollectorStatus, HealthResponse};
use axum::{extract::State, Json};
use tracing::debug;

/// GET /api/v1/health
///
/// Returns the health status of the service including:
/// - Overall service status (healthy/degraded/unhealthy)
/// - Service uptime in seconds
/// - Status of individual collectors
/// - InfluxDB connection status
pub async fn get_health(
    State(state): State<AppState>,
) -> Json<ApiResponse<HealthResponse>> {
    debug!("Health check requested");

    // Check InfluxDB health async
    let influxdb_healthy = if state.config.influxdb.enabled {
        state.influxdb.health_check().await
    } else {
        false
    };

    // Build collector status list - now checking Option status
    let cpu_available = state.collectors.cpu.is_some();
    let memory_available = state.collectors.memory.is_some();
    let storage_available = state.collectors.storage.is_some();
    let network_available = state.collectors.network.is_some();
    
    let collector_statuses = vec![
        CollectorStatus {
            name: "cpu".to_string(),
            enabled: state.config.collection.collect_cpu,
            healthy: cpu_available,
            message: if cpu_available { None } else { Some("CPU collector failed to initialize".to_string()) },
        },
        CollectorStatus {
            name: "gpu".to_string(),
            enabled: state.config.collection.collect_gpu,
            healthy: state.collectors.gpu.is_available(),
            message: if state.collectors.gpu.is_available() { None } else { Some("No NVIDIA GPU detected".to_string()) },
        },
        CollectorStatus {
            name: "memory".to_string(),
            enabled: state.config.collection.collect_memory,
            healthy: memory_available,
            message: if memory_available { None } else { Some("Memory collector failed to initialize".to_string()) },
        },
        CollectorStatus {
            name: "storage".to_string(),
            enabled: state.config.collection.collect_storage,
            healthy: storage_available,
            message: if storage_available { None } else { Some("Storage collector failed to initialize".to_string()) },
        },
        CollectorStatus {
            name: "network".to_string(),
            enabled: state.config.collection.collect_network,
            healthy: network_available,
            message: if network_available { None } else { Some("Network collector failed to initialize".to_string()) },
        },
    ];

    // Determine overall health status
    let critical_collectors_healthy = cpu_available && memory_available;
    let status = if !critical_collectors_healthy {
        "unhealthy"
    } else if state.config.influxdb.enabled && !influxdb_healthy {
        "degraded"
    } else {
        "healthy"
    };

    let health = HealthResponse {
        status: status.to_string(),
        uptime_seconds: state.uptime_seconds(),
        collectors: collector_statuses,
        influxdb_connected: influxdb_healthy,
    };

    Json(ApiResponse::new(health))
}
