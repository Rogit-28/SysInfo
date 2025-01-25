//! Windows-specific implementations.
//!
//! Contains Windows API wrappers and utilities for hardware data collection.
//!
//! # Overview
//!
//! This module provides abstractions over various Windows APIs:
//! - Performance Data Helper (PDH) for performance counters
//! - Windows Management Instrumentation (WMI) for system queries
//! - IP Helper API for network information
//! - GlobalMemoryStatusEx for memory information
//! - Power Information APIs for CPU power data

use std::collections::HashMap;
use tracing::{debug, warn};
use wmi::{COMLibrary, Variant, WMIConnection};

/// Result type for Windows platform operations
pub type WindowsResult<T> = Result<T, WindowsError>;

/// Windows platform operation errors
#[derive(Debug, thiserror::Error)]
pub enum WindowsError {
    #[error("COM initialization failed: {0}")]
    ComError(String),

    #[error("WMI connection failed: {0}")]
    WmiConnectionError(String),

    #[error("WMI query failed: {0}")]
    WmiQueryError(String),

    #[error("Windows API error: {0}")]
    ApiError(String),
}

/// WMI helper for querying Windows Management Instrumentation.
pub struct WmiHelper {
    connection: WMIConnection,
}

impl WmiHelper {
    /// Create a new WMI helper with the default namespace (ROOT\CIMV2).
    pub fn new() -> WindowsResult<Self> {
        let com = COMLibrary::new()
            .map_err(|e| WindowsError::ComError(e.to_string()))?;
        
        let connection = WMIConnection::new(com)
            .map_err(|e| WindowsError::WmiConnectionError(e.to_string()))?;

        Ok(Self { connection })
    }

    /// Create a new WMI helper with a specific namespace.
    pub fn with_namespace(namespace: &str) -> WindowsResult<Self> {
        let com = COMLibrary::new()
            .map_err(|e| WindowsError::ComError(e.to_string()))?;
        
        let connection = WMIConnection::with_namespace_path(namespace, com)
            .map_err(|e| WindowsError::WmiConnectionError(e.to_string()))?;

        Ok(Self { connection })
    }

    /// Execute a raw WMI query and return results as a vector of HashMaps.
    pub fn query(&self, wql: &str) -> WindowsResult<Vec<HashMap<String, Variant>>> {
        self.connection
            .raw_query(wql)
            .map_err(|e| WindowsError::WmiQueryError(e.to_string()))
    }

    /// Execute a WMI query and extract a specific field as u32.
    pub fn query_u32(&self, wql: &str, field: &str) -> WindowsResult<Option<u32>> {
        let results = self.query(wql)?;
        
        if let Some(result) = results.first() {
            if let Some(variant) = result.get(field) {
                return Ok(variant_to_u32(variant));
            }
        }
        
        Ok(None)
    }

    /// Execute a WMI query and extract a specific field as String.
    pub fn query_string(&self, wql: &str, field: &str) -> WindowsResult<Option<String>> {
        let results = self.query(wql)?;
        
        if let Some(result) = results.first() {
            if let Some(variant) = result.get(field) {
                return Ok(variant_to_string(variant));
            }
        }
        
        Ok(None)
    }
}

/// Convert a WMI Variant to u32 if possible.
fn variant_to_u32(variant: &Variant) -> Option<u32> {
    match variant {
        Variant::UI1(v) => Some(*v as u32),
        Variant::UI2(v) => Some(*v as u32),
        Variant::UI4(v) => Some(*v),
        Variant::UI8(v) => Some(*v as u32),
        Variant::I1(v) => Some(*v as u32),
        Variant::I2(v) => Some(*v as u32),
        Variant::I4(v) => Some(*v as u32),
        Variant::I8(v) => Some(*v as u32),
        _ => None,
    }
}

/// Convert a WMI Variant to String if possible.
fn variant_to_string(variant: &Variant) -> Option<String> {
    match variant {
        Variant::String(s) => Some(s.clone()),
        Variant::UI1(v) => Some(v.to_string()),
        Variant::UI2(v) => Some(v.to_string()),
        Variant::UI4(v) => Some(v.to_string()),
        Variant::UI8(v) => Some(v.to_string()),
        Variant::I1(v) => Some(v.to_string()),
        Variant::I2(v) => Some(v.to_string()),
        Variant::I4(v) => Some(v.to_string()),
        Variant::I8(v) => Some(v.to_string()),
        _ => None,
    }
}

/// Get system uptime in seconds using Windows GetTickCount64.
pub fn get_system_uptime_seconds() -> u64 {
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::GetTickCount64;
        // GetTickCount64 returns milliseconds since system start
        unsafe { GetTickCount64() / 1000 }
    }
    
    #[cfg(not(windows))]
    {
        0
    }
}

/// Get the system hostname.
pub fn get_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|e| {
            warn!("Failed to get hostname: {}", e);
            "Unknown".to_string()
        })
}

/// Get operating system information.
pub fn get_os_info() -> (String, String) {
    let os_name = std::env::consts::OS.to_string();
    
    // Try to get Windows version from WMI
    let os_version = match WmiHelper::new() {
        Ok(wmi) => {
            match wmi.query_string("SELECT Version FROM Win32_OperatingSystem", "Version") {
                Ok(Some(version)) => version,
                _ => "Unknown".to_string(),
            }
        }
        Err(e) => {
            debug!("Failed to get OS version from WMI: {}", e);
            "Unknown".to_string()
        }
    };

    (os_name, os_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hostname() {
        let hostname = get_hostname();
        assert!(!hostname.is_empty());
    }

    #[test]
    fn test_get_uptime() {
        let uptime = get_system_uptime_seconds();
        // System should have been up for at least a few seconds
        assert!(uptime > 0);
    }
}
