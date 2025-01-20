//! Windows services collector.
//!
//! Collects Windows service status using PowerShell as a reliable cross-version approach.

use crate::models::ServiceInfo;
use std::process::Command;
use tracing::{debug, warn};

/// Services collector using PowerShell.
pub struct ServiceCollector;

/// Sanitize service name to prevent command injection.
/// Only allows alphanumeric characters, hyphens, underscores, and dots.
fn sanitize_service_name(name: &str) -> Option<String> {
    if name.is_empty() || name.len() > 256 {
        return None;
    }
    
    if name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
        Some(name.to_string())
    } else {
        warn!("Rejecting service name with invalid characters: {}", name);
        None
    }
}

impl ServiceCollector {
    /// Create a new service collector.
    pub fn new() -> Self {
        Self
    }

    /// Collect Windows services status.
    /// If watchlist is empty, returns all services.
    /// If watchlist is provided, only returns those services.
    pub fn collect(&self, watchlist: &[String], include_stopped: bool) -> Vec<ServiceInfo> {
        // Use PowerShell with CIM to get service information including PID
        let ps_script = if watchlist.is_empty() {
            r#"Get-CimInstance -ClassName Win32_Service | Select-Object Name, DisplayName, State, StartMode, ProcessId | ForEach-Object {
    [PSCustomObject]@{
        Name = $_.Name
        DisplayName = $_.DisplayName
        Status = $_.State
        StartType = $_.StartMode
        Pid = if ($_.ProcessId -and $_.ProcessId -gt 0) { $_.ProcessId } else { $null }
    }
} | ConvertTo-Json -Compress"#.to_string()
        } else {
            // Sanitize all service names to prevent command injection
            let sanitized: Vec<String> = watchlist.iter()
                .filter_map(|n| sanitize_service_name(n))
                .collect();
            
            if sanitized.is_empty() {
                warn!("All service names in watchlist were rejected due to invalid characters");
                return Vec::new();
            }
            
            let names = sanitized.iter()
                .map(|n| format!("'{}'", n))
                .collect::<Vec<_>>()
                .join(",");
            format!(r#"Get-CimInstance -ClassName Win32_Service -Filter "Name IN ({})" | Select-Object Name, DisplayName, State, StartMode, ProcessId | ForEach-Object {{
    [PSCustomObject]@{{
        Name = $_.Name
        DisplayName = $_.DisplayName
        Status = $_.State
        StartType = $_.StartMode
        Pid = if ($_.ProcessId -and $_.ProcessId -gt 0) {{ $_.ProcessId }} else {{ $null }}
    }}
}} | ConvertTo-Json -Compress"#, names)
        };

        let output = match Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                warn!("Failed to run PowerShell for services: {}", e);
                return Vec::new();
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("PowerShell command failed: {}", stderr);
            return Vec::new();
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse JSON output
        let json_value: serde_json::Value = match serde_json::from_str(&stdout) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to parse services JSON: {}", e);
                return Vec::new();
            }
        };
        
        let services: Vec<ServiceInfo> = match json_value {
            serde_json::Value::Array(arr) => {
                arr.iter()
                    .filter_map(|v| parse_service(v))
                    .filter(|s| include_stopped || s.status == "Running")
                    .collect()
            }
            serde_json::Value::Object(_) => {
                // Single service returns as object, not array
                if let Some(s) = parse_service(&json_value) {
                    if include_stopped || s.status == "Running" {
                        vec![s]
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        };

        debug!("Collected {} services", services.len());
        services
    }
}

fn parse_service(v: &serde_json::Value) -> Option<ServiceInfo> {
    let name = v.get("Name")?.as_str()?.to_string();
    let display_name = v.get("DisplayName")?.as_str()?.to_string();
    
    // Status from Win32_Service is a string (State field)
    let status = v.get("Status")
        .and_then(|s| s.as_str())
        .unwrap_or("Unknown")
        .to_string();

    // StartType from Win32_Service (StartMode field) is a string
    let start_type = v.get("StartType")
        .and_then(|s| s.as_str())
        .unwrap_or("Unknown")
        .to_string();

    // PID for running services
    let pid = v.get("Pid")
        .and_then(|p| p.as_u64())
        .map(|p| p as u32);

    Some(ServiceInfo {
        name,
        display_name,
        status,
        start_type,
        pid,
    })
}
