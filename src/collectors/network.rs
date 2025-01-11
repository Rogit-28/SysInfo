//! Network metrics collector.
//!
//! Collects network interface information and statistics using sysinfo crate.
//! Uses periodic list refresh (every 60s) to avoid expensive interface discovery on each call.
//! Link speeds are fetched via PowerShell and cached.

use crate::models::NetworkInfo;
use parking_lot::Mutex;
use sysinfo::Networks;
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, warn};

/// How often to refresh the network interface list (expensive operation)
const LIST_REFRESH_INTERVAL: Duration = Duration::from_secs(60);

/// Network collector errors.
#[derive(Debug, Error)]
pub enum NetworkCollectorError {
    #[error("Failed to initialize network collector: {0}")]
    InitError(String),

    #[error("Failed to collect network metrics: {0}")]
    CollectionError(String),
}

/// Previous network measurement for rate calculation
#[derive(Debug, Clone)]
struct PreviousMeasurement {
    bytes_sent: u64,
    bytes_received: u64,
    timestamp: Instant,
}

/// Network metrics collector.
///
/// Collects information about network interfaces including
/// addresses, status, and traffic statistics.
pub struct NetworkCollector {
    /// sysinfo Networks instance
    networks: Mutex<Networks>,
    /// Previous measurements for rate calculation
    previous: Mutex<HashMap<String, PreviousMeasurement>>,
    /// Last time we refreshed the interface list
    last_list_refresh: Mutex<Instant>,
    /// Cached link speeds (interface name -> speed in Mbps)
    link_speeds: Mutex<HashMap<String, u64>>,
    /// Last time we refreshed link speeds
    last_speed_refresh: Mutex<Instant>,
}

impl NetworkCollector {
    /// Create a new network collector.
    pub fn new() -> Result<Self, NetworkCollectorError> {
        let networks = Networks::new_with_refreshed_list();
        let link_speeds = fetch_link_speeds();

        Ok(Self {
            networks: Mutex::new(networks),
            previous: Mutex::new(HashMap::new()),
            last_list_refresh: Mutex::new(Instant::now()),
            link_speeds: Mutex::new(link_speeds),
            last_speed_refresh: Mutex::new(Instant::now()),
        })
    }

    /// Collect metrics from all detected network interfaces.
    ///
    /// Returns a vector of `NetworkInfo` for each detected interface.
    /// Note: Interface list is only refreshed every 60 seconds for performance.
    pub fn collect(&self) -> Vec<NetworkInfo> {
        let mut networks = self.networks.lock();
        let mut previous = self.previous.lock();
        let mut last_refresh = self.last_list_refresh.lock();
        let mut link_speeds = self.link_speeds.lock();
        let mut last_speed_refresh = self.last_speed_refresh.lock();
        let now = Instant::now();

        // Only refresh the interface list periodically (expensive operation)
        if last_refresh.elapsed() >= LIST_REFRESH_INTERVAL {
            debug!("Refreshing network interface list (periodic)");
            networks.refresh_list();
            *last_refresh = now;
            
            // Also refresh link speeds when interface list is refreshed
            *link_speeds = fetch_link_speeds();
            *last_speed_refresh = now;
        }
        
        // Always refresh stats (cheap operation)
        networks.refresh();

        let mut network_list = Vec::new();

        for (name, data) in networks.iter() {
            let bytes_sent = data.total_transmitted();
            let bytes_received = data.total_received();

            // Calculate rates based on previous measurement
            let (send_rate_mbps, receive_rate_mbps) = if let Some(prev) = previous.get(name) {
                let elapsed = now.duration_since(prev.timestamp).as_secs_f64();
                if elapsed > 0.0 {
                    let sent_diff = bytes_sent.saturating_sub(prev.bytes_sent) as f64;
                    let recv_diff = bytes_received.saturating_sub(prev.bytes_received) as f64;
                    
                    // Convert bytes/sec to Mbps (megabits per second)
                    // bytes * 8 / 1_000_000 = megabits
                    let send_rate = (sent_diff / elapsed) * 8.0 / 1_000_000.0;
                    let recv_rate = (recv_diff / elapsed) * 8.0 / 1_000_000.0;
                    
                    (Some(send_rate), Some(recv_rate))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Store current measurement for next rate calculation
            previous.insert(
                name.clone(),
                PreviousMeasurement {
                    bytes_sent,
                    bytes_received,
                    timestamp: now,
                },
            );

            // Get MAC address
            let mac = {
                let mac_addr = data.mac_address();
                let mac_str = format!("{}", mac_addr);
                if mac_str == "00:00:00:00:00:00" || mac_str.is_empty() {
                    None
                } else {
                    Some(mac_str)
                }
            };

            // Get IP addresses from sysinfo
            let ip_networks = data.ip_networks();
            let mut ipv4: Option<String> = None;
            let mut ipv6: Option<String> = None;

            for ip_net in ip_networks {
                match ip_net.addr {
                    std::net::IpAddr::V4(addr) => {
                        if ipv4.is_none() {
                            ipv4 = Some(addr.to_string());
                        }
                    }
                    std::net::IpAddr::V6(addr) => {
                        if ipv6.is_none() {
                            ipv6 = Some(addr.to_string());
                        }
                    }
                }
            }

            // Determine status based on traffic (heuristic)
            // If there's recent traffic or the interface has an IP, consider it "up"
            let status = if bytes_sent > 0 || bytes_received > 0 || ipv4.is_some() || ipv6.is_some() {
                "up".to_string()
            } else {
                "down".to_string()
            };

            debug!(
                "Network: {} - TX: {} bytes, RX: {} bytes",
                name, bytes_sent, bytes_received
            );

            // Get cached link speed for this interface
            let speed_mbps = link_speeds.get(name).copied();

            network_list.push(NetworkInfo {
                name: name.clone(),
                mac,
                ipv4,
                ipv6,
                status,
                speed_mbps,
                bytes_sent,
                bytes_received,
                send_rate_mbps,
                receive_rate_mbps,
                packets_sent: data.total_packets_transmitted(),
                packets_received: data.total_packets_received(),
                rx_errors: data.total_errors_on_received(),
                tx_errors: data.total_errors_on_transmitted(),
                rx_dropped: 0, // sysinfo doesn't expose dropped packets on Windows
                tx_dropped: 0, // sysinfo doesn't expose dropped packets on Windows
            });
        }

        network_list
    }
}

/// Fetch link speeds for all network adapters via PowerShell.
/// Returns a map of interface name to speed in Mbps.
fn fetch_link_speeds() -> HashMap<String, u64> {
    let ps_script = r#"Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | Select-Object Name, LinkSpeed | ForEach-Object {
    $speed = 0
    if ($_.LinkSpeed -match '(\d+(?:\.\d+)?)\s*(Gbps|Mbps|Kbps)') {
        $value = [double]$Matches[1]
        switch ($Matches[2]) {
            'Gbps' { $speed = [uint64]($value * 1000) }
            'Mbps' { $speed = [uint64]$value }
            'Kbps' { $speed = [uint64]($value / 1000) }
        }
    }
    [PSCustomObject]@{ Name = $_.Name; SpeedMbps = $speed }
} | ConvertTo-Json -Compress"#;

    let output = match Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            warn!("Failed to fetch link speeds: {}", e);
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

    let mut speeds = HashMap::new();
    
    let items = match &json_value {
        serde_json::Value::Array(arr) => arr.clone(),
        obj @ serde_json::Value::Object(_) => vec![obj.clone()],
        _ => return HashMap::new(),
    };

    for item in items {
        if let (Some(name), Some(speed)) = (
            item.get("Name").and_then(|n| n.as_str()),
            item.get("SpeedMbps").and_then(|s| s.as_u64()),
        ) {
            if speed > 0 {
                speeds.insert(name.to_string(), speed);
            }
        }
    }

    debug!("Fetched link speeds for {} adapters", speeds.len());
    speeds
}
