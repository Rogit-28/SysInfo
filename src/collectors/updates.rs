//! Windows Update status collector.
//!
//! Collects Windows Update status using a single batched PowerShell call.
//! This combines reboot check, pending updates, and recent installs into one invocation.

use crate::models::{UpdateInfo, UpdateSummary};
use std::process::Command;
use tracing::{debug, error};

/// Collect Windows Update status using a single batched PowerShell call.
pub fn collect() -> UpdateSummary {
    debug!("Collecting Windows Update status");

    // Single batched PowerShell call for all update info
    match collect_all_update_info() {
        Ok(summary) => summary,
        Err(e) => {
            error!("Failed to collect update info: {}", e);
            UpdateSummary {
                last_check: None,
                last_install: None,
                reboot_pending: false,
                pending_count: 0,
                pending_updates: Vec::new(),
                recent_installed_count: 0,
            }
        }
    }
}

/// Collect all update information in a single PowerShell call.
/// This batches reboot check, pending updates, and recent installs together.
fn collect_all_update_info() -> Result<UpdateSummary, String> {
    let ps_script = r#"
$ErrorActionPreference = 'SilentlyContinue'

# === 1. Check Reboot Pending ===
$rebootRequired = $false

$wuKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired'
if (Test-Path $wuKey) { $rebootRequired = $true }

$cbsKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending'
if (Test-Path $cbsKey) { $rebootRequired = $true }

$pfroKey = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager'
$pfro = Get-ItemProperty -Path $pfroKey -Name 'PendingFileRenameOperations' -ErrorAction SilentlyContinue
if ($pfro.PendingFileRenameOperations) { $rebootRequired = $true }

# === 2. Get Pending Updates ===
$lastSearch = $null
$pendingUpdates = @()

try {
    $session = New-Object -ComObject Microsoft.Update.Session
    $searcher = $session.CreateUpdateSearcher()
    
    try {
        $autoUpdate = New-Object -ComObject Microsoft.Update.AutoUpdate
        $lastSearch = $autoUpdate.Results.LastSearchSuccessDate
        if ($lastSearch) {
            $lastSearch = $lastSearch.ToString("o")
        }
    } catch {}
    
    $searchResult = $searcher.Search("IsInstalled=0 AND IsHidden=0")
    
    foreach ($update in $searchResult.Updates) {
        $kbIds = @()
        foreach ($kb in $update.KBArticleIDs) {
            $kbIds += "KB$kb"
        }
        
        $sizeMB = $null
        if ($update.MaxDownloadSize -gt 0) {
            $sizeMB = [math]::Round($update.MaxDownloadSize / 1MB, 2)
        }
        
        $pendingUpdates += [PSCustomObject]@{
            Title = $update.Title
            KbId = if ($kbIds.Count -gt 0) { $kbIds[0] } else { $null }
            Description = $update.Description
            Severity = $update.MsrcSeverity
            IsInstalled = $false
            IsMandatory = $update.IsMandatory
            RebootRequired = $update.RebootRequired
            InstallDate = $null
            SizeMB = $sizeMB
        }
    }

    # === 3. Get Recent Installs (reuse session/searcher) ===
    $historyCount = $searcher.GetTotalHistoryCount()
    $history = $searcher.QueryHistory(0, [math]::Min($historyCount, 100))
    
    $thirtyDaysAgo = (Get-Date).AddDays(-30)
    $recentCount = 0
    $lastInstall = $null
    
    foreach ($entry in $history) {
        if ($entry.Operation -eq 1 -and $entry.ResultCode -eq 2) {
            if (-not $lastInstall -or $entry.Date -gt $lastInstall) {
                $lastInstall = $entry.Date
            }
            if ($entry.Date -gt $thirtyDaysAgo) {
                $recentCount++
            }
        }
    }
    
    $lastInstallStr = $null
    if ($lastInstall) {
        $lastInstallStr = $lastInstall.ToString("o")
    }

} catch {
    $lastSearch = $null
    $pendingUpdates = @()
    $lastInstallStr = $null
    $recentCount = 0
}

# === Output Combined Result ===
[PSCustomObject]@{
    RebootPending = $rebootRequired
    LastCheck = $lastSearch
    LastInstall = $lastInstallStr
    RecentInstalledCount = $recentCount
    PendingUpdates = $pendingUpdates
} | ConvertTo-Json -Depth 3 -Compress
"#;

    let output = run_powershell(ps_script)?;
    parse_update_summary(&output)
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

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("PowerShell error: {}", stderr))
    }
}

/// Parse the combined update summary JSON from the batched PowerShell call.
fn parse_update_summary(json_str: &str) -> Result<UpdateSummary, String> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() {
        return Ok(UpdateSummary {
            last_check: None,
            last_install: None,
            reboot_pending: false,
            pending_count: 0,
            pending_updates: Vec::new(),
            recent_installed_count: 0,
        });
    }

    let json_value: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {}", e))?;

    let reboot_pending = json_value
        .get("RebootPending")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let last_check = json_value
        .get("LastCheck")
        .and_then(|v| v.as_str())
        .map(String::from);

    let last_install = json_value
        .get("LastInstall")
        .and_then(|v| v.as_str())
        .map(String::from);

    let recent_installed_count = json_value
        .get("RecentInstalledCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let pending_updates: Vec<UpdateInfo> = json_value
        .get("PendingUpdates")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(UpdateInfo {
                        title: v.get("Title")?.as_str()?.to_string(),
                        kb_id: v.get("KbId").and_then(|k| k.as_str()).map(String::from),
                        description: v
                            .get("Description")
                            .and_then(|d| d.as_str())
                            .map(String::from),
                        severity: v
                            .get("Severity")
                            .and_then(|s| s.as_str())
                            .map(String::from),
                        is_installed: v
                            .get("IsInstalled")
                            .and_then(|i| i.as_bool())
                            .unwrap_or(false),
                        is_mandatory: v
                            .get("IsMandatory")
                            .and_then(|i| i.as_bool())
                            .unwrap_or(false),
                        reboot_required: v
                            .get("RebootRequired")
                            .and_then(|r| r.as_bool())
                            .unwrap_or(false),
                        install_date: None,
                        size_mb: v.get("SizeMB").and_then(|s| s.as_f64()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(UpdateSummary {
        last_check,
        last_install,
        reboot_pending,
        pending_count: pending_updates.len() as u32,
        pending_updates,
        recent_installed_count,
    })
}
