use crate::models::CpuInfo;
use sysinfo::{CpuRefreshKind, RefreshKind, System};
use std::sync::Mutex;

pub struct CpuCollector { system: Mutex<System> }

impl CpuCollector {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        Self { system: Mutex::new(sys) }
    }
    pub fn collect(&self) -> CpuInfo {
        let mut sys = self.system.lock().unwrap();
        sys.refresh_cpu_all();
        CpuInfo {
            name: sys.cpus().first().map(|c| c.brand().into()).unwrap_or_default(),
            vendor: sys.cpus().first().map(|c| c.vendor_id().into()).unwrap_or_default(),
            cores: sys.physical_core_count().unwrap_or(0) as u32,
            threads: sys.cpus().len() as u32,
            architecture: std::env::consts::ARCH.into(),
            usage_percent: Some(sys.global_cpu_usage() as f64),
            ..Default::default()
        }
    }
}
