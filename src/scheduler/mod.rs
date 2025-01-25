//! Background scheduler for periodic data collection.
//!
//! This module handles the periodic collection of hardware metrics
//! and their persistence to InfluxDB. It runs in a background task
//! independent of the API server.

use crate::collectors::CollectorRegistry;
use crate::config::Settings;
use crate::models::SystemSnapshot;
use crate::persistence::influxdb::InfluxDbWriter;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info};

/// Background scheduler for metric collection.
pub struct Scheduler {
    collectors: Arc<CollectorRegistry>,
    influxdb: Arc<InfluxDbWriter>,
    settings: Arc<Settings>,
}

impl Scheduler {
    /// Create a new scheduler instance.
    pub fn new(
        collectors: Arc<CollectorRegistry>,
        influxdb: Arc<InfluxDbWriter>,
        settings: Arc<Settings>,
    ) -> Self {
        Self {
            collectors,
            influxdb,
            settings,
        }
    }

    /// Start the collection loop.
    ///
    /// This function runs indefinitely until the application shuts down.
    pub async fn run(&self) {
        // Only run the scheduler if InfluxDB is enabled
        // Otherwise, there's no point collecting data in the background
        if !self.settings.influxdb.enabled {
            info!("Scheduler disabled (InfluxDB not enabled)");
            return;
        }

        let interval_ms = self.settings.collection.interval_ms;
        let mut interval = time::interval(Duration::from_millis(interval_ms));

        info!(
            "Starting collection scheduler (interval: {}ms)",
            interval_ms
        );

        loop {
            interval.tick().await;

            debug!("Scheduler: starting collection cycle");

            // 1. Collect metrics in a blocking thread to avoid blocking the async runtime
            let collectors = Arc::clone(&self.collectors);
            let settings = Arc::clone(&self.settings);
            
            let snapshot = match tokio::task::spawn_blocking(move || {
                Self::collect_metrics_sync(&collectors, &settings)
            }).await {
                Ok(s) => s,
                Err(e) => {
                    error!("Collection task panicked: {}", e);
                    continue;
                }
            };

            debug!("Scheduler: collection complete, writing to InfluxDB");

            // 2. Persist to InfluxDB
            if let Err(e) = self.influxdb.write_snapshot(&snapshot).await {
                error!("Failed to write snapshot to InfluxDB: {}", e);
            }
        }
    }

    /// Collect a full system snapshot (synchronous, runs in blocking thread).
    fn collect_metrics_sync(collectors: &CollectorRegistry, settings: &Settings) -> SystemSnapshot {
        // Create a new snapshot
        let mut snapshot = SystemSnapshot::default();
        
        // Collect enabled metrics - handle Optional collectors gracefully
        if settings.collection.collect_cpu {
            if let Some(cpu) = &collectors.cpu {
                snapshot.cpu = cpu.collect();
            }
        }

        if settings.collection.collect_gpu {
            snapshot.gpus = collectors.gpu.collect();
        }

        if settings.collection.collect_memory {
            if let Some(memory) = &collectors.memory {
                snapshot.memory = memory.collect();
            }
        }

        if settings.collection.collect_storage {
            if let Some(storage) = &collectors.storage {
                snapshot.storage = storage.collect();
            }
        }

        if settings.collection.collect_network {
            if let Some(network) = &collectors.network {
                snapshot.network = network.collect();
            }
        }
        
        // System info is always collected
        snapshot.system = crate::collectors::system::collect_system_info();

        snapshot
    }
}
