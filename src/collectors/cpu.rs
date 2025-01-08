//! CPU metrics collector.
//!
//! Collects CPU identification, usage, and frequency data using the sysinfo crate.
//! Temperature is fetched via WMI when available.

use crate::models::CpuInfo;
use parking_lot::Mutex;
use sysinfo::{CpuRefreshKind, RefreshKind, System};
use std::process::Command;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info};

/// CPU collector errors.
#[derive(Debug, Error)]
pub enum CpuCollectorError {
    #[error("Failed to initialize CPU collector: {0}")]
    InitError(String),

    #[error("Failed to collect CPU metrics: {0}")]
    CollectionError(String),
}

/// Cached value with TTL
struct CachedValue<T> {
    value: Option<T>,
    last_updated: Option<Instant>,
    ttl: Duration,
}

impl<T: Clone> CachedValue<T> {
    fn new(ttl: Duration) -> Self {
        Self {
            value: None,
            last_updated: None,
            ttl,
        }
    }

    fn get(&self) -> Option<T> {
        if let (Some(value), Some(last_updated)) = (&self.value, &self.last_updated) {
            if last_updated.elapsed() < self.ttl {
                return Some(value.clone());
            }
        }
        None
    }

    fn set(&mut self, value: T) {
        self.value = Some(value);
        self.last_updated = Some(Instant::now());
    }
}

/// CPU metrics collector.
///
/// Collects comprehensive CPU data including identification, usage statistics,
/// and per-core frequencies using the sysinfo crate.
pub struct CpuCollector {
    /// sysinfo System instance for CPU metrics
    system: Mutex<System>,
    /// Cached CPU name (permanent, doesn't change)
    cached_name: Mutex<Option<String>>,
    /// Cached vendor info (permanent, doesn't change)
    cached_vendor: Mutex<Option<String>>,
    /// Last collected CpuInfo for fast returns
    cached_result: Mutex<CachedValue<CpuInfo>>,
    /// Cached CPU temperature (refreshed periodically)
    cached_temperature: Mutex<CachedValue<Option<f64>>>,
}

impl CpuCollector {
    /// Create a new CPU collector.
    pub fn new() -> Result<Self, CpuCollectorError> {
        let system = System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
        );

        info!("CPU collector initialized");

        Ok(Self {
            system: Mutex::new(system),
            cached_name: Mutex::new(None),
            cached_vendor: Mutex::new(None),
            cached_result: Mutex::new(CachedValue::new(Duration::from_millis(500))),
            cached_temperature: Mutex::new(CachedValue::new(Duration::from_secs(2))),
        })
    }

    /// Collect current CPU metrics.
    ///
    /// Returns a `CpuInfo` struct with all available CPU data.
    /// Fields that cannot be collected will be `None`.
    pub fn collect(&self) -> CpuInfo {
        // Check if we have a fresh cached result
        {
            let cache = self.cached_result.lock();
            if let Some(result) = cache.get() {
                debug!("Returning cached CPU result");
                return result;
            }
        }

        debug!("Collecting fresh CPU metrics");
        
        let mut sys = self.system.lock();

        // Refresh CPU data
        sys.refresh_cpu_all();

        // Get CPU info
        let cpus = sys.cpus();
        
        // Get or cache CPU name and vendor
        let (name, vendor) = {
            let mut cached_name = self.cached_name.lock();
            let mut cached_vendor = self.cached_vendor.lock();

            if cached_name.is_none() || cached_vendor.is_none() {
                if let Some(first_cpu) = cpus.first() {
                    let cpu_brand = first_cpu.brand().to_string();
                    let cpu_vendor = first_cpu.vendor_id().to_string();
                    
                    *cached_name = Some(if cpu_brand.is_empty() { 
                        "Unknown CPU".to_string() 
                    } else { 
                        cpu_brand 
                    });
                    
                    *cached_vendor = Some(if cpu_vendor.is_empty() { 
                        "Unknown".to_string() 
                    } else { 
                        cpu_vendor 
                    });
                }
            }

            (
                cached_name.clone().unwrap_or_else(|| "Unknown CPU".to_string()),
                cached_vendor.clone().unwrap_or_else(|| "Unknown".to_string()),
            )
        };

        // Get physical core and thread count
        let threads = cpus.len() as u32;
        let cores = sys.physical_core_count().unwrap_or(threads as usize / 2) as u32;

        // Calculate total CPU usage (average across all cores)
        let per_core_usage: Vec<f64> = cpus.iter().map(|cpu| cpu.cpu_usage() as f64).collect();
        let total_usage = if !per_core_usage.is_empty() {
            per_core_usage.iter().sum::<f64>() / per_core_usage.len() as f64
        } else {
            0.0
        };

        // Get per-core frequencies
        let per_core_frequency: Vec<u32> = cpus.iter().map(|cpu| cpu.frequency() as u32).collect();

        // Use max frequency as base clock (best approximation without WMI)
        let base_clock = per_core_frequency.iter().max().copied();

        // Get CPU temperature (cached, refreshed every 2 seconds)
        let temperature_celsius = {
            let mut temp_cache = self.cached_temperature.lock();
            if let Some(temp) = temp_cache.get() {
                temp
            } else {
                let temp = fetch_cpu_temperature();
                temp_cache.set(temp);
                temp
            }
        };

        let result = CpuInfo {
            name,
            vendor,
            cores,
            threads,
            architecture: std::env::consts::ARCH.to_string(),
            base_clock_mhz: base_clock,
            usage_percent: Some(total_usage),
            per_core_usage: Some(per_core_usage),
            per_core_frequency_mhz: if per_core_frequency.is_empty() { None } else { Some(per_core_frequency) },
            temperature_celsius,
            power_watts: None, // Requires vendor-specific APIs (Intel RAPL, etc.)
        };

        // Cache the result
        self.cached_result.lock().set(result.clone());

        result
    }
}

/// Fetch CPU temperature via WMI.
/// Tries MSAcpi_ThermalZoneTemperature first (requires admin), then falls back to
/// Win32_TemperatureProbe. Returns None if temperature cannot be retrieved.
fn fetch_cpu_temperature() -> Option<f64> {
    // Try MSAcpi_ThermalZoneTemperature (most reliable, requires admin)
    let ps_script = r#"
$temp = $null
try {
    # Try thermal zone (requires admin, most reliable)
    $tz = Get-CimInstance -Namespace "root/WMI" -ClassName MSAcpi_ThermalZoneTemperature -ErrorAction Stop | Select-Object -First 1
    if ($tz.CurrentTemperature) {
        # Temperature is in tenths of Kelvin, convert to Celsius
        $temp = ($tz.CurrentTemperature - 2732) / 10.0
    }
} catch {
    # Thermal zone not available or no admin rights
}
if ($temp) { Write-Output $temp } else { Write-Output "null" }
"#;

    let output = match Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            debug!("Failed to fetch CPU temperature: {}", e);
            return None;
        }
    };

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    
    if trimmed.is_empty() || trimmed == "null" {
        return None;
    }

    match trimmed.parse::<f64>() {
        Ok(temp) if temp > 0.0 && temp < 150.0 => {
            debug!("CPU temperature: {:.1}°C", temp);
            Some(temp)
        }
        _ => None,
    }
}
