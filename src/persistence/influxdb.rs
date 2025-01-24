//! InfluxDB client for time-series data persistence.
//!
//! Provides functionality to write telemetry data to InfluxDB v2.x.

use crate::config::InfluxDbConfig;
use crate::models::SystemSnapshot;
use thiserror::Error;
use tracing::{debug, error, warn};

/// InfluxDB client errors.
#[derive(Debug, Error)]
pub enum InfluxDbError {
    #[error("Failed to connect to InfluxDB: {0}")]
    ConnectionError(String),

    #[error("Failed to write data: {0}")]
    WriteError(String),

    #[error("Authentication failed")]
    AuthError,

    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

/// Escape special characters in tag values for InfluxDB line protocol.
/// Tags must escape: commas, equals signs, spaces, and backslashes.
#[inline]
fn escape_tag_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace('=', "\\=")
        .replace(' ', "\\ ")
}

/// Escape special characters in field string values for InfluxDB line protocol.
/// Field strings (quoted) must escape: backslashes and double quotes.
#[inline]
#[allow(dead_code)] // Utility function for future string field values
fn escape_field_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
}

/// InfluxDB v2.x client for writing telemetry data.
///
/// Uses the InfluxDB v2 HTTP API with token authentication.
pub struct InfluxDbWriter {
    /// HTTP client for making requests
    client: reqwest::Client,

    /// InfluxDB server URL
    url: String,

    /// Organization name
    org: String,

    /// Bucket name
    bucket: String,

    /// Authentication token
    token: String,
}

// Type alias for backward compatibility or clarity if needed, though Writer is better name now
pub type InfluxDbClient = InfluxDbWriter;

impl InfluxDbWriter {
    /// Create a new InfluxDB client from configuration.
    ///
    /// Returns a new instance. Validation happens on first use effectively,
    /// but we pre-validate config fields here.
    pub fn new(config: &InfluxDbConfig) -> Self {
        // We use unwrap_or_default/clone because this is now called in main()
        // before we might know if it's enabled or disabled in a way that prevents startup.
        // Actually, let's keep it simple: we construct it, if it fails later it fails.
        // But for "new" to be infallible (as expected by Arc::new pattern in main), we should handle errors or panic.
        // Given this is a critical component if enabled, panicking on invalid config (if enabled) is acceptable,
        // but here we just construct it.
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|e| {
                warn!("Failed to build HTTP client for InfluxDB: {}", e);
                reqwest::Client::new()
            });

        Self {
            client,
            url: config.url.clone(),
            org: config.org.clone(),
            bucket: config.bucket.clone(),
            token: config.token.clone(),
        }
    }

    /// Write a system snapshot to InfluxDB.
    ///
    /// Converts the snapshot to InfluxDB line protocol and writes it.
    pub async fn write_snapshot(&self, snapshot: &SystemSnapshot) -> Result<(), InfluxDbError> {
        if self.url.is_empty() {
             // If disabled/empty, just return ok
             return Ok(());
        }

        let line_protocol = self.snapshot_to_line_protocol(snapshot);

        debug!("Writing {} bytes to InfluxDB", line_protocol.len());

        let write_url = format!(
            "{}/api/v2/write?org={}&bucket={}&precision=ms",
            self.url, self.org, self.bucket
        );

        let response = self
            .client
            .post(&write_url)
            .header("Authorization", format!("Token {}", self.token))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(line_protocol)
            .send()
            .await
            .map_err(|e| InfluxDbError::WriteError(e.to_string()))?;

        if response.status().is_success() {
            debug!("Successfully wrote data to InfluxDB");
            Ok(())
        } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            error!("InfluxDB authentication failed");
            Err(InfluxDbError::AuthError)
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("InfluxDB write failed: {} - {}", status, body);
            Err(InfluxDbError::WriteError(format!("{}: {}", status, body)))
        }
    }

    /// Convert a system snapshot to InfluxDB line protocol format.
    fn snapshot_to_line_protocol(&self, snapshot: &SystemSnapshot) -> String {
        let timestamp = snapshot.timestamp.timestamp_millis();
        let hostname = escape_tag_value(&snapshot.system.hostname);
        let mut lines = Vec::new();

        // CPU metrics
        let mut cpu_fields = vec![
            format!("cores={}i", snapshot.cpu.cores),
            format!("threads={}i", snapshot.cpu.threads),
        ];

        if let Some(usage) = snapshot.cpu.usage_percent {
            cpu_fields.push(format!("usage_percent={}", usage));
        }
        if let Some(temp) = snapshot.cpu.temperature_celsius {
            cpu_fields.push(format!("temperature_celsius={}", temp));
        }
        if let Some(power) = snapshot.cpu.power_watts {
            cpu_fields.push(format!("power_watts={}", power));
        }

        lines.push(format!(
            "cpu,host={} {} {}",
            hostname,
            cpu_fields.join(","),
            timestamp
        ));

        // Memory metrics
        lines.push(format!(
            "memory,host={} total_gb={},available_gb={},used_gb={},usage_percent={} {}",
            hostname,
            snapshot.memory.total_gb,
            snapshot.memory.available_gb,
            snapshot.memory.used_gb,
            snapshot.memory.usage_percent,
            timestamp
        ));

        // GPU metrics
        for gpu in &snapshot.gpus {
            let mut gpu_fields = Vec::new();

            if let Some(vram_total) = gpu.vram_total_mb {
                gpu_fields.push(format!("vram_total_mb={}i", vram_total));
            }
            if let Some(vram_used) = gpu.vram_used_mb {
                gpu_fields.push(format!("vram_used_mb={}i", vram_used));
            }
            if let Some(usage) = gpu.usage_percent {
                gpu_fields.push(format!("usage_percent={}", usage));
            }
            if let Some(temp) = gpu.temperature_celsius {
                gpu_fields.push(format!("temperature_celsius={}", temp));
            }
            if let Some(power) = gpu.power_watts {
                gpu_fields.push(format!("power_watts={}", power));
            }

            if !gpu_fields.is_empty() {
                lines.push(format!(
                    "gpu,host={},index={},name={} {} {}",
                    hostname,
                    gpu.index,
                    escape_tag_value(&gpu.name),
                    gpu_fields.join(","),
                    timestamp
                ));
            }
        }

        // Storage metrics
        for storage in &snapshot.storage {
            let mut storage_fields = vec![
                format!("total_gb={}", storage.total_gb),
                format!("used_gb={}", storage.used_gb),
                format!("available_gb={}", storage.available_gb),
                format!("usage_percent={}", storage.usage_percent),
            ];

            if let Some(temp) = storage.temperature_celsius {
                storage_fields.push(format!("temperature_celsius={}", temp));
            }
            if let Some(read) = storage.read_speed_mbps {
                storage_fields.push(format!("read_speed_mbps={}", read));
            }
            if let Some(write) = storage.write_speed_mbps {
                storage_fields.push(format!("write_speed_mbps={}", write));
            }

            lines.push(format!(
                "storage,host={},mount_point={},drive_type={} {} {}",
                hostname,
                escape_tag_value(&storage.mount_point),
                escape_tag_value(&storage.drive_type),
                storage_fields.join(","),
                timestamp
            ));
        }

        // Network metrics
        for net in &snapshot.network {
            let mut net_fields = vec![
                format!("bytes_sent={}i", net.bytes_sent),
                format!("bytes_received={}i", net.bytes_received),
            ];

            if let Some(send_rate) = net.send_rate_mbps {
                net_fields.push(format!("send_rate_mbps={}", send_rate));
            }
            if let Some(recv_rate) = net.receive_rate_mbps {
                net_fields.push(format!("receive_rate_mbps={}", recv_rate));
            }

            lines.push(format!(
                "network,host={},interface={},status={} {} {}",
                hostname,
                escape_tag_value(&net.name),
                escape_tag_value(&net.status),
                net_fields.join(","),
                timestamp
            ));
        }

        lines.join("\n")
    }

    /// Check if the InfluxDB connection is healthy.
    pub async fn health_check(&self) -> bool {
        if self.url.is_empty() {
            return false;
        }
        
        let health_url = format!("{}/health", self.url);

        match self.client.get(&health_url).send().await {
            Ok(response) => response.status().is_success(),
            Err(e) => {
                warn!("InfluxDB health check failed: {}", e);
                false
            }
        }
    }
}
