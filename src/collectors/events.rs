//! Windows Event Log collector.
//!
//! Collects recent events from Windows Event Logs using PowerShell.

use crate::config::Settings;
use crate::models::EventInfo;
use std::process::Command;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Collect recent Windows Event Log entries.
pub fn collect(config: &Settings) -> Vec<EventInfo> {
    debug!("Collecting Windows Event Log entries");

    let max_age = config.events.max_age_minutes;
    let max_count = config.events.max_count;
    let severity_levels = &config.events.severity;

    // Build severity filter for PowerShell
    // Level: 1=Critical, 2=Error, 3=Warning, 4=Information
    let mut levels: Vec<&str> = Vec::new();
    for s in severity_levels {
        match s.to_lowercase().as_str() {
            "critical" => levels.push("1"),
            "error" => levels.push("2"),
            "warning" => levels.push("3"),
            "information" | "info" => levels.push("4"),
            _ => {}
        }
    }

    if levels.is_empty() {
        levels = vec!["1", "2", "3"]; // Default: Critical, Error, Warning
    }

    let level_filter = levels.join(",");

    // PowerShell script to get events
    let ps_script = format!(
        r#"
$ErrorActionPreference = 'SilentlyContinue'
$startTime = (Get-Date).AddMinutes(-{max_age})
$logs = @('System', 'Application')
$events = @()

foreach ($log in $logs) {{
    $logEvents = Get-WinEvent -FilterHashtable @{{
        LogName = $log
        Level = {level_filter}
        StartTime = $startTime
    }} -MaxEvents {max_count} 2>$null

    if ($logEvents) {{
        $events += $logEvents
    }}
}}

$events | Sort-Object TimeCreated -Descending | Select-Object -First {max_count} | ForEach-Object {{
    $level = switch ($_.Level) {{
        1 {{ "Critical" }}
        2 {{ "Error" }}
        3 {{ "Warning" }}
        4 {{ "Information" }}
        default {{ "Unknown" }}
    }}
    [PSCustomObject]@{{
        LogName = $_.LogName
        Source = $_.ProviderName
        EventId = $_.Id
        Level = $level
        Message = ($_.Message -replace "`r`n", " " -replace "`n", " " | Select-Object -First 500)
        TimeCreated = $_.TimeCreated.ToString("o")
        Computer = $_.MachineName
    }}
}} | ConvertTo-Json -Compress
"#,
        max_age = max_age,
        max_count = max_count,
        level_filter = level_filter
    );

    match run_powershell_with_timeout(&ps_script, Duration::from_secs(30)) {
        Ok(output) => parse_events_json(&output),
        Err(e) => {
            error!("Failed to collect Windows events: {}", e);
            Vec::new()
        }
    }
}

/// Run PowerShell command with timeout.
fn run_powershell_with_timeout(script: &str, timeout: Duration) -> Result<String, String> {
    use std::io::Read;
    
    let mut child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn PowerShell: {}", e))?;

    // Wait with timeout using polling
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process completed
                let mut stdout = String::new();
                let mut stderr = String::new();
                
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_string(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    let _ = err.read_to_string(&mut stderr);
                }
                
                if status.success() {
                    return Ok(stdout);
                } else {
                    return Err(format!("PowerShell error: {}", stderr));
                }
            }
            Ok(None) => {
                // Still running, check timeout
                if start.elapsed() > timeout {
                    // Kill the process
                    let _ = child.kill();
                    let _ = child.wait(); // Reap the zombie
                    return Err(format!("PowerShell timed out after {:?}", timeout));
                }
                // Sleep a bit before checking again
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!("Failed to wait for PowerShell: {}", e));
            }
        }
    }
}

/// Parse JSON output from PowerShell into EventInfo structs.
fn parse_events_json(json_str: &str) -> Vec<EventInfo> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() {
        debug!("No events returned from PowerShell");
        return Vec::new();
    }

    // Handle both single object and array
    let json_value: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse events JSON: {}", e);
            return Vec::new();
        }
    };

    let events_array = match json_value {
        serde_json::Value::Array(arr) => arr,
        other => vec![other],
    };

    events_array
        .iter()
        .filter_map(|v| {
            Some(EventInfo {
                log_name: v.get("LogName")?.as_str()?.to_string(),
                source: v.get("Source")?.as_str()?.to_string(),
                event_id: v.get("EventId")?.as_u64()? as u32,
                level: v.get("Level")?.as_str()?.to_string(),
                message: v
                    .get("Message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string(),
                time_created: v.get("TimeCreated")?.as_str()?.to_string(),
                computer: v.get("Computer").and_then(|c| c.as_str()).map(String::from),
            })
        })
        .collect()
}
