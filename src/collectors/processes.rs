use crate::models::ProcessInfo;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, ProcessesToUpdate};

pub struct ProcessCollector { system: System }

impl ProcessCollector {
    pub fn new() -> Self {
        Self { system: System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::everything())) }
    }
    pub fn collect(&mut self, top_n: usize, sort_by: &str) -> Vec<ProcessInfo> {
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        let mut procs: Vec<_> = self.system.processes().values().map(|p| ProcessInfo {
            pid: p.pid().as_u32(), name: p.name().to_string_lossy().into(),
            cpu_percent: p.cpu_usage(), memory_mb: p.memory() as f64 / 1_048_576.0,
            virtual_memory_mb: p.virtual_memory() as f64 / 1_048_576.0,
            status: format!("{:?}", p.status()), start_time: p.start_time(), ..Default::default()
        }).collect();
        match sort_by { "memory" => procs.sort_by(|a, b| b.memory_mb.partial_cmp(&a.memory_mb).unwrap()),
                        _ => procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap()) }
        procs.truncate(top_n); procs
    }
}
