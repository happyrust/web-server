# Remote Sync Test Environment - Quick Start

## Overview

This directory contains a complete test environment for simulating remote synchronization between two sites (Site 1112 and Site 7000) on a single machine.

## Directory Structure

```
test-real/
├── README.md                 # This file
├── rumqttd.toml             # MQTT server configuration
├── site-1112/               # Site 1112 environment
│   ├── DbOption.toml        # Site 1112 configuration
│   ├── deployment_sites.sqlite  # SQLite database
│   ├── assets/archives/     # Archive storage
│   ├── output/remote_sync/  # Received sync files
│   └── surrealdb-data/      # SurrealDB data (created on first run)
└── site-7000/               # Site 7000 environment
    ├── DbOption.toml        # Site 7000 configuration
    ├── deployment_sites.sqlite  # SQLite database
    ├── assets/archives/     # Archive storage
    ├── output/remote_sync/  # Received sync files
    └── surrealdb-data/      # SurrealDB data (created on first run)
```

## Prerequisites

1. **Rust** - Ensure Rust is installed (https://rustup.rs/)
2. **SurrealDB** - Install SurrealDB:
   - Windows: `iwr https://windows.surrealdb.com -useb | iex`
   - Or: `cargo install --locked surrealdb`
3. **MQTT Server** - Either:
   - Use rumqttd (included in project or install via `cargo install rumqttd`)
   - Or use any MQTT broker on port 1883

## Quick Start

### 🚀 Automated Testing (Recommended)

The easiest way to test the environment is using the automated scripts:

#### Option 1: Run Complete Automated Test
```powershell
# Run a 5-minute automated test with file sync simulation
.\scripts\test-real\run-full-test.ps1 -Duration 300 -GenerateReport

# Run a 10-minute test and keep services running after
.\scripts\test-real\run-full-test.ps1 -Duration 600 -KeepServicesRunning -GenerateReport
```

This will:
1. Start all 6 services automatically
2. Run file sync simulation for the specified duration
3. Monitor service health
4. Generate a test report
5. Optionally stop services when done

#### Option 2: Manual Control
```powershell
# 1. Start all services (one command!)
.\scripts\test-real\start-all-services.ps1

# 2. Check service status
.\scripts\test-real\check-services-status.ps1

# 3. Start file sync simulation (runs continuously)
.\scripts\test-real\simulate-file-sync.ps1 -IntervalSeconds 30 -Continuous

# 4. View logs in real-time
.\scripts\test-real\view-logs.ps1 -Service web-1112 -Follow

# 5. Stop all services when done
.\scripts\test-real\stop-all-services.ps1
```

### 📋 Available Management Scripts

| Script | Description |
|--------|-------------|
| `start-all-services.ps1` | Start all 6 services in background |
| `stop-all-services.ps1` | Stop all running services |
| `check-services-status.ps1` | Check service health status |
| `view-logs.ps1` | View service logs |
| `simulate-file-sync.ps1` | Simulate file changes and trigger sync |
| `run-full-test.ps1` | Complete automated test workflow |

### 🛠️ Manual Setup (Alternative)

If you prefer to manage services manually:

#### Step 1: Initialize Databases

The SQLite databases have already been initialized. If you need to recreate them:

```powershell
# From project root
cd D:\work\plant\web-server

# Initialize Site 1112 database
cargo run --bin init_test_db --features web_server -- "remote-test-dir/test-real/site-1112" "Site1112-TestEnv" "SITE1112" "1112" "Site-7000" "SITE7000" "http://127.0.0.1:8082/files" "7000"

# Initialize Site 7000 database
cargo run --bin init_test_db --features web_server -- "remote-test-dir/test-real/site-7000" "Site7000-TestEnv" "SITE7000" "7000" "Site-1112" "SITE1112" "http://127.0.0.1:8081/files" "1112"
```

### Step 2: Start Services (6 Terminal Windows)

Open 6 separate terminal windows and run these commands:

#### Terminal 1: MQTT Server
```powershell
.\scripts\test-real\start-mqtt-server.ps1
```
Wait for "rumqttd started" message.

#### Terminal 2: SurrealDB - Site 1112
```powershell
.\scripts\test-real\start-surreal-1112.ps1
```
Wait for "Started web server on 127.0.0.1:8021" message.

#### Terminal 3: SurrealDB - Site 7000
```powershell
.\scripts\test-real\start-surreal-7000.ps1
```
Wait for "Started web server on 127.0.0.1:8022" message.

#### Terminal 4: Initialize Databases (Optional)
```powershell
.\scripts\test-real\init-database.ps1
```
This parses AVEVA database files and imports into SurrealDB. Skip if not testing with real AVEVA data.

#### Terminal 5: Web Server - Site 1112
```powershell
.\scripts\test-real\start-site-1112.ps1
```
Wait for server startup message.

#### Terminal 6: Web Server - Site 7000
```powershell
.\scripts\test-real\start-site-7000.ps1
```
Wait for server startup message.

## Port Allocation

| Service | Site 1112 | Site 7000 | Shared |
|---------|-----------|-----------|--------|
| HTTP | 8081 | 8082 | - |
| SurrealDB | 8021 | 8022 | - |
| MQTT | - | - | 1883 |

## Testing Sync

1. **Check Services**
   ```powershell
   # Check web servers
   Invoke-WebRequest -Uri http://127.0.0.1:8081/health
   Invoke-WebRequest -Uri http://127.0.0.1:8082/health

   # Check SurrealDB
   Invoke-WebRequest -Uri http://127.0.0.1:8021/health
   Invoke-WebRequest -Uri http://127.0.0.1:8022/health
   ```

2. **Monitor Logs**
   - Watch terminal outputs for sync activity
   - Check for file change detection
   - Look for HTTP transfer logs

3. **Verify Sync Files**
   ```powershell
   # Check Site 1112 sent files
   Get-ChildItem remote-test-dir\test-real\site-1112\assets\archives\ -Recurse

   # Check Site 7000 received files
   Get-ChildItem remote-test-dir\test-real\site-7000\output\remote_sync\ -Recurse
   ```

## Configuration Details

### Site 1112
- Location: SITE1112
- Database: 1112
- HTTP: http://127.0.0.1:8081
- File Upload: http://127.0.0.1:8081/files
- Remote Site: Site 7000 at http://127.0.0.1:8082/files

### Site 7000
- Location: SITE7000
- Database: 7000
- HTTP: http://127.0.0.1:8082
- File Upload: http://127.0.0.1:8082/files
- Remote Site: Site 1112 at http://127.0.0.1:8081/files

## 📊 Monitoring and Logs

### Real-time Monitoring
```powershell
# Watch service status (refreshes every 3 seconds)
.\scripts\test-real\check-services-status.ps1 -Watch

# View detailed service status
.\scripts\test-real\check-services-status.ps1 -Detailed
```

### Log Management
```powershell
# View all logs summary
.\scripts\test-real\view-logs.ps1

# View specific service log
.\scripts\test-real\view-logs.ps1 -Service web-1112

# Follow log in real-time (like tail -f)
.\scripts\test-real\view-logs.ps1 -Service mqtt -Follow

# View last 100 lines
.\scripts\test-real\view-logs.ps1 -Service surreal-1112 -Tail 100

# Clear all logs
.\scripts\test-real\view-logs.ps1 -Clear
```

### Log Files Location
All logs are stored in `remote-test-dir/test-real/logs/`:
- `mqtt.log` - MQTT server logs
- `surreal-1112.log` - SurrealDB (Site 1112) logs
- `surreal-7000.log` - SurrealDB (Site 7000) logs
- `web-1112.log` - Web Server (Site 1112) logs
- `web-7000.log` - Web Server (Site 7000) logs
- `sync-simulator.log` - File sync simulation logs
- `test-session-*.json` - Test session data
- `test-report-*.html` - Test reports (when using -GenerateReport)

## 🧪 File Sync Simulation

The file sync simulator copies AVEVA database files to trigger synchronization:

```powershell
# Single sync operation
.\scripts\test-real\simulate-file-sync.ps1

# Continuous sync every 30 seconds
.\scripts\test-real\simulate-file-sync.ps1 -IntervalSeconds 30 -Continuous

# Run for 5 minutes with custom settings
.\scripts\test-real\simulate-file-sync.ps1 -Duration 300 -IntervalSeconds 20 -FilesPerSync 2

# Sync only to Site 1112
.\scripts\test-real\simulate-file-sync.ps1 -Direction 1112 -Continuous

# Random sync direction (default)
.\scripts\test-real\simulate-file-sync.ps1 -Direction random -Continuous
```

### How It Works
1. Reads AVEVA project files from `D:/AVEVA/Projects/E3D2.1/AvevaMarineSample/ams000`
2. Randomly selects database directories (e.g., `ams1112_0001`, `ams7000_0001`)
3. Copies files to test site directories
4. Triggers file watchers in the web servers
5. Initiates synchronization between sites

## Troubleshooting

### Port Already in Use
```powershell
# Automated: Stop all services and kill port processes
.\scripts\test-real\stop-all-services.ps1 -Force

# Manual: Find and kill process
netstat -ano | findstr "8081"
taskkill /PID <pid> /F
```

### Clean and Restart
```powershell
# Automated: Stop services, clean, and restart
.\scripts\test-real\stop-all-services.ps1 -Force
Remove-Item remote-test-dir\test-real\site-1112\surrealdb-data -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item remote-test-dir\test-real\site-7000\surrealdb-data -Recurse -Force -ErrorAction SilentlyContinue
.\scripts\test-real\start-all-services.ps1 -Force
```

### Check What's Running
```powershell
# View background jobs
Get-Job

# Check services status
.\scripts\test-real\check-services-status.ps1 -Detailed

# Check port usage
Get-NetTCPConnection -LocalPort 8081,8082,8021,8022,1883
```

### Service Not Starting
1. Check logs: `.\scripts\test-real\view-logs.ps1 -Service <name>`
2. Verify prerequisites are installed (SurrealDB, MQTT server)
3. Check if ports are available
4. Try force restart: `.\scripts\test-real\start-all-services.ps1 -Force`

## 📈 Example Test Workflow

```powershell
# Complete automated test with report generation
.\scripts\test-real\run-full-test.ps1 -Duration 600 -SyncInterval 20 -GenerateReport

# Or manual step-by-step:
# 1. Start all services
.\scripts\test-real\start-all-services.ps1

# 2. Check everything is running
.\scripts\test-real\check-services-status.ps1

# 3. In a new terminal, start file sync simulation
.\scripts\test-real\simulate-file-sync.ps1 -IntervalSeconds 30 -Continuous

# 4. In another terminal, monitor logs
.\scripts\test-real\view-logs.ps1 -Service web-1112 -Follow

# 5. When done, stop everything
.\scripts\test-real\stop-all-services.ps1
```

## For More Information

See [docs/TEST_REAL_ENV_SETUP.md](../../docs/TEST_REAL_ENV_SETUP.md) for detailed documentation.
