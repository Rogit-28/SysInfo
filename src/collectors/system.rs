//! System info collector.
//!
//! Collects general system information like hostname, OS, uptime, etc.

use crate::models::SystemInfo;
use sysinfo::System;

/// Collect system information.
pub fn collect_system_info() -> SystemInfo {
    let uptime = System::uptime();
    let boot_time = System::boot_time();
    
    SystemInfo {
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        os: System::name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".to_string()),
        uptime_seconds: uptime,
        boot_time,
        architecture: std::env::consts::ARCH.to_string(),
    }
}
