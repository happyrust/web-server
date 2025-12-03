# Stop All Services for Remote Sync Test Environment

param(
    [switch]$Force  # Force kill services
)

$ErrorActionPreference = "Continue"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Stopping All Test Services" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Get all test service jobs
$RunningJobs = Get-Job -Name "TestService-*" -ErrorAction SilentlyContinue

if (-not $RunningJobs) {
    Write-Host "No running services found." -ForegroundColor Yellow
    Write-Host ""
    exit 0
}

Write-Host "Found $($RunningJobs.Count) running service(s):" -ForegroundColor Cyan
$RunningJobs | ForEach-Object {
    $serviceName = $_.Name -replace "TestService-", ""
    $status = $_.State
    $color = switch ($status) {
        "Running" { "Green" }
        "Completed" { "Gray" }
        "Failed" { "Red" }
        default { "Yellow" }
    }
    Write-Host "  - $serviceName [$status]" -ForegroundColor $color
}
Write-Host ""

# Stop services
if ($Force) {
    Write-Host "Force stopping all services..." -ForegroundColor Yellow
    $RunningJobs | Stop-Job -PassThru | Remove-Job -Force
} else {
    Write-Host "Gracefully stopping services..." -ForegroundColor Yellow

    # Send stop signal
    $RunningJobs | Stop-Job

    # Wait for jobs to finish (max 10 seconds)
    $timeout = 10
    $elapsed = 0
    while (($RunningJobs | Where-Object { $_.State -eq "Running" }).Count -gt 0 -and $elapsed -lt $timeout) {
        Start-Sleep -Seconds 1
        $elapsed++
        Write-Host "." -NoNewline -ForegroundColor Gray
    }
    Write-Host ""

    # Force remove any remaining jobs
    $stillRunning = $RunningJobs | Where-Object { $_.State -eq "Running" }
    if ($stillRunning) {
        Write-Host "Force removing $($stillRunning.Count) service(s) that did not stop gracefully..." -ForegroundColor Yellow
        $stillRunning | Remove-Job -Force
    }

    # Remove completed jobs
    $RunningJobs | Remove-Job -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "All services stopped." -ForegroundColor Green

# Also try to kill any remaining processes on the ports
Write-Host ""
Write-Host "Checking for processes still using ports..." -ForegroundColor Cyan

$ports = @(1883, 8021, 8022, 8081, 8082)

foreach ($port in $ports) {
    $connections = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue

    if ($connections) {
        Write-Host "  Port $port is still in use by process(es):" -ForegroundColor Yellow

        $connections | ForEach-Object {
            $processId = $_.OwningProcess
            $process = Get-Process -Id $processId -ErrorAction SilentlyContinue

            if ($process) {
                Write-Host "    PID $processId - $($process.Name)" -ForegroundColor Gray

                if ($Force) {
                    Write-Host "    Killing process $processId..." -ForegroundColor Red
                    Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
                }
            }
        }
    }
}

if (-not $Force) {
    Write-Host ""
    Write-Host "Hint: Use -Force to kill processes still using ports" -ForegroundColor Gray
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Cleanup Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
