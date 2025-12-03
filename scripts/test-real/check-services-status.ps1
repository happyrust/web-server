# Check Status of All Test Services

param(
    [switch]$Detailed,  # Show detailed information
    [switch]$Watch,     # Continuously monitor (refresh every 3 seconds)
    [int]$RefreshInterval = 3  # Refresh interval for watch mode
)

$ErrorActionPreference = "Continue"

function Show-ServiceStatus {
    Clear-Host

    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Test Services Status" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Time: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor Gray
    Write-Host ""

    # Service configurations
    $services = @(
        @{
            Name = "MQTT"
            Port = 1883
            Type = "TCP"
            HealthCheck = {
                Test-NetConnection -ComputerName 127.0.0.1 -Port 1883 -WarningAction SilentlyContinue -InformationLevel Quiet
            }
        },
        @{
            Name = "SurrealDB-1112"
            Port = 8021
            Type = "HTTP"
            Url = "http://127.0.0.1:8021/health"
            HealthCheck = {
                try {
                    $response = Invoke-WebRequest -Uri "http://127.0.0.1:8021/health" -TimeoutSec 2 -ErrorAction Stop
                    return $response.StatusCode -eq 200
                } catch { return $false }
            }
        },
        @{
            Name = "SurrealDB-7000"
            Port = 8022
            Type = "HTTP"
            Url = "http://127.0.0.1:8022/health"
            HealthCheck = {
                try {
                    $response = Invoke-WebRequest -Uri "http://127.0.0.1:8022/health" -TimeoutSec 2 -ErrorAction Stop
                    return $response.StatusCode -eq 200
                } catch { return $false }
            }
        },
        @{
            Name = "WebServer-1112"
            Port = 8081
            Type = "HTTP"
            Url = "http://127.0.0.1:8081/health"
            HealthCheck = {
                try {
                    $response = Invoke-WebRequest -Uri "http://127.0.0.1:8081/health" -TimeoutSec 2 -ErrorAction Stop
                    return $response.StatusCode -eq 200
                } catch { return $false }
            }
        },
        @{
            Name = "WebServer-7000"
            Port = 8082
            Type = "HTTP"
            Url = "http://127.0.0.1:8082/health"
            HealthCheck = {
                try {
                    $response = Invoke-WebRequest -Uri "http://127.0.0.1:8082/health" -TimeoutSec 2 -ErrorAction Stop
                    return $response.StatusCode -eq 200
                } catch { return $false }
            }
        }
    )

    # Check background jobs
    Write-Host "Background Jobs:" -ForegroundColor Cyan
    $jobs = Get-Job -Name "TestService-*" -ErrorAction SilentlyContinue

    if ($jobs) {
        foreach ($job in $jobs) {
            $serviceName = $job.Name -replace "TestService-", ""
            $status = $job.State
            $color = switch ($status) {
                "Running" { "Green" }
                "Completed" { "Gray" }
                "Failed" { "Red" }
                "Stopped" { "Yellow" }
                default { "White" }
            }

            $statusIcon = switch ($status) {
                "Running" { "✓" }
                "Failed" { "✗" }
                "Stopped" { "■" }
                default { "?" }
            }

            Write-Host "  $statusIcon $serviceName" -ForegroundColor $color -NoNewline
            Write-Host " [$status]" -ForegroundColor Gray

            if ($Detailed) {
                Write-Host "    Job ID: $($job.Id)" -ForegroundColor DarkGray
                if ($job.PSBeginTime) {
                    $uptime = (Get-Date) - $job.PSBeginTime
                    Write-Host "    Uptime: $([int]$uptime.TotalMinutes)m $($uptime.Seconds)s" -ForegroundColor DarkGray
                }
            }
        }
    } else {
        Write-Host "  No background jobs found" -ForegroundColor Yellow
        Write-Host "  (Services may be running in separate terminals)" -ForegroundColor Gray
    }

    Write-Host ""

    # Check service endpoints
    Write-Host "Service Endpoints:" -ForegroundColor Cyan

    $allHealthy = $true

    foreach ($service in $services) {
        $isHealthy = & $service.HealthCheck

        $statusIcon = if ($isHealthy) { "✓" } else { "✗" }
        $color = if ($isHealthy) { "Green" } else { "Red" }
        $statusText = if ($isHealthy) { "Running" } else { "Down" }

        Write-Host "  $statusIcon $($service.Name)" -ForegroundColor $color -NoNewline
        Write-Host " [$statusText]" -ForegroundColor Gray

        if ($Detailed) {
            Write-Host "    Port: $($service.Port)" -ForegroundColor DarkGray
            if ($service.Url) {
                Write-Host "    URL:  $($service.Url)" -ForegroundColor DarkGray
            }

            # Check port
            $connection = Get-NetTCPConnection -LocalPort $service.Port -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($connection) {
                $process = Get-Process -Id $connection.OwningProcess -ErrorAction SilentlyContinue
                if ($process) {
                    Write-Host "    Process: $($process.Name) (PID: $($process.Id))" -ForegroundColor DarkGray
                    Write-Host "    CPU: $([int]$process.CPU)s, Memory: $([int]($process.WorkingSet64 / 1MB))MB" -ForegroundColor DarkGray
                }
            }
        }

        if (-not $isHealthy) {
            $allHealthy = $false
        }
    }

    Write-Host ""

    # Summary
    $healthyCount = ($services | Where-Object { & $_.HealthCheck }).Count
    $totalCount = $services.Count

    Write-Host "Summary:" -ForegroundColor Cyan
    Write-Host "  Status: $healthyCount/$totalCount services running" -ForegroundColor $(if ($allHealthy) { "Green" } else { "Yellow" })

    # Check logs directory
    $ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
    $LogsDir = Join-Path $ProjectRoot "remote-test-dir" "test-real" "logs"

    if (Test-Path $LogsDir) {
        $logFiles = Get-ChildItem -Path $LogsDir -Filter "*.log" -ErrorAction SilentlyContinue
        if ($logFiles) {
            Write-Host "  Logs: $($logFiles.Count) log files in $LogsDir" -ForegroundColor Gray
        }
    }

    Write-Host ""

    # Recommendations
    if (-not $allHealthy) {
        Write-Host "Troubleshooting:" -ForegroundColor Yellow
        Write-Host "  1. Check logs: .\scripts\test-real\view-logs.ps1" -ForegroundColor Gray
        Write-Host "  2. Restart services: .\scripts\test-real\start-all-services.ps1 -Force" -ForegroundColor Gray
        Write-Host "  3. Check ports: netstat -ano | findstr \"8081 8082 8021 8022 1883\"" -ForegroundColor Gray
        Write-Host ""
    }

    if ($Watch) {
        Write-Host "Press Ctrl+C to exit watch mode" -ForegroundColor Gray
    }
}

# Main execution
if ($Watch) {
    try {
        while ($true) {
            Show-ServiceStatus
            Start-Sleep -Seconds $RefreshInterval
        }
    } catch {
        Write-Host ""
        Write-Host "Watch mode stopped" -ForegroundColor Yellow
    }
} else {
    Show-ServiceStatus
}
