//! Data models for the SysInfo telemetry system.
//!
//! This module contains all the data structures used throughout the application
//! for representing hardware metrics, API responses, and system information.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// CPU Information
// ============================================================================

/// Comprehensive CPU metrics including identification, usage, and thermal data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    /// CPU model name (e.g., "AMD Ryzen 9 5900X")
    pub name: String,

    /// CPU vendor (e.g., "AMD", "Intel")
    pub vendor: String,

    /// Number of physical cores
    pub cores: u32,

    /// Number of logical threads (including hyperthreading)
    pub threads: u32,

    /// CPU architecture (e.g., "x86_64", "ARM64")
    pub architecture: String,

    /// Base clock frequency in MHz
    pub base_clock_mhz: Option<u32>,

    /// Overall CPU usage percentage (0-100)
    pub usage_percent: Option<f64>,

    /// Per-core usage percentages (0-100 for each core)
    pub per_core_usage: Option<Vec<f64>>,

    /// Per-core current frequency in MHz
    pub per_core_frequency_mhz: Option<Vec<u32>>,

    /// CPU package temperature in Celsius
    pub temperature_celsius: Option<f64>,

    /// CPU package power consumption in Watts
    pub power_watts: Option<f64>,
}

impl Default for CpuInfo {
    fn default() -> Self {
        Self {
            name: String::from("Unknown"),
            vendor: String::from("Unknown"),
            cores: 0,
            threads: 0,
            architecture: String::from("Unknown"),
            base_clock_mhz: None,
            usage_percent: None,
            per_core_usage: None,
            per_core_frequency_mhz: None,
            temperature_celsius: None,
            power_watts: None,
        }
    }
}

// ============================================================================
// GPU Information
// ============================================================================

/// GPU metrics for a single graphics card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// GPU index (0-based, for multi-GPU systems)
    pub index: u32,

    /// GPU model name (e.g., "NVIDIA GeForce RTX 4090")
    pub name: String,

    /// GPU vendor (e.g., "NVIDIA", "AMD", "Intel")
    pub vendor: String,

    /// Total VRAM capacity in megabytes
    pub vram_total_mb: Option<u64>,

    /// Used VRAM in megabytes
    pub vram_used_mb: Option<u64>,

    /// GPU core utilization percentage (0-100)
    pub usage_percent: Option<f64>,

    /// GPU memory controller utilization percentage (0-100)
    pub memory_usage_percent: Option<f64>,

    /// GPU core temperature in Celsius
    pub temperature_celsius: Option<f64>,

    /// GPU hotspot temperature in Celsius (if available)
    pub hotspot_temp_celsius: Option<f64>,

    /// GPU power consumption in Watts
    pub power_watts: Option<f64>,

    /// Fan speed as percentage (0-100)
    pub fan_speed_percent: Option<f64>,

    /// Current graphics clock in MHz
    pub clock_graphics_mhz: Option<u32>,

    /// Current memory clock in MHz
    pub clock_memory_mhz: Option<u32>,
}

impl Default for GpuInfo {
    fn default() -> Self {
        Self {
            index: 0,
            name: String::from("Unknown"),
            vendor: String::from("Unknown"),
            vram_total_mb: None,
            vram_used_mb: None,
            usage_percent: None,
            memory_usage_percent: None,
            temperature_celsius: None,
            hotspot_temp_celsius: None,
            power_watts: None,
            fan_speed_percent: None,
            clock_graphics_mhz: None,
            clock_memory_mhz: None,
        }
    }
}

// ============================================================================
// Memory Information
// ============================================================================

/// System memory (RAM) metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total physical memory in gigabytes
    pub total_gb: f64,

    /// Available (free) memory in gigabytes
    pub available_gb: f64,

    /// Used memory in gigabytes
    pub used_gb: f64,

    /// Memory usage percentage (0-100)
    pub usage_percent: f64,
}

impl Default for MemoryInfo {
    fn default() -> Self {
        Self {
            total_gb: 0.0,
            available_gb: 0.0,
            used_gb: 0.0,
            usage_percent: 0.0,
        }
    }
}

// ============================================================================
// Storage Information
// ============================================================================

/// Storage device/volume metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Device or volume name
    pub name: String,

    /// Mount point or drive letter (e.g., "C:\")
    pub mount_point: String,

    /// Drive type (e.g., "SSD", "HDD", "NVMe", "Removable")
    pub drive_type: String,

    /// Total capacity in gigabytes
    pub total_gb: f64,

    /// Used space in gigabytes
    pub used_gb: f64,

    /// Available space in gigabytes
    pub available_gb: f64,

    /// Storage usage percentage (0-100)
    pub usage_percent: f64,

    /// Drive temperature in Celsius (if available via SMART)
    pub temperature_celsius: Option<f64>,

    /// Current read speed in MB/s (requires PDH counters on Windows)
    pub read_speed_mbps: Option<f64>,

    /// Current write speed in MB/s (requires PDH counters on Windows)
    pub write_speed_mbps: Option<f64>,

    /// Total bytes read since boot
    pub read_bytes_total: Option<u64>,

    /// Total bytes written since boot
    pub write_bytes_total: Option<u64>,

    /// Current read rate in bytes/sec (delta-based)
    pub read_bytes_per_sec: Option<f64>,

    /// Current write rate in bytes/sec (delta-based)
    pub write_bytes_per_sec: Option<f64>,
}

impl Default for StorageInfo {
    fn default() -> Self {
        Self {
            name: String::from("Unknown"),
            mount_point: String::new(),
            drive_type: String::from("Unknown"),
            total_gb: 0.0,
            used_gb: 0.0,
            available_gb: 0.0,
            usage_percent: 0.0,
            temperature_celsius: None,
            read_speed_mbps: None,
            write_speed_mbps: None,
            read_bytes_total: None,
            write_bytes_total: None,
            read_bytes_per_sec: None,
            write_bytes_per_sec: None,
        }
    }
}

// ============================================================================
// Network Information
// ============================================================================

/// Network interface metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Interface name/description
    pub name: String,

    /// MAC address (if available)
    pub mac: Option<String>,

    /// IPv4 address (if assigned)
    pub ipv4: Option<String>,

    /// IPv6 address (if assigned)
    pub ipv6: Option<String>,

    /// Interface status ("up" or "down")
    pub status: String,

    /// Link speed in Mbps
    pub speed_mbps: Option<u64>,

    /// Total bytes sent since boot
    pub bytes_sent: u64,

    /// Total bytes received since boot
    pub bytes_received: u64,

    /// Current send rate in Mbps
    pub send_rate_mbps: Option<f64>,

    /// Current receive rate in Mbps
    pub receive_rate_mbps: Option<f64>,

    /// Total packets sent since boot
    pub packets_sent: u64,

    /// Total packets received since boot
    pub packets_received: u64,

    /// Total receive errors since boot
    pub rx_errors: u64,

    /// Total transmit errors since boot
    pub tx_errors: u64,

    /// Total received packets dropped since boot
    pub rx_dropped: u64,

    /// Total transmitted packets dropped since boot
    pub tx_dropped: u64,
}

impl Default for NetworkInfo {
    fn default() -> Self {
        Self {
            name: String::from("Unknown"),
            mac: None,
            ipv4: None,
            ipv6: None,
            status: String::from("unknown"),
            speed_mbps: None,
            bytes_sent: 0,
            bytes_received: 0,
            send_rate_mbps: None,
            receive_rate_mbps: None,
            packets_sent: 0,
            packets_received: 0,
            rx_errors: 0,
            tx_errors: 0,
            rx_dropped: 0,
            tx_dropped: 0,
        }
    }
}

// ============================================================================
// System Information
// ============================================================================

/// General system/OS information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Machine hostname
    pub hostname: String,

    /// Operating system name (e.g., "Windows")
    pub os: String,

    /// OS version string (e.g., "10.0.22621")
    pub os_version: String,

    /// Kernel/build version
    pub kernel_version: String,

    /// System uptime in seconds
    pub uptime_seconds: u64,

    /// System boot timestamp (Unix timestamp)
    pub boot_time: u64,

    /// CPU architecture (e.g., "x86_64")
    pub architecture: String,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            hostname: String::from("Unknown"),
            os: String::from("Unknown"),
            os_version: String::from("Unknown"),
            kernel_version: String::from("Unknown"),
            uptime_seconds: 0,
            boot_time: 0,
            architecture: String::from("Unknown"),
        }
    }
}

// ============================================================================
// Process Information
// ============================================================================

/// Process metrics for a single running process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: u32,

    /// Process name
    pub name: String,

    /// CPU usage percentage (0-100)
    pub cpu_percent: f32,

    /// Memory usage in MB
    pub memory_mb: f64,

    /// Virtual memory size in MB
    pub virtual_memory_mb: f64,

    /// Process status
    pub status: String,

    /// Process start time (Unix timestamp)
    pub start_time: u64,

    /// User who owns the process
    pub user: Option<String>,

    /// Command line used to start the process
    pub cmd: Option<String>,
}

impl Default for ProcessInfo {
    fn default() -> Self {
        Self {
            pid: 0,
            name: String::from("Unknown"),
            cpu_percent: 0.0,
            memory_mb: 0.0,
            virtual_memory_mb: 0.0,
            status: String::from("Unknown"),
            start_time: 0,
            user: None,
            cmd: None,
        }
    }
}

// ============================================================================
// Windows Service Information
// ============================================================================

/// Windows service status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name (internal name)
    pub name: String,

    /// Service display name
    pub display_name: String,

    /// Current status: "running", "stopped", "paused", "start_pending", "stop_pending", etc.
    pub status: String,

    /// Start type: "auto", "manual", "disabled", "auto_delayed"
    pub start_type: String,

    /// Process ID if service is running
    pub pid: Option<u32>,
}

impl Default for ServiceInfo {
    fn default() -> Self {
        Self {
            name: String::from("Unknown"),
            display_name: String::from("Unknown"),
            status: String::from("Unknown"),
            start_type: String::from("Unknown"),
            pid: None,
        }
    }
}

// ============================================================================
// API Response Wrappers
// ============================================================================

/// Generic API response wrapper providing consistent response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// ISO 8601 UTC timestamp of the response
    pub timestamp: DateTime<Utc>,

    /// Response payload
    pub data: T,
}

impl<T> ApiResponse<T> {
    /// Create a new API response with current timestamp.
    pub fn new(data: T) -> Self {
        Self {
            timestamp: Utc::now(),
            data,
        }
    }
}

/// API error response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code for programmatic handling
    pub code: String,

    /// Human-readable error message
    pub message: String,
}

impl ApiError {
    /// Create a new API error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// API error response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    /// ISO 8601 UTC timestamp of the error
    pub timestamp: DateTime<Utc>,

    /// Error details
    pub error: ApiError,
}

impl ApiErrorResponse {
    /// Create a new API error response with current timestamp.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            error: ApiError::new(code, message),
        }
    }
}
