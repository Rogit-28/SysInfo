//! Hardware data collectors module.
//!
//! Contains collectors for each hardware category (CPU, GPU, memory, storage, network).
//! Each collector is responsible for querying Windows APIs and returning structured data.

pub mod cpu;
pub mod events;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod processes;
pub mod services;
pub mod sessions;
pub mod storage;
pub mod system;
pub mod updates;

use crate::config::Settings;
use tracing::{info, warn, error};

/// Registry of all hardware collectors.
///
/// Manages the lifecycle and access to individual collectors based on configuration.
pub struct CollectorRegistry {
    pub cpu: Option<cpu::CpuCollector>,
    pub gpu: gpu::GpuCollector,
    pub memory: Option<memory::MemoryCollector>,
    pub storage: Option<storage::StorageCollector>,
    pub network: Option<network::NetworkCollector>,
    pub processes: processes::ProcessCollector,
    pub services: services::ServiceCollector,
}

impl CollectorRegistry {
    /// Create a new collector registry based on configuration.
    /// Gracefully handles collector initialization failures without crashing.
    pub fn new(_settings: &Settings) -> Self {
        let cpu = match cpu::CpuCollector::new() {
            Ok(c) => {
                info!("CPU collector initialized");
                Some(c)
            }
            Err(e) => {
                error!("Failed to initialize CPU collector: {} - CPU metrics will be unavailable", e);
                None
            }
        };

        let gpu = match gpu::GpuCollector::new() {
            Ok(c) => {
                info!("GPU collector initialized");
                c
            }
            Err(e) => {
                warn!("Failed to initialize GPU collector: {} - GPU metrics will be unavailable", e);
                gpu::GpuCollector::default()
            }
        };

        let memory = match memory::MemoryCollector::new() {
            Ok(c) => {
                info!("Memory collector initialized");
                Some(c)
            }
            Err(e) => {
                error!("Failed to initialize Memory collector: {} - Memory metrics will be unavailable", e);
                None
            }
        };

        let storage = match storage::StorageCollector::new() {
            Ok(c) => {
                info!("Storage collector initialized");
                Some(c)
            }
            Err(e) => {
                error!("Failed to initialize Storage collector: {} - Storage metrics will be unavailable", e);
                None
            }
        };

        let network = match network::NetworkCollector::new() {
            Ok(c) => {
                info!("Network collector initialized");
                Some(c)
            }
            Err(e) => {
                error!("Failed to initialize Network collector: {} - Network metrics will be unavailable", e);
                None
            }
        };

        let processes = processes::ProcessCollector::new();
        info!("Process collector initialized");

        let services = services::ServiceCollector::new();
        info!("Service collector initialized");

        Self {
            cpu,
            gpu,
            memory,
            storage,
            network,
            processes,
            services,
        }
    }
    
    /// Check if all critical collectors are available.
    pub fn is_healthy(&self) -> bool {
        self.cpu.is_some() && self.memory.is_some()
    }
}
