//! System snapshot endpoint handler.
//!
//! Returns a complete system telemetry snapshot with all hardware metrics.

use crate::api::AppState;
use crate::models::{ApiResponse, CpuInfo, MemoryInfo, NetworkInfo, StorageInfo, SystemInfo, SystemSnapshot};
use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Default timeout for individual collector (5 seconds)
const COLLECTOR_TIMEOUT: Duration = Duration::from_secs(5);

/// GET /api/v1/system
///
/// Returns a complete system snapshot including all hardware metrics:
/// - CPU information and usage
/// - GPU information and usage (all detected GPUs)
/// - Memory usage
/// - Storage devices and usage
/// - Network interfaces and statistics
/// - System information (hostname, OS, uptime)
pub async fn get_system(
    State(state): State<AppState>,
) -> Json<ApiResponse<SystemSnapshot>> {
    debug!("System snapshot requested - collecting all metrics in parallel");

    // Clone collectors for each parallel task
    let collectors_cpu = Arc::clone(&state.collectors);
    let collectors_gpu = Arc::clone(&state.collectors);
    let collectors_memory = Arc::clone(&state.collectors);
    let collectors_storage = Arc::clone(&state.collectors);
    let collectors_network = Arc::clone(&state.collectors);

    // Spawn all collection tasks in parallel
    let cpu_task = tokio::spawn(async move {
        tokio::time::timeout(
            COLLECTOR_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                collectors_cpu.cpu.as_ref().map(|c| c.collect()).unwrap_or_default()
            })
        ).await
    });

    let gpu_task = tokio::spawn(async move {
        tokio::time::timeout(
            COLLECTOR_TIMEOUT,
            tokio::task::spawn_blocking(move || collectors_gpu.gpu.collect())
        ).await
    });

    let memory_task = tokio::spawn(async move {
        tokio::time::timeout(
            COLLECTOR_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                collectors_memory.memory.as_ref().map(|c| c.collect()).unwrap_or_default()
            })
        ).await
    });

    let storage_task = tokio::spawn(async move {
        tokio::time::timeout(
            COLLECTOR_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                collectors_storage.storage.as_ref().map(|c| c.collect()).unwrap_or_default()
            })
        ).await
    });

    let network_task = tokio::spawn(async move {
        tokio::time::timeout(
            COLLECTOR_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                collectors_network.network.as_ref().map(|c| c.collect()).unwrap_or_default()
            })
        ).await
    });

    let system_task = tokio::spawn(async move {
        tokio::time::timeout(
            COLLECTOR_TIMEOUT,
            tokio::task::spawn_blocking(crate::collectors::system::collect_system_info)
        ).await
    });

    // Await all tasks and handle results
    let (cpu_result, gpu_result, memory_result, storage_result, network_result, system_result) = 
        tokio::join!(cpu_task, gpu_task, memory_task, storage_task, network_task, system_task);

    // Process CPU result
    let cpu = match cpu_result {
        Ok(Ok(Ok(cpu))) => cpu,
        Ok(Ok(Err(e))) => {
            warn!("CPU collection task panicked: {:?}", e);
            CpuInfo::default()
        }
        Ok(Err(_)) => {
            warn!("CPU collection timed out");
            CpuInfo::default()
        }
        Err(e) => {
            warn!("CPU task join error: {:?}", e);
            CpuInfo::default()
        }
    };

    // Process GPU result
    let gpus = match gpu_result {
        Ok(Ok(Ok(gpus))) => gpus,
        Ok(Ok(Err(e))) => {
            warn!("GPU collection task panicked: {:?}", e);
            Vec::new()
        }
        Ok(Err(_)) => {
            warn!("GPU collection timed out");
            Vec::new()
        }
        Err(e) => {
            warn!("GPU task join error: {:?}", e);
            Vec::new()
        }
    };

    // Process Memory result
    let memory = match memory_result {
        Ok(Ok(Ok(memory))) => memory,
        Ok(Ok(Err(e))) => {
            warn!("Memory collection task panicked: {:?}", e);
            MemoryInfo::default()
        }
        Ok(Err(_)) => {
            warn!("Memory collection timed out");
            MemoryInfo::default()
        }
        Err(e) => {
            warn!("Memory task join error: {:?}", e);
            MemoryInfo::default()
        }
    };

    // Process Storage result
    let storage: Vec<StorageInfo> = match storage_result {
        Ok(Ok(Ok(storage))) => storage,
        Ok(Ok(Err(e))) => {
            warn!("Storage collection task panicked: {:?}", e);
            Vec::new()
        }
        Ok(Err(_)) => {
            warn!("Storage collection timed out");
            Vec::new()
        }
        Err(e) => {
            warn!("Storage task join error: {:?}", e);
            Vec::new()
        }
    };

    // Process Network result
    let network: Vec<NetworkInfo> = match network_result {
        Ok(Ok(Ok(network))) => network,
        Ok(Ok(Err(e))) => {
            warn!("Network collection task panicked: {:?}", e);
            Vec::new()
        }
        Ok(Err(_)) => {
            warn!("Network collection timed out");
            Vec::new()
        }
        Err(e) => {
            warn!("Network task join error: {:?}", e);
            Vec::new()
        }
    };

    // Process System info result
    let system: SystemInfo = match system_result {
        Ok(Ok(Ok(system))) => system,
        Ok(Ok(Err(e))) => {
            warn!("System info collection task panicked: {:?}", e);
            SystemInfo::default()
        }
        Ok(Err(_)) => {
            warn!("System info collection timed out");
            SystemInfo::default()
        }
        Err(e) => {
            warn!("System info task join error: {:?}", e);
            SystemInfo::default()
        }
    };

    let snapshot = SystemSnapshot {
        timestamp: Utc::now(),
        cpu,
        gpus,
        memory,
        storage,
        network,
        system,
    };

    debug!("System snapshot collection complete");
    Json(ApiResponse::new(snapshot))
}
