# Start All Services for Remote Sync Test Environment
# This script starts all required services in background jobs

param(
    [switch]$NoWait,  # Don't wait for services to be ready
    [switch]$Force    # Force restart if already running
)

$ErrorActionPreference = "Continue"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Starting All Test Services" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Get project root
$ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
$TestRealDir = Join-Path $ProjectRoot "remote-test-dir" "test-real"
$LogsDir = Join-Path $TestRealDir "logs"

# Create logs directory
if (-not (Test-Path $LogsDir)) {
    New-Item -ItemType Directory -Force -Path $LogsDir | Out-Null
    Write-Host "Created logs directory: $LogsDir" -ForegroundColor Green
}

# Service configuration
$Services = @(
    @{
        Name = "MQTT"
        ScriptPath = "scripts\test-real\start-mqtt-server.ps1"
        LogFile = "mqtt.log"
        HealthCheck = { Test-NetConnection -ComputerName 127.0.0.1 -Port 1883 -WarningAction SilentlyContinue -InformationLevel Quiet }
        WaitTime = 3
    },
    @{
        Name = "SurrealDB-1112"
        ScriptPath = "scripts\test-real\start-surreal-1112.ps1"
        LogFile = "surreal-1112.log"
        HealthCheck = {
            try {
                $response = Invoke-WebRequest -Uri "http://127.0.0.1:8021/health" -TimeoutSec 2 -ErrorAction Stop
                return $response.StatusCode -eq 200
            } catch { return $false }
        }
        WaitTime = 5
    },
    @{
        Name = "SurrealDB-7000"
        ScriptPath = "scripts\test-real\start-surreal-7000.ps1"
        LogFile = "surreal-7000.log"
        HealthCheck = {
            try {
                $response = Invoke-WebRequest -Uri "http://127.0.0.1:8022/health" -TimeoutSec 2 -ErrorAction Stop
                return $response.StatusCode -eq 200
            } catch { return $false }
        }
        WaitTime = 5
    },
    @{
        Name = "WebServer-1112"
        ScriptPath = "scripts\test-real\start-site-1112.ps1"
        LogFile = "web-1112.log"
        HealthCheck = {
            try {
                $response = Invoke-WebRequest -Uri "http://127.0.0.1:8081/health" -TimeoutSec 2 -ErrorAction Stop
                return $response.StatusCode -eq 200
            } catch { return $false }
        }
        WaitTime = 10
        DependsOn = @("SurrealDB-1112", "MQTT")
    },
    @{
        Name = "WebServer-7000"
        ScriptPath = "scripts\test-real\start-site-7000.ps1"
        LogFile = "web-7000.log"
        HealthCheck = {
            try {
                $response = Invoke-WebRequest -Uri "http://127.0.0.1:8082/health" -TimeoutSec 2 -ErrorAction Stop
                return $response.StatusCode -eq 200
            } catch { return $false }
        }
        WaitTime = 10
        DependsOn = @("SurrealDB-7000", "MQTT")
    }
)

# Check if services are already running
$RunningJobs = Get-Job -Name "TestService-*" -ErrorAction SilentlyContinue
if ($RunningJobs) {
    if ($Force) {
        Write-Host "Stopping existing services..." -ForegroundColor Yellow
        $RunningJobs | Stop-Job
        $RunningJobs | Remove-Job -Force
        Start-Sleep -Seconds 2
    } else {
        Write-Host "Services are already running. Use -Force to restart." -ForegroundColor Yellow
        Write-Host "Running jobs:" -ForegroundColor Gray
        $RunningJobs | ForEach-Object { Write-Host "  - $($_.Name)" -ForegroundColor Gray }
        Write-Host ""
        Write-Host "To view status: .\scripts\test-real\check-services-status.ps1" -ForegroundColor Cyan
        Write-Host "To stop all: .\scripts\test-real\stop-all-services.ps1" -ForegroundColor Cyan
        exit 0
    }
}

# Function to start a service
function Start-Service {
    param($ServiceConfig)

    $jobName = "TestService-$($ServiceConfig.Name)"
    $logPath = Join-Path $LogsDir $ServiceConfig.LogFile
    $scriptPath = Join-Path $ProjectRoot $ServiceConfig.ScriptPath

    Write-Host "[$($ServiceConfig.Name)] Starting..." -ForegroundColor Yellow

    # Start the service as a background job
    $job = Start-Job -Name $jobName -ScriptBlock {
        param($ScriptPath, $LogPath)
        & PowerShell -ExecutionPolicy Bypass -File $ScriptPath *>&1 | Tee-Object -FilePath $LogPath
    } -ArgumentList $scriptPath, $logPath

    Write-Host "[$($ServiceConfig.Name)] Started (Job ID: $($job.Id))" -ForegroundColor Green
    Write-Host "[$($ServiceConfig.Name)] Log: $logPath" -ForegroundColor Gray

    return $job
}

# Function to wait for service to be ready
function Wait-ServiceReady {
    param($ServiceConfig, $MaxWaitSeconds = 30)

    Write-Host "[$($ServiceConfig.Name)] Waiting for service to be ready..." -ForegroundColor Yellow

    $elapsed = 0
    $checkInterval = 1

    while ($elapsed -lt $MaxWaitSeconds) {
        $isReady = & $ServiceConfig.HealthCheck

        if ($isReady) {
            Write-Host "[$($ServiceConfig.Name)] Ready!" -ForegroundColor Green
            return $true
        }

        Start-Sleep -Seconds $checkInterval
        $elapsed += $checkInterval
        Write-Host "." -NoNewline -ForegroundColor Gray
    }

    Write-Host ""
    Write-Host "[$($ServiceConfig.Name)] Timeout waiting for service" -ForegroundColor Red
    return $false
}

# Start services in order
$StartedServices = @{}

foreach ($service in $Services) {
    # Check dependencies
    if ($service.DependsOn) {
        Write-Host "[$($service.Name)] Checking dependencies: $($service.DependsOn -join ', ')" -ForegroundColor Cyan

        foreach ($dep in $service.DependsOn) {
            if (-not $StartedServices.ContainsKey($dep)) {
                Write-Host "[$($service.Name)] Waiting for dependency: $dep" -ForegroundColor Yellow
                Start-Sleep -Seconds 2
            }
        }
    }

    # Start the service
    $job = Start-Service -ServiceConfig $service
    $StartedServices[$service.Name] = $job

    if (-not $NoWait) {
        # Wait for initial startup
        Start-Sleep -Seconds $service.WaitTime

        # Health check
        $ready = Wait-ServiceReady -ServiceConfig $service

        if (-not $ready) {
            Write-Host "[$($service.Name)] Failed to start properly. Check logs: $($service.LogFile)" -ForegroundColor Red
        }
    }

    Write-Host ""
}

# Summary
Write-Host "========================================" -ForegroundColor Green
Write-Host "Service Startup Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""

Write-Host "Started Services:" -ForegroundColor Cyan
foreach ($service in $Services) {
    $job = $StartedServices[$service.Name]
    $status = if ($job.State -eq "Running") { "Running" } else { $job.State }
    $color = if ($job.State -eq "Running") { "Green" } else { "Red" }
    Write-Host "  [$status] $($service.Name)" -ForegroundColor $color
}

Write-Host ""
Write-Host "Service Endpoints:" -ForegroundColor Cyan
Write-Host "  MQTT:              127.0.0.1:1883" -ForegroundColor Gray
Write-Host "  SurrealDB (1112):  http://127.0.0.1:8021" -ForegroundColor Gray
Write-Host "  SurrealDB (7000):  http://127.0.0.1:8022" -ForegroundColor Gray
Write-Host "  Web Server (1112): http://127.0.0.1:8081" -ForegroundColor Gray
Write-Host "  Web Server (7000): http://127.0.0.1:8082" -ForegroundColor Gray

Write-Host ""
Write-Host "Logs Directory: $LogsDir" -ForegroundColor Cyan

Write-Host ""
Write-Host "Useful Commands:" -ForegroundColor Yellow
Write-Host "  Check status:  .\scripts\test-real\check-services-status.ps1" -ForegroundColor Gray
Write-Host "  View logs:     .\scripts\test-real\view-logs.ps1 -Service <name>" -ForegroundColor Gray
Write-Host "  Stop all:      .\scripts\test-real\stop-all-services.ps1" -ForegroundColor Gray
Write-Host "  Simulate sync: .\scripts\test-real\simulate-file-sync.ps1" -ForegroundColor Gray
Write-Host ""

# Save service info for management scripts
$ServiceInfo = @{
    Services = $Services
    LogsDir = $LogsDir
    StartedAt = Get-Date
} | ConvertTo-Json -Depth 10

$ServiceInfo | Out-File -FilePath (Join-Path $LogsDir "services.json") -Encoding UTF8

Write-Host "Service information saved to: $LogsDir\services.json" -ForegroundColor Green
Write-Host ""
