// Caching: TTL-based cache for disk I/O stats
//! Storage metrics collector.
//!
//! Collects storage device and volume information using sysinfo crate.
//! Uses periodic list refresh (every 60s) to avoid expensive disk discovery on each call.
//! I/O stats are fetched via WMI performance counters.
//! Temperature is fetched via Storage Reliability Counters (SMART data).

use crate::models::StorageInfo;
use parking_lot::Mutex;
use sysinfo::Disks;
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, warn};

/// How often to refresh the disk list (expensive operation)
const LIST_REFRESH_INTERVAL: Duration = Duration::from_secs(60);

/// How often to refresh temperature data (moderately expensive)
const TEMP_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

/// Storage collector errors.
#[derive(Debug, Error)]
pub enum StorageCollectorError {
    #[error("Failed to initialize storage collector: {0}")]
    InitError(String),

    #[error("Failed to collect storage metrics: {0}")]
    CollectionError(String),
}

/// Bytes to gigabytes conversion constant
const BYTES_TO_GB: f64 = 1_073_741_824.0;

/// Disk I/O statistics from WMI
#[derive(Debug, Clone, Default)]
struct DiskIoStats {
    read_bytes_per_sec: Option<f64>,
    write_bytes_per_sec: Option<f64>,
}

/// Storage metrics collector.
///
/// Collects information about storage devices and volumes including
/// capacity, usage, type, and I/O statistics.
pub struct StorageCollector {
    /// sysinfo Disks instance
    disks: Mutex<Disks>,
    /// Last time we refreshed the disk list
    last_list_refresh: Mutex<Instant>,
    /// Cached I/O stats per drive letter
    io_stats: Mutex<HashMap<String, DiskIoStats>>,
    /// Last time we fetched I/O stats
    last_io_refresh: Mutex<Instant>,
    /// Cached temperature per drive letter
    temperatures: Mutex<HashMap<String, f64>>,
    /// Last time we fetched temperatures
    last_temp_refresh: Mutex<Instant>,
}

impl StorageCollector {
    /// Create a new storage collector.
    pub fn new() -> Result<Self, StorageCollectorError> {
        let disks = Disks::new_with_refreshed_list();

        Ok(Self {
            disks: Mutex::new(disks),
            last_list_refresh: Mutex::new(Instant::now()),
            io_stats: Mutex::new(HashMap::new()),
            last_io_refresh: Mutex::new(Instant::now() - Duration::from_secs(10)), // Force initial fetch
            temperatures: Mutex::new(HashMap::new()),
            last_temp_refresh: Mutex::new(Instant::now() - Duration::from_secs(60)), // Force initial fetch
        })
    }

    /// Collect metrics from all detected storage devices/volumes.
    ///
    /// Returns a vector of `StorageInfo` for each detected volume.
    /// Note: Disk list is only refreshed every 60 seconds for performance.
    pub fn collect(&self) -> Vec<StorageInfo> {
        let mut disks = self.disks.lock();
        let mut last_refresh = self.last_list_refresh.lock();
        let mut io_stats = self.io_stats.lock();
        let mut last_io_refresh = self.last_io_refresh.lock();
        let mut temperatures = self.temperatures.lock();
        let mut last_temp_refresh = self.last_temp_refresh.lock();
        let now = Instant::now();

        // Only refresh the disk list periodically (expensive operation)
        if last_refresh.elapsed() >= LIST_REFRESH_INTERVAL {
            debug!("Refreshing disk list (periodic)");
            disks.refresh_list();
            *last_refresh = now;
        }
        
        // Always refresh stats (cheap operation)
        disks.refresh();

        // Refresh I/O stats every 2 seconds
        if last_io_refresh.elapsed() >= Duration::from_secs(2) {
            *io_stats = fetch_disk_io_stats();
            *last_io_refresh = now;
        }

        // Refresh temperatures every 30 seconds (moderately expensive)
        if last_temp_refresh.elapsed() >= TEMP_REFRESH_INTERVAL {
            *temperatures = fetch_disk_temperatures();
            *last_temp_refresh = now;
        }

        let mut storage_list = Vec::new();

        for disk in disks.iter() {
            let name = disk.name().to_string_lossy().to_string();
            let mount_point = disk.mount_point().to_string_lossy().to_string();

            // Get disk type
            let drive_type = match disk.kind() {
                sysinfo::DiskKind::SSD => "SSD".to_string(),
                sysinfo::DiskKind::HDD => "HDD".to_string(),
                sysinfo::DiskKind::Unknown(_) => "Unknown".to_string(),
            };

            // Get capacity information
            let total_bytes = disk.total_space();
            let available_bytes = disk.available_space();
            let used_bytes = total_bytes.saturating_sub(available_bytes);

            let total_gb = total_bytes as f64 / BYTES_TO_GB;
            let available_gb = available_bytes as f64 / BYTES_TO_GB;
            let used_gb = used_bytes as f64 / BYTES_TO_GB;

            // Calculate usage percentage
            let usage_percent = if total_bytes > 0 {
                (used_bytes as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };

            debug!(
                "Disk: {} ({}) - {:.1} GB / {:.1} GB ({:.1}%)",
                mount_point, drive_type, used_gb, total_gb, usage_percent
            );

            // Get I/O stats for this drive (keyed by drive letter, e.g., "C:")
            let drive_key = if mount_point.len() >= 2 {
                mount_point[..2].to_uppercase()
            } else {
                mount_point.clone()
            };
            let io = io_stats.get(&drive_key).cloned().unwrap_or_default();
            let temp = temperatures.get(&drive_key).copied();

            storage_list.push(StorageInfo {
                name: if name.is_empty() { mount_point.clone() } else { name },
                mount_point,
                drive_type,
                total_gb,
                used_gb,
                available_gb,
                usage_percent,
                temperature_celsius: temp,
                read_speed_mbps: io.read_bytes_per_sec.map(|b| b / 1_000_000.0),
                write_speed_mbps: io.write_bytes_per_sec.map(|b| b / 1_000_000.0),
                read_bytes_total: None,    // WMI provides per-sec, not totals
                write_bytes_total: None,   // WMI provides per-sec, not totals
                read_bytes_per_sec: io.read_bytes_per_sec,
                write_bytes_per_sec: io.write_bytes_per_sec,
            });
        }

        storage_list
    }
}

/// Fetch disk I/O statistics via WMI performance counters.
/// Returns a map of drive letter (e.g., "C:") to I/O stats.
fn fetch_disk_io_stats() -> HashMap<String, DiskIoStats> {
    let ps_script = r#"Get-CimInstance -ClassName Win32_PerfFormattedData_PerfDisk_LogicalDisk | 
Where-Object { $_.Name -match '^[A-Z]:$' } | 
Select-Object Name, DiskReadBytesPersec, DiskWriteBytesPersec | 
ConvertTo-Json -Compress"#;

    let output = match Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            warn!("Failed to fetch disk I/O stats: {}", e);
            return HashMap::new();
        }
    };

    if !output.status.success() {
        return HashMap::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    
    if trimmed.is_empty() {
        return HashMap::new();
    }

    let json_value: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    let mut stats = HashMap::new();
    
    let items = match &json_value {
        serde_json::Value::Array(arr) => arr.clone(),
        obj @ serde_json::Value::Object(_) => vec![obj.clone()],
        _ => return HashMap::new(),
    };

    for item in items {
        if let Some(name) = item.get("Name").and_then(|n| n.as_str()) {
            let read_bytes = item.get("DiskReadBytesPersec")
                .and_then(|v| v.as_u64())
                .map(|v| v as f64);
            let write_bytes = item.get("DiskWriteBytesPersec")
                .and_then(|v| v.as_u64())
                .map(|v| v as f64);
            
            stats.insert(name.to_uppercase(), DiskIoStats {
                read_bytes_per_sec: read_bytes,
                write_bytes_per_sec: write_bytes,
            });
        }
    }

    debug!("Fetched I/O stats for {} drives", stats.len());
    stats
}

/// Fetch disk temperatures via Storage Reliability Counters (SMART data).
/// Returns a map of drive letter (e.g., "C:") to temperature in Celsius.
/// Note: This requires administrator privileges to access SMART data.
fn fetch_disk_temperatures() -> HashMap<String, f64> {
    // Use Get-PhysicalDisk with Get-StorageReliabilityCounter for SMART temperature
    // Then map physical disks to drive letters via partitions
    // This requires admin privileges - will silently return empty if not elevated
    let ps_script = r#"
$temps = @{}
try {
    $physicalDisks = Get-PhysicalDisk -ErrorAction Stop
    foreach ($disk in $physicalDisks) {
        $reliability = $disk | Get-StorageReliabilityCounter -ErrorAction Stop
        if ($reliability -and $reliability.Temperature -gt 0) {
            $partitions = Get-Partition -DiskNumber $disk.DeviceId -ErrorAction SilentlyContinue
            foreach ($part in $partitions) {
                if ($part.DriveLetter) {
                    $temps["$($part.DriveLetter):"] = $reliability.Temperature
                }
            }
        }
    }
} catch {
    # Requires admin - silently fail
}
if ($temps.Count -gt 0) { $temps | ConvertTo-Json -Compress } else { '{}' }
"#;

    let output = match Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", ps_script])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            warn!("Failed to fetch disk temperatures: {}", e);
            return HashMap::new();
        }
    };

    if !output.status.success() {
        return HashMap::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    
    if trimmed.is_empty() || trimmed == "{}" {
        return HashMap::new();
    }

    let json_value: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    let mut temps = HashMap::new();
    
    if let serde_json::Value::Object(map) = json_value {
        for (key, value) in map {
            if let Some(temp) = value.as_f64().or_else(|| value.as_i64().map(|i| i as f64)) {
                temps.insert(key.to_uppercase(), temp);
            }
        }
    }

    if !temps.is_empty() {
        debug!("Fetched temperatures for {} drives", temps.len());
    }
    temps
}

