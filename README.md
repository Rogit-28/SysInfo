# SysInfo Telemetry System

A high-performance Windows hardware telemetry collection system built in Rust. Provides comprehensive real-time monitoring capabilities via a REST API with optional InfluxDB persistence for Grafana-based dashboards.

## Features

- **Real-time Hardware Monitoring**: CPU, GPU, Memory, Storage, Network metrics
- **Windows-specific Telemetry**: Services, Event Logs, User Sessions, Windows Updates
- **REST API**: Clean JSON API with versioned endpoints
- **InfluxDB Integration**: Optional time-series persistence for historical analysis
- **Configurable**: TOML-based configuration with sensible defaults
- **Secure**: API key authentication, constant-time comparison, configurable CORS
- **Production-ready**: Graceful shutdown, error handling, structured logging

## Quick Start

### Prerequisites

- Rust 1.70+ (for building)
- Windows 10/11 or Windows Server 2016+
- (Optional) NVIDIA GPU with drivers for GPU metrics
- (Optional) InfluxDB v2.x for metrics persistence

### Installation

```bash
# Clone the repository
git clone https://github.com/Rogit-28/SysInfo.git
cd SysInfo

# Build release binary
cargo build --release

# Run with default config
./target/release/sysinfo-telemetry.exe
```

### Configuration

Create `config/sysinfo.toml` or use the included example:

```toml
[server]
host = "127.0.0.1"  # Use "0.0.0.0" for network access
port = 8080
api_key = ""  # Set for authentication

[collection]
interval_ms = 1000
enable_cpu = true
enable_gpu = true
enable_memory = true
enable_storage = true
enable_network = true

[influxdb]
enabled = false
url = "http://localhost:8086"
org = "myorg"
bucket = "sysinfo"
token = "${INFLUXDB_TOKEN}"  # Environment variable
```

## API Reference

Base URL: `http://localhost:8080/api/v1`

### Endpoints Overview

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Service health status |
| `/system` | GET | Complete system snapshot |
| `/cpu` | GET | CPU metrics |
| `/gpu` | GET | GPU metrics (NVIDIA) |
| `/memory` | GET | Memory usage |
| `/storage` | GET | Disk/volume metrics |
| `/network` | GET | Network interface stats |
| `/processes` | GET | Top processes by CPU/memory |
| `/services` | GET | Windows services status |
| `/events` | GET | Windows event log entries |
| `/sessions` | GET | Active user sessions |
| `/updates` | GET | Windows Update status |

### Authentication

When `api_key` is configured, include the header:

```
X-API-Key: your-api-key-here
```

The `/health` endpoint always bypasses authentication for monitoring purposes.

---

### GET /api/v1/health

Returns service health status.

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": {
    "status": "healthy",
    "uptime_seconds": 3600,
    "collectors": [
      {"name": "cpu", "enabled": true, "healthy": true, "message": null},
      {"name": "gpu", "enabled": true, "healthy": true, "message": null}
    ],
    "influxdb_connected": false
  }
}
```

---

### GET /api/v1/cpu

Returns CPU metrics including per-core usage and frequencies.

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": {
    "name": "13th Gen Intel(R) Core(TM) i5-13500HX",
    "vendor": "GenuineIntel",
    "cores": 14,
    "threads": 20,
    "architecture": "x86_64",
    "base_clock_mhz": 2500,
    "usage_percent": 15.5,
    "per_core_usage": [10.2, 20.5, 15.0, ...],
    "per_core_frequency_mhz": [2500, 2500, 1800, ...],
    "temperature_celsius": 45.0,
    "power_watts": null
  }
}
```

**Notes:**
- `temperature_celsius` requires running as Administrator
- `power_watts` requires Intel RAPL support

---

### GET /api/v1/gpu

Returns GPU metrics for all detected NVIDIA GPUs.

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": [
    {
      "index": 0,
      "name": "NVIDIA GeForce RTX 4050 Laptop GPU",
      "vendor": "NVIDIA",
      "vram_total_mb": 6144,
      "vram_used_mb": 512,
      "usage_percent": 5.0,
      "memory_usage_percent": 8.3,
      "temperature_celsius": 42.0,
      "hotspot_temp_celsius": null,
      "power_watts": 15.5,
      "fan_speed_percent": null,
      "clock_graphics_mhz": 2100,
      "clock_memory_mhz": 8000
    }
  ]
}
```

**Notes:**
- Requires NVIDIA GPU with NVML-compatible drivers
- `fan_speed_percent` may be null on laptops without controllable fans

---

### GET /api/v1/memory

Returns system memory usage.

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": {
    "total_gb": 16.0,
    "available_gb": 8.5,
    "used_gb": 7.5,
    "usage_percent": 46.9
  }
}
```

---

### GET /api/v1/storage

Returns metrics for all detected storage volumes.

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": [
    {
      "name": "Main",
      "mount_point": "C:\\",
      "drive_type": "SSD",
      "total_gb": 500.0,
      "used_gb": 250.0,
      "available_gb": 250.0,
      "usage_percent": 50.0,
      "temperature_celsius": 38.0,
      "read_speed_mbps": 150.5,
      "write_speed_mbps": 80.2,
      "read_bytes_per_sec": 157810688.0,
      "write_bytes_per_sec": 84082892.0
    }
  ]
}
```

**Notes:**
- `temperature_celsius` requires Administrator privileges (SMART data)
- I/O stats are refreshed every 2 seconds

---

### GET /api/v1/network

Returns metrics for all network interfaces.

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": [
    {
      "name": "Wi-Fi",
      "mac": "AA:BB:CC:DD:EE:FF",
      "ipv4": "192.168.1.100",
      "ipv6": "fe80::1",
      "status": "up",
      "speed_mbps": 867,
      "bytes_sent": 1234567890,
      "bytes_received": 9876543210,
      "send_rate_mbps": 1.5,
      "receive_rate_mbps": 10.2,
      "packets_sent": 123456,
      "packets_received": 654321,
      "rx_errors": 0,
      "tx_errors": 0
    }
  ]
}
```

---

### GET /api/v1/processes

Returns top N processes sorted by CPU or memory usage.

**Configuration:**
```toml
[processes]
top_n = 10
sort_by = "cpu"  # or "memory"
```

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": [
    {
      "pid": 1234,
      "name": "chrome.exe",
      "cpu_percent": 15.5,
      "memory_mb": 512.0,
      "virtual_memory_mb": 1024.0,
      "status": "running",
      "start_time": 1705312200,
      "user": "john",
      "cmd": "C:\\Program Files\\Google\\Chrome\\chrome.exe --flag"
    }
  ]
}
```

---

### GET /api/v1/services

Returns Windows services status.

**Configuration:**
```toml
[services]
watchlist = []  # Empty = all services, or ["wuauserv", "bits"]
include_stopped = true
```

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": [
    {
      "name": "wuauserv",
      "display_name": "Windows Update",
      "status": "Running",
      "start_type": "Auto",
      "pid": 1234
    }
  ]
}
```

---

### GET /api/v1/events

Returns recent Windows Event Log entries.

**Configuration:**
```toml
[events]
max_age_minutes = 60
max_count = 50
severity = ["error", "warning"]
```

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": [
    {
      "log_name": "System",
      "source": "Service Control Manager",
      "event_id": 7036,
      "level": "Information",
      "message": "The Windows Update service entered the running state.",
      "time_created": "2025-01-15T10:25:00+00:00",
      "computer": "DESKTOP-ABC123"
    }
  ]
}
```

---

### GET /api/v1/sessions

Returns active user sessions.

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": [
    {
      "username": "john",
      "session_id": 1,
      "session_type": "Console",
      "state": "Active",
      "logon_time": "2025-01-15T08:00:00+00:00",
      "idle_seconds": null,
      "client_name": null,
      "client_ip": null
    }
  ]
}
```

---

### GET /api/v1/updates

Returns Windows Update status.

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": {
    "last_check": "2025-01-15T06:00:00",
    "last_install": "2025-01-14T22:00:00",
    "reboot_pending": false,
    "pending_count": 2,
    "pending_updates": [
      {
        "title": "Security Update KB5034441",
        "kb_id": "KB5034441",
        "severity": "Critical",
        "is_installed": false,
        "is_mandatory": true,
        "reboot_required": true,
        "size_mb": 150.5
      }
    ],
    "recent_installed_count": 5
  }
}
```

---

### GET /api/v1/system

Returns a complete system snapshot with all metrics.

**Response:**
```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "data": {
    "timestamp": "2025-01-15T10:30:00Z",
    "cpu": { ... },
    "gpus": [ ... ],
    "memory": { ... },
    "storage": [ ... ],
    "network": [ ... ],
    "system": {
      "hostname": "DESKTOP-ABC123",
      "os": "Windows",
      "os_version": "11 (22631)",
      "kernel_version": "22631",
      "uptime_seconds": 86400,
      "boot_time": 1705225800,
      "architecture": "x86_64"
    }
  }
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      HTTP Clients                           │
│              (Grafana, curl, custom apps)                   │
└─────────────────────┬───────────────────────────────────────┘
                      │ REST API
┌─────────────────────▼───────────────────────────────────────┐
│                    API Layer (Axum)                         │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐   │
│  │ Routes  │ │  Auth   │ │  CORS   │ │    Handlers     │   │
│  │         │ │Middleware│ │         │ │ (12 endpoints)  │   │
│  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘   │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                  Collector Registry                          │
│  ┌─────┐ ┌─────┐ ┌────────┐ ┌─────────┐ ┌─────────┐        │
│  │ CPU │ │ GPU │ │ Memory │ │ Storage │ │ Network │        │
│  └─────┘ └─────┘ └────────┘ └─────────┘ └─────────┘        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐        │
│  │ Processes│ │ Services │ │  Events  │ │ Updates │        │
│  └──────────┘ └──────────┘ └──────────┘ └─────────┘        │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                   Platform Layer                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   sysinfo    │  │     NVML     │  │ WMI/PowerShell   │  │
│  │   (Rust)     │  │   (NVIDIA)   │  │   (Windows)      │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                   Persistence Layer                          │
│  ┌──────────────────────┐  ┌─────────────────────────────┐  │
│  │   Background         │  │      InfluxDB v2.x          │  │
│  │   Scheduler          │──│   (Line Protocol Writer)    │  │
│  └──────────────────────┘  └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Project Structure

```
SysInfo/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library exports
│   ├── api/
│   │   ├── mod.rs           # API module
│   │   ├── server.rs        # HTTP server setup
│   │   ├── routes.rs        # Routing & middleware
│   │   └── handlers/        # Endpoint handlers
│   ├── collectors/
│   │   ├── mod.rs           # Collector registry
│   │   ├── cpu.rs           # CPU metrics
│   │   ├── gpu.rs           # GPU metrics (NVML)
│   │   ├── memory.rs        # Memory metrics
│   │   ├── storage.rs       # Disk metrics
│   │   ├── network.rs       # Network metrics
│   │   ├── processes.rs     # Process metrics
│   │   ├── services.rs      # Windows services
│   │   ├── events.rs        # Event logs
│   │   ├── sessions.rs      # User sessions
│   │   └── updates.rs       # Windows Update
│   ├── config/
│   │   └── settings.rs      # Configuration
│   ├── models/
│   │   └── mod.rs           # Data structures
│   ├── persistence/
│   │   └── influxdb.rs      # InfluxDB client
│   ├── scheduler/
│   │   └── mod.rs           # Background tasks
│   └── platform/
│       └── windows/         # Windows-specific code
├── config/
│   └── sysinfo.toml         # Example config
├── Cargo.toml
└── README.md
```

## InfluxDB Integration

### Setup

1. Install InfluxDB v2.x
2. Create a bucket for metrics
3. Generate an API token
4. Configure in `sysinfo.toml`:

```toml
[influxdb]
enabled = true
url = "http://localhost:8086"
org = "myorg"
bucket = "sysinfo"
token = "${INFLUXDB_TOKEN}"
```

### Metrics Schema

Data is written in InfluxDB line protocol:

```
cpu,host=DESKTOP-ABC usage_percent=15.5,cores=8i,threads=16i 1705312200000
memory,host=DESKTOP-ABC total_gb=16.0,used_gb=8.0,usage_percent=50.0 1705312200000
gpu,host=DESKTOP-ABC,index=0,name=RTX\ 4050 usage_percent=5.0,temperature_celsius=42.0 1705312200000
storage,host=DESKTOP-ABC,mount_point=C:\,drive_type=SSD total_gb=500.0,used_gb=250.0 1705312200000
network,host=DESKTOP-ABC,interface=Wi-Fi,status=up bytes_sent=1234i,bytes_received=5678i 1705312200000
```

### Grafana Dashboard

Import the metrics into Grafana using InfluxDB as a data source. Example queries:

```flux
// CPU Usage over time
from(bucket: "sysinfo")
  |> range(start: -1h)
  |> filter(fn: (r) => r._measurement == "cpu")
  |> filter(fn: (r) => r._field == "usage_percent")

// Memory usage
from(bucket: "sysinfo")
  |> range(start: -1h)
  |> filter(fn: (r) => r._measurement == "memory")
  |> filter(fn: (r) => r._field == "usage_percent")
```

## Configuration Reference

### [server]

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `host` | string | `"127.0.0.1"` | Bind address |
| `port` | integer | `8080` | Listen port |
| `api_key` | string | `""` | API key (empty = no auth) |

### [server.cors]

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `allowed_origins` | array | `[]` | Allowed origins (empty = all) |
| `allow_credentials` | bool | `false` | Allow credentials |
| `max_age_seconds` | integer | `3600` | Preflight cache duration |

### [influxdb]

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | Enable InfluxDB persistence |
| `url` | string | `"http://localhost:8086"` | InfluxDB URL |
| `org` | string | `""` | Organization name |
| `bucket` | string | `""` | Bucket name |
| `token` | string | `""` | Auth token (supports `${ENV_VAR}`) |

### [collection]

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `interval_ms` | integer | `1000` | Collection interval (min: 100) |
| `enable_cpu` | bool | `true` | Enable CPU collector |
| `enable_gpu` | bool | `true` | Enable GPU collector |
| `enable_memory` | bool | `true` | Enable memory collector |
| `enable_storage` | bool | `true` | Enable storage collector |
| `enable_network` | bool | `true` | Enable network collector |

### [processes]

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `top_n` | integer | `10` | Number of processes to return |
| `sort_by` | string | `"cpu"` | Sort by `"cpu"` or `"memory"` |

### [services]

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `watchlist` | array | `[]` | Services to monitor (empty = all) |
| `include_stopped` | bool | `true` | Include stopped services |

### [events]

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_age_minutes` | integer | `60` | Max event age |
| `max_count` | integer | `50` | Max events to return |
| `severity` | array | `["error", "warning"]` | Severity filter |

### [logging]

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `level` | string | `"info"` | Log level (trace/debug/info/warn/error) |
| `file` | string | `null` | Optional log file path |

## Building from Source

### Requirements

- Rust 1.70+
- Windows SDK (for WMI bindings)
- NVIDIA CUDA Toolkit (optional, for GPU metrics)

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run
```

## Running as a Service

### Using NSSM (Non-Sucking Service Manager)

```powershell
# Install as service
nssm install SysInfoTelemetry "C:\path\to\sysinfo-telemetry.exe"
nssm set SysInfoTelemetry AppDirectory "C:\path\to"
nssm set SysInfoTelemetry DisplayName "SysInfo Telemetry Service"
nssm set SysInfoTelemetry Start SERVICE_AUTO_START

# Start service
nssm start SysInfoTelemetry
```

### Using Windows Task Scheduler

Create a task to run at startup with highest privileges for full metric access.

## Troubleshooting

### Common Issues

**GPU metrics not available**
- Ensure NVIDIA drivers are installed
- Check that `nvml.dll` is accessible

**Temperature readings are null**
- Run as Administrator for SMART/thermal data access

**Service PIDs are null**
- This is normal for stopped services

**High CPU usage on /processes endpoint**
- Consider increasing `interval_ms` in config
- Reduce `top_n` value

### Logging

Enable debug logging for troubleshooting:

```toml
[logging]
level = "debug"
```

Or via environment variable:

```bash
RUST_LOG=sysinfo_telemetry=debug ./sysinfo-telemetry.exe
```

## Contributing

Contributions are welcome! Please follow these guidelines:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Commit** your changes (`git commit -m 'feat: add amazing feature'`)
4. **Push** to the branch (`git push origin feature/amazing-feature`)
5. **Open** a Pull Request

### Commit Message Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `test:` Test additions/changes
- `chore:` Maintenance tasks

### Code Style

- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes without warnings
- Add documentation for public APIs

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) - Cross-platform system information
- [nvml-wrapper](https://github.com/Cldfire/nvml-wrapper) - NVIDIA Management Library bindings
- [Axum](https://github.com/tokio-rs/axum) - Ergonomic web framework
- [tokio](https://tokio.rs/) - Async runtime for Rust
