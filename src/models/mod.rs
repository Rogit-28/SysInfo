use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuInfo {
    pub name: String, pub vendor: String, pub cores: u32, pub threads: u32,
    pub architecture: String, pub base_clock_mhz: Option<u32>,
    pub usage_percent: Option<f64>, pub per_core_usage: Option<Vec<f64>>,
    pub per_core_frequency_mhz: Option<Vec<u32>>,
    pub temperature_celsius: Option<f64>, pub power_watts: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GpuInfo {
    pub index: u32, pub name: String, pub vendor: String,
    pub vram_total_mb: Option<u64>, pub vram_used_mb: Option<u64>,
    pub usage_percent: Option<f64>, pub memory_usage_percent: Option<f64>,
    pub temperature_celsius: Option<f64>, pub hotspot_temp_celsius: Option<f64>,
    pub power_watts: Option<f64>, pub fan_speed_percent: Option<f64>,
    pub clock_graphics_mhz: Option<u32>, pub clock_memory_mhz: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryInfo {
    pub total_gb: f64, pub available_gb: f64, pub used_gb: f64, pub usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageInfo {
    pub name: String, pub mount_point: String, pub drive_type: String,
    pub total_gb: f64, pub used_gb: f64, pub available_gb: f64, pub usage_percent: f64,
    pub temperature_celsius: Option<f64>, pub read_speed_mbps: Option<f64>,
    pub write_speed_mbps: Option<f64>, pub read_bytes_total: Option<u64>,
    pub write_bytes_total: Option<u64>, pub read_bytes_per_sec: Option<f64>,
    pub write_bytes_per_sec: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInfo {
    pub name: String, pub mac: Option<String>, pub ipv4: Option<String>,
    pub ipv6: Option<String>, pub status: String, pub speed_mbps: Option<u64>,
    pub bytes_sent: u64, pub bytes_received: u64,
    pub send_rate_mbps: Option<f64>, pub receive_rate_mbps: Option<f64>,
    pub packets_sent: u64, pub packets_received: u64,
    pub rx_errors: u64, pub tx_errors: u64, pub rx_dropped: u64, pub tx_dropped: u64,
}
