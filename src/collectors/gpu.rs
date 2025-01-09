//! GPU metrics collector.
//!
//! Collects GPU metrics using NVML (NVIDIA Management Library) for NVIDIA GPUs.
//! Provides detailed GPU telemetry including utilization, temperature, power,
//! memory usage, and clock speeds.

use crate::models::GpuInfo;
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use nvml_wrapper::Nvml;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// GPU collector errors.
#[derive(Debug, Error)]
pub enum GpuCollectorError {
    #[error("Failed to initialize GPU collector: {0}")]
    InitError(String),

    #[error("Failed to collect GPU metrics: {0}")]
    CollectionError(String),

    #[error("No GPUs detected")]
    NoGpusFound,

    #[error("NVML error: {0}")]
    NvmlError(String),
}

/// GPU metrics collector.
///
/// Supports NVIDIA GPUs via NVML. The collector initializes NVML on creation
/// and queries GPU metrics on each collect() call.
pub struct GpuCollector {
    /// NVML instance for GPU queries
    nvml: Option<Nvml>,
    /// Number of detected NVIDIA GPUs
    gpu_count: u32,
}

impl GpuCollector {
    /// Create a new GPU collector.
    ///
    /// Attempts to initialize NVML and detect available NVIDIA GPUs.
    /// If NVML initialization fails (e.g., no NVIDIA driver installed),
    /// the collector will still be created but will return empty results.
    pub fn new() -> Result<Self, GpuCollectorError> {
        match Nvml::init() {
            Ok(nvml) => {
                // Get device count
                match nvml.device_count() {
                    Ok(count) => {
                        info!("GPU collector initialized: found {} NVIDIA GPU(s)", count);
                        Ok(Self {
                            nvml: Some(nvml),
                            gpu_count: count,
                        })
                    }
                    Err(e) => {
                        warn!("NVML initialized but failed to get device count: {}", e);
                        Ok(Self {
                            nvml: Some(nvml),
                            gpu_count: 0,
                        })
                    }
                }
            }
            Err(e) => {
                // NVML init failed - this is expected if no NVIDIA GPU/driver
                warn!(
                    "NVML initialization failed (no NVIDIA GPU or driver?): {}",
                    e
                );
                info!("GPU collector initialized without NVML support");
                Ok(Self {
                    nvml: None,
                    gpu_count: 0,
                })
            }
        }
    }

    /// Check if NVML is available and GPUs are detected.
    pub fn is_available(&self) -> bool {
        self.nvml.is_some() && self.gpu_count > 0
    }

    /// Get the number of detected GPUs.
    pub fn gpu_count(&self) -> u32 {
        self.gpu_count
    }

    /// Collect metrics from all detected GPUs.
    ///
    /// Returns a vector of `GpuInfo` for each detected NVIDIA GPU.
    /// Returns an empty vector if no GPUs are available.
    pub fn collect(&self) -> Vec<GpuInfo> {
        let nvml = match &self.nvml {
            Some(n) => n,
            None => {
                debug!("No NVML available, returning empty GPU list");
                return Vec::new();
            }
        };

        if self.gpu_count == 0 {
            debug!("No GPUs detected");
            return Vec::new();
        }

        let mut gpus = Vec::with_capacity(self.gpu_count as usize);

        for i in 0..self.gpu_count {
            match self.collect_gpu_info(nvml, i) {
                Ok(info) => gpus.push(info),
                Err(e) => {
                    error!("Failed to collect metrics for GPU {}: {}", i, e);
                    // Add a placeholder with error indication
                    gpus.push(GpuInfo {
                        index: i,
                        name: format!("GPU {} (error)", i),
                        vendor: "NVIDIA".to_string(),
                        ..Default::default()
                    });
                }
            }
        }

        gpus
    }

    /// Collect metrics for a single GPU by index.
    fn collect_gpu_info(&self, nvml: &Nvml, index: u32) -> Result<GpuInfo, GpuCollectorError> {
        let device = nvml
            .device_by_index(index)
            .map_err(|e| GpuCollectorError::NvmlError(format!("Failed to get device {}: {}", index, e)))?;

        // Get GPU name
        let name = device
            .name()
            .unwrap_or_else(|_| format!("NVIDIA GPU {}", index));

        // Get memory info (total and used)
        let (vram_total_mb, vram_used_mb, memory_usage_percent) = match device.memory_info() {
            Ok(mem) => {
                let total_mb = mem.total / (1024 * 1024);
                let used_mb = mem.used / (1024 * 1024);
                let usage = if mem.total > 0 {
                    (mem.used as f64 / mem.total as f64) * 100.0
                } else {
                    0.0
                };
                (Some(total_mb), Some(used_mb), Some(usage))
            }
            Err(e) => {
                debug!("Failed to get memory info for GPU {}: {}", index, e);
                (None, None, None)
            }
        };

        // Get GPU utilization
        let usage_percent = match device.utilization_rates() {
            Ok(util) => Some(util.gpu as f64),
            Err(e) => {
                debug!("Failed to get utilization for GPU {}: {}", index, e);
                None
            }
        };

        // Get temperature
        let temperature_celsius = match device.temperature(TemperatureSensor::Gpu) {
            Ok(temp) => Some(temp as f64),
            Err(e) => {
                debug!("Failed to get temperature for GPU {}: {}", index, e);
                None
            }
        };

        // Get power usage (in milliwatts, convert to watts)
        let power_watts = match device.power_usage() {
            Ok(power_mw) => Some(power_mw as f64 / 1000.0),
            Err(e) => {
                debug!("Failed to get power usage for GPU {}: {}", index, e);
                None
            }
        };

        // Get fan speed (percentage)
        // Note: Laptop GPUs may not support fan speed queries
        let fan_speed_percent = match device.num_fans() {
            Ok(num_fans) if num_fans > 0 => {
                // Try to get fan speed for the first fan
                match device.fan_speed(0) {
                    Ok(speed) => Some(speed as f64),
                    Err(e) => {
                        debug!("Failed to get fan speed for GPU {}: {}", index, e);
                        None
                    }
                }
            }
            _ => {
                debug!("No fans detected for GPU {} (laptop GPU?)", index);
                None
            }
        };

        // Get graphics clock speed
        let clock_graphics_mhz = match device.clock_info(Clock::Graphics) {
            Ok(clock) => Some(clock),
            Err(e) => {
                debug!("Failed to get graphics clock for GPU {}: {}", index, e);
                None
            }
        };

        // Get memory clock speed
        let clock_memory_mhz = match device.clock_info(Clock::Memory) {
            Ok(clock) => Some(clock),
            Err(e) => {
                debug!("Failed to get memory clock for GPU {}: {}", index, e);
                None
            }
        };

        Ok(GpuInfo {
            index,
            name,
            vendor: "NVIDIA".to_string(),
            vram_total_mb,
            vram_used_mb,
            usage_percent,
            memory_usage_percent,
            temperature_celsius,
            hotspot_temp_celsius: None, // NVML doesn't expose hotspot temp directly
            power_watts,
            fan_speed_percent,
            clock_graphics_mhz,
            clock_memory_mhz,
        })
    }
}

impl Default for GpuCollector {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            nvml: None,
            gpu_count: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_collector_creation() {
        // This test will pass even without NVIDIA GPU
        let collector = GpuCollector::new();
        assert!(collector.is_ok());
    }

    #[test]
    fn test_gpu_collect_returns_valid_data() {
        let collector = GpuCollector::new().unwrap();
        let gpus = collector.collect();
        
        // If GPUs are available, verify the data structure
        for gpu in &gpus {
            assert!(!gpu.name.is_empty());
            assert_eq!(gpu.vendor, "NVIDIA");
        }
    }
}
