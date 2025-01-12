//! Process metrics collector.
//!
//! Collects information about running processes using sysinfo crate.

use crate::models::ProcessInfo;
use parking_lot::Mutex;
use sysinfo::{ProcessRefreshKind, ProcessStatus, System, UpdateKind};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::debug;

const BYTES_TO_MB: f64 = 1_048_576.0;

/// Cache lifetime for username resolution
const USERNAME_CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// Process metrics collector.
pub struct ProcessCollector {
    system: Mutex<System>,
    /// Cache of SID -> username mappings
    username_cache: Mutex<HashMap<String, String>>,
    /// Last cache refresh time
    last_cache_refresh: Mutex<Instant>,
}

impl ProcessCollector {
    /// Create a new process collector.
    pub fn new() -> Self {
        let mut system = System::new();
        // Initial refresh with all process data
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::everything(),
        );
        
        Self {
            system: Mutex::new(system),
            username_cache: Mutex::new(HashMap::new()),
            last_cache_refresh: Mutex::new(Instant::now()),
        }
    }

    /// Collect top N processes sorted by CPU or memory usage.
    pub fn collect(&self, top_n: usize, sort_by: &str) -> Vec<ProcessInfo> {
        let mut system = self.system.lock();
        let mut username_cache = self.username_cache.lock();
        let mut last_cache_refresh = self.last_cache_refresh.lock();
        
        // Refresh process information with full details including user and cmd
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new()
                .with_cpu()
                .with_memory()
                .with_user(UpdateKind::Always)
                .with_cmd(UpdateKind::Always),
        );

        // Clear cache periodically
        if last_cache_refresh.elapsed() >= USERNAME_CACHE_TTL {
            username_cache.clear();
            *last_cache_refresh = Instant::now();
        }

        let mut processes: Vec<ProcessInfo> = system
            .processes()
            .iter()
            .map(|(pid, process)| {
                let status = match process.status() {
                    ProcessStatus::Run => "running",
                    ProcessStatus::Sleep => "sleeping",
                    ProcessStatus::Stop => "stopped",
                    ProcessStatus::Zombie => "zombie",
                    ProcessStatus::Idle => "idle",
                    _ => "unknown",
                };

                // Resolve SID to username
                let user = process.user_id().map(|uid| {
                    let sid = uid.to_string();
                    // Check cache first
                    if let Some(cached) = username_cache.get(&sid) {
                        return cached.clone();
                    }
                    // Resolve well-known SIDs
                    let resolved = resolve_sid_to_name(&sid);
                    username_cache.insert(sid, resolved.clone());
                    resolved
                });

                ProcessInfo {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().to_string(),
                    cpu_percent: process.cpu_usage(),
                    memory_mb: process.memory() as f64 / BYTES_TO_MB,
                    virtual_memory_mb: process.virtual_memory() as f64 / BYTES_TO_MB,
                    status: status.to_string(),
                    start_time: process.start_time(),
                    user,
                    cmd: {
                        let cmd = process.cmd();
                        if cmd.is_empty() {
                            None
                        } else {
                            Some(cmd.iter().map(|s| s.to_string_lossy().to_string()).collect::<Vec<_>>().join(" "))
                        }
                    },
                }
            })
            .collect();

        // Sort by specified field
        match sort_by.to_lowercase().as_str() {
            "memory" | "mem" => {
                processes.sort_by(|a, b| b.memory_mb.partial_cmp(&a.memory_mb).unwrap_or(std::cmp::Ordering::Equal));
            }
            _ => {
                // Default to CPU
                processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
            }
        }

        // Take top N
        processes.truncate(top_n);

        debug!("Collected {} processes (top {} by {})", processes.len(), top_n, sort_by);
        processes
    }
}

/// Resolve a Windows SID to a username.
/// Falls back to the SID string if resolution fails.
fn resolve_sid_to_name(sid: &str) -> String {
    // Well-known SIDs
    match sid {
        "S-1-5-18" => return "SYSTEM".to_string(),
        "S-1-5-19" => return "LOCAL SERVICE".to_string(),
        "S-1-5-20" => return "NETWORK SERVICE".to_string(),
        _ => {}
    }

    // For user SIDs, try to resolve via Windows API
    // User SIDs typically look like: S-1-5-21-XXXXXXX-XXXXXXX-XXXXXXX-YYYY
    if sid.starts_with("S-1-5-21-") {
        // Try to get username from the system
        if let Some(username) = resolve_user_sid(sid) {
            return username;
        }
    }

    // Return SID if we can't resolve it
    sid.to_string()
}

/// Try to resolve a user SID to username using whoami or registry
fn resolve_user_sid(sid: &str) -> Option<String> {
    // Use PowerShell to resolve - this is cached so only called once per SID
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive", 
            "-Command",
            &format!(
                "try {{ (New-Object System.Security.Principal.SecurityIdentifier('{}')).Translate([System.Security.Principal.NTAccount]).Value }} catch {{ }}",
                sid
            )
        ])
        .output()
        .ok()?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            // Return just the username part (after the backslash)
            return Some(name.split('\\').last().unwrap_or(&name).to_string());
        }
    }

    None
}
