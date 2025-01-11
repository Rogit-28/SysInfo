use crate::models::StorageInfo;
use sysinfo::{Disks, DiskKind};

pub struct StorageCollector;

impl StorageCollector {
    pub fn new() -> Self { Self }
    pub fn collect(&self) -> Vec<StorageInfo> {
        Disks::new_with_refreshed_list().iter().map(|d| {
            let total = d.total_space() as f64 / 1_073_741_824.0;
            let avail = d.available_space() as f64 / 1_073_741_824.0;
            StorageInfo {
                name: d.name().to_string_lossy().into(),
                mount_point: d.mount_point().to_string_lossy().into(),
                drive_type: match d.kind() { DiskKind::SSD => "SSD", DiskKind::HDD => "HDD", _ => "Unknown" }.into(),
                total_gb: total, used_gb: total - avail, available_gb: avail,
                usage_percent: if total > 0.0 { ((total - avail) / total) * 100.0 } else { 0.0 },
                ..Default::default()
            }
        }).collect()
    }
}
