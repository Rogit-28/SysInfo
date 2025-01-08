//! Memory metrics collector.
//!
//! Collects system memory (RAM) usage using sysinfo crate.

use crate::models::MemoryInfo;
use parking_lot::Mutex;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};
use thiserror::Error;

/// Memory collector errors.
#[derive(Debug, Error)]
pub enum MemoryCollectorError {
    #[error("Failed to initialize memory collector: {0}")]
    InitError(String),

    #[error("Failed to collect memory metrics: {0}")]
    CollectionError(String),
}

/// Bytes to gigabytes conversion constant
const BYTES_TO_GB: f64 = 1_073_741_824.0;

/// Memory metrics collector.
///
/// Collects system memory usage statistics using sysinfo.
pub struct MemoryCollector {
    /// sysinfo System instance for memory metrics
    system: Mutex<System>,
}

impl MemoryCollector {
    /// Create a new memory collector.
    pub fn new() -> Result<Self, MemoryCollectorError> {
        let system = System::new_with_specifics(
            RefreshKind::new().with_memory(MemoryRefreshKind::everything()),
        );

        Ok(Self {
            system: Mutex::new(system),
        })
    }

    /// Collect current memory metrics.
    ///
    /// Returns a `MemoryInfo` struct with memory usage data.
    pub fn collect(&self) -> MemoryInfo {
        let mut sys = self.system.lock();

        // Refresh memory information
        sys.refresh_memory();

        // Get memory values in bytes
        let total_bytes = sys.total_memory();
        let available_bytes = sys.available_memory();
        let used_bytes = total_bytes.saturating_sub(available_bytes);

        // Convert to gigabytes
        let total_gb = total_bytes as f64 / BYTES_TO_GB;
        let available_gb = available_bytes as f64 / BYTES_TO_GB;
        let used_gb = used_bytes as f64 / BYTES_TO_GB;

        // Calculate usage percentage
        let usage_percent = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        MemoryInfo {
            total_gb,
            available_gb,
            used_gb,
            usage_percent,
        }
    }
}
