//! User session collector.
//!
//! Collects active user sessions using quser/query user command.
//! Includes caching to avoid excessive PowerShell calls.

use crate::models::SessionInfo;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

/// Cache TTL for sessions (5 seconds - sessions don't change frequently)
const CACHE_TTL: Duration = Duration::from_secs(5);

/// Cached session data with timestamp
struct SessionCache {
    sessions: Vec<SessionInfo>,
    cached_at: Instant,
}

/// Global session cache
static SESSION_CACHE: Mutex<Option<SessionCache>> = Mutex::new(None);

/// Collect active user sessions.
/// Uses caching to avoid excessive PowerShell calls.
pub fn collect() -> Vec<SessionInfo> {
    debug!("Collecting user sessions");

    // Check cache first
    if let Ok(cache_guard) = SESSION_CACHE.lock() {
        if let Some(cache) = cache_guard.as_ref() {
            if cache.cached_at.elapsed() < CACHE_TTL {
                debug!("Returning cached session data");
                return cache.sessions.clone();
            }
        }
    }

    // Cache miss or expired - collect fresh data
    let sessions = collect_fresh();
    
    // Update cache
    if let Ok(mut cache_guard) = SESSION_CACHE.lock() {
        *cache_guard = Some(SessionCache {
            sessions: sessions.clone(),
            cached_at: Instant::now(),
        });
    }

    sessions
}

/// Collect fresh session data (no caching).
fn collect_fresh() -> Vec<SessionInfo> {
    // Try PowerShell approach first (more detailed)
    match collect_via_powershell() {
        Ok(sessions) if !sessions.is_empty() => return sessions,
        Ok(_) => debug!("PowerShell returned no sessions, trying quser"),
        Err(e) => warn!("PowerShell session collection failed: {}, trying quser", e),
    }

    // Fallback to quser command
    match collect_via_quser() {
        Ok(sessions) => sessions,
        Err(e) => {
            error!("Failed to collect sessions: {}", e);
            Vec::new()
        }
    }
}

/// Collect sessions via PowerShell (more detailed info).
fn collect_via_powershell() -> Result<Vec<SessionInfo>, String> {
    let ps_script = r#"
$ErrorActionPreference = 'SilentlyContinue'

# Get sessions using CIM
$sessions = @()

try {
    $logonSessions = Get-CimInstance -ClassName Win32_LogonSession | Where-Object {
        $_.LogonType -in @(2, 10, 11, 12)  # Interactive, RemoteInteractive, CachedInteractive, CachedRemoteInteractive
    }
    
    foreach ($session in $logonSessions) {
        $user = Get-CimAssociatedInstance -InputObject $session -ResultClassName Win32_UserAccount -ErrorAction SilentlyContinue
        if (-not $user) {
            # Try to get user from LoggedOnUser
            $loggedOn = Get-CimInstance -ClassName Win32_LoggedOnUser -ErrorAction SilentlyContinue | 
                Where-Object { $_.Dependent.LogonId -eq $session.LogonId }
            if ($loggedOn) {
                $userPath = $loggedOn.Antecedent
                $username = ($userPath -split 'Name="')[1] -replace '".*',''
            } else {
                continue
            }
        } else {
            $username = $user.Name
        }
        
        $sessionType = switch ($session.LogonType) {
            2 { "Console" }
            10 { "RDP" }
            11 { "CachedConsole" }
            12 { "CachedRDP" }
            default { "Unknown" }
        }
        
        $logonTime = $null
        if ($session.StartTime) {
            $logonTime = $session.StartTime.ToString("o")
        }
        
        $sessions += [PSCustomObject]@{
            Username = $username
            SessionId = $session.LogonId
            SessionType = $sessionType
            State = "Active"
            LogonTime = $logonTime
            IdleSeconds = $null
            ClientName = $null
            ClientIP = $null
        }
    }
} catch {
    # Silently fail, will fallback to quser
}

# Deduplicate by username and session type
$sessions = $sessions | Sort-Object Username, SessionType -Unique

$sessions | ConvertTo-Json -Compress
"#;

    let output = run_powershell(ps_script)?;
    parse_sessions_json(&output)
}

/// Collect sessions via quser command (fallback).
fn collect_via_quser() -> Result<Vec<SessionInfo>, String> {
    let output = Command::new("cmd")
        .args(["/c", "quser", "2>nul"])
        .output()
        .map_err(|e| format!("Failed to run quser: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_quser_output(&stdout)
}

/// Run PowerShell command.
fn run_powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse JSON output from PowerShell.
fn parse_sessions_json(json_str: &str) -> Result<Vec<SessionInfo>, String> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let json_value: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {}", e))?;

    let sessions_array = match json_value {
        serde_json::Value::Array(arr) => arr,
        other => vec![other],
    };

    let sessions = sessions_array
        .iter()
        .filter_map(|v| {
            Some(SessionInfo {
                username: v.get("Username")?.as_str()?.to_string(),
                session_id: v.get("SessionId")?.as_u64().unwrap_or(0) as u32,
                session_type: v
                    .get("SessionType")
                    .and_then(|s| s.as_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                state: v
                    .get("State")
                    .and_then(|s| s.as_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                logon_time: v
                    .get("LogonTime")
                    .and_then(|t| t.as_str())
                    .map(String::from),
                idle_seconds: v.get("IdleSeconds").and_then(|i| i.as_u64()),
                client_name: v
                    .get("ClientName")
                    .and_then(|c| c.as_str())
                    .map(String::from),
                client_ip: v
                    .get("ClientIP")
                    .and_then(|c| c.as_str())
                    .map(String::from),
            })
        })
        .collect();

    Ok(sessions)
}

/// Parse quser command output.
fn parse_quser_output(output: &str) -> Result<Vec<SessionInfo>, String> {
    let mut sessions = Vec::new();
    let lines: Vec<&str> = output.lines().collect();

    // Skip header line
    for line in lines.iter().skip(1) {
        if line.trim().is_empty() {
            continue;
        }

        // quser output format (fixed width):
        // USERNAME              SESSIONNAME        ID  STATE   IDLE TIME  LOGON TIME
        // >username             console             1  Active      none   12/29/2024 10:30 AM

        let line = line.trim_start_matches('>').trim();
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 4 {
            let username = parts[0].to_string();
            let session_name = parts[1].to_string();
            let session_id = parts[2].parse::<u32>().unwrap_or(0);
            let state = parts[3].to_string();

            let session_type = if session_name.to_lowercase().contains("console") {
                "Console"
            } else if session_name.to_lowercase().contains("rdp") {
                "RDP"
            } else {
                "Interactive"
            }
            .to_string();

            sessions.push(SessionInfo {
                username,
                session_id,
                session_type,
                state,
                logon_time: None, // quser format is complex to parse
                idle_seconds: None,
                client_name: None,
                client_ip: None,
            });
        }
    }

    Ok(sessions)
}
