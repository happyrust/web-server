# Run Complete Automated Test for Remote Sync
# This script orchestrates the entire test process

param(
    [int]$Duration = 300,           # Total test duration in seconds (default: 5 minutes)
    [int]$SyncInterval = 30,         # File sync interval in seconds
    [int]$FilesPerSync = 1,          # Number of files to copy per sync
    [switch]$SkipServiceStart,      # Skip starting services (use if already running)
    [switch]$KeepServicesRunning,   # Don't stop services at the end
    [switch]$GenerateReport         # Generate test report at the end
)

$ErrorActionPreference = "Continue"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Remote Sync Automated Test" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Get project root
$ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
$TestRealDir = Join-Path $ProjectRoot "remote-test-dir" "test-real"
$LogsDir = Join-Path $TestRealDir "logs"
$ScriptsDir = Join-Path $ProjectRoot "scripts" "test-real"

# Test configuration
Write-Host "Test Configuration:" -ForegroundColor Cyan
Write-Host "  Duration:       $Duration seconds ($([int]($Duration/60)) minutes)" -ForegroundColor Gray
Write-Host "  Sync Interval:  $SyncInterval seconds" -ForegroundColor Gray
Write-Host "  Files Per Sync: $FilesPerSync" -ForegroundColor Gray
Write-Host ""

# Create test session info
$testSessionId = (Get-Date -Format "yyyyMMdd-HHmmss")
$testSessionFile = Join-Path $LogsDir "test-session-$testSessionId.json"

$testSession = @{
    SessionId = $testSessionId
    StartTime = Get-Date
    Duration = $Duration
    SyncInterval = $SyncInterval
    FilesPerSync = $FilesPerSync
    Status = "Running"
    Events = @()
}

function Add-TestEvent {
    param([string]$Message, [string]$Type = "Info")

    $event = @{
        Timestamp = Get-Date
        Type = $Type
        Message = $Message
    }

    $testSession.Events += $event

    $color = switch ($Type) {
        "Error" { "Red" }
        "Warning" { "Yellow" }
        "Success" { "Green" }
        default { "White" }
    }

    Write-Host "[$(Get-Date -Format 'HH:mm:ss')] $Message" -ForegroundColor $color
}

# Step 1: Start services (if needed)
if (-not $SkipServiceStart) {
    Add-TestEvent "Starting all services..." "Info"
    Write-Host ""

    & "$ScriptsDir\start-all-services.ps1"

    if ($LASTEXITCODE -ne 0 -and $null -ne $LASTEXITCODE) {
        Add-TestEvent "Failed to start services" "Error"
        $testSession.Status = "Failed"
        $testSession | ConvertTo-Json -Depth 10 | Out-File $testSessionFile
        exit 1
    }

    Add-TestEvent "All services started successfully" "Success"
    Write-Host ""

    # Wait for services to stabilize
    Add-TestEvent "Waiting 10 seconds for services to stabilize..." "Info"
    Start-Sleep -Seconds 10
} else {
    Add-TestEvent "Skipping service startup (using existing services)" "Info"
}

# Step 2: Verify all services are running
Add-TestEvent "Verifying service health..." "Info"
Write-Host ""

& "$ScriptsDir\check-services-status.ps1"

Write-Host ""

# Step 3: Start file sync simulation
Add-TestEvent "Starting file sync simulation..." "Info"
Write-Host ""

$syncJobName = "TestSession-FileSync-$testSessionId"

$syncJob = Start-Job -Name $syncJobName -ScriptBlock {
    param($ScriptPath, $Duration, $SyncInterval, $FilesPerSync, $LogPath)

    & PowerShell -ExecutionPolicy Bypass -File $ScriptPath `
        -Duration $Duration `
        -IntervalSeconds $SyncInterval `
        -FilesPerSync $FilesPerSync `
        -Direction "random" `
        -Continuous `
        *>&1 | Tee-Object -FilePath $LogPath

} -ArgumentList @(
    (Join-Path $ScriptsDir "simulate-file-sync.ps1"),
    $Duration,
    $SyncInterval,
    $FilesPerSync,
    (Join-Path $LogsDir "test-sync-$testSessionId.log")
)

Add-TestEvent "File sync simulation started (Job ID: $($syncJob.Id))" "Success"

# Step 4: Monitor test progress
Add-TestEvent "Test is now running for $Duration seconds..." "Info"
Write-Host ""
Write-Host "Monitoring test progress (Press Ctrl+C to stop early)..." -ForegroundColor Cyan
Write-Host ""

$startTime = Get-Date
$lastStatusCheck = $startTime

try {
    while ($true) {
        $elapsed = ((Get-Date) - $startTime).TotalSeconds

        # Check if duration reached
        if ($elapsed -ge $Duration) {
            Add-TestEvent "Test duration reached" "Info"
            break
        }

        # Periodic status check (every 30 seconds)
        if (((Get-Date) - $lastStatusCheck).TotalSeconds -ge 30) {
            $lastStatusCheck = Get-Date

            Write-Host ""
            Write-Host "--- Progress: $([int]$elapsed)/$Duration seconds ---" -ForegroundColor Cyan

            # Check if sync job is still running
            $syncJobState = (Get-Job -Id $syncJob.Id).State
            if ($syncJobState -ne "Running") {
                Add-TestEvent "File sync job stopped unexpectedly (State: $syncJobState)" "Warning"
            }

            # Quick service health check
            $servicesOk = $true
            @(8081, 8082, 8021, 8022, 1883) | ForEach-Object {
                $port = $_
                $conn = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue
                if (-not $conn) {
                    Add-TestEvent "Port $port is not in use - service may have stopped" "Warning"
                    $servicesOk = $false
                }
            }

            if ($servicesOk) {
                Write-Host "All services are still running" -ForegroundColor Green
            }

            Write-Host ""
        }

        Start-Sleep -Seconds 5
    }

} catch {
    Add-TestEvent "Test interrupted by user" "Warning"
}

# Step 5: Stop file sync
Add-TestEvent "Stopping file sync simulation..." "Info"
Stop-Job -Id $syncJob.Id -ErrorAction SilentlyContinue
Wait-Job -Id $syncJob.Id -Timeout 10 | Out-Null
Remove-Job -Id $syncJob.Id -Force -ErrorAction SilentlyContinue

# Step 6: Collect results
Add-TestEvent "Collecting test results..." "Info"
Write-Host ""

$endTime = Get-Date
$actualDuration = ($endTime - $testSession.StartTime).TotalSeconds

$testSession.EndTime = $endTime
$testSession.ActualDuration = $actualDuration
$testSession.Status = "Completed"

# Check sync simulator log for stats
$syncLogPath = Join-Path $LogsDir "sync-simulator.log"
if (Test-Path $syncLogPath) {
    $syncEntries = Get-Content $syncLogPath -ErrorAction SilentlyContinue | ForEach-Object {
        try { $_ | ConvertFrom-Json } catch { $null }
    } | Where-Object { $_ -ne $null }

    $testSession.SyncStats = @{
        TotalOperations = $syncEntries.Count
        Site1112Operations = ($syncEntries | Where-Object { $_.Site -eq "Site-1112" }).Count
        Site7000Operations = ($syncEntries | Where-Object { $_.Site -eq "Site-7000" }).Count
        TotalFilesCopied = ($syncEntries | Measure-Object -Property FilesCount -Sum).Sum
    }
}

# Save test session
$testSession | ConvertTo-Json -Depth 10 | Out-File $testSessionFile

Add-TestEvent "Test session saved to: $testSessionFile" "Success"

# Step 7: Display summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Test Summary" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Session ID:     $testSessionId" -ForegroundColor Cyan
Write-Host "  Start Time:     $($testSession.StartTime.ToString('yyyy-MM-dd HH:mm:ss'))" -ForegroundColor Gray
Write-Host "  End Time:       $($testSession.EndTime.ToString('yyyy-MM-dd HH:mm:ss'))" -ForegroundColor Gray
Write-Host "  Duration:       $([int]$actualDuration) seconds" -ForegroundColor Gray
Write-Host ""

if ($testSession.SyncStats) {
    Write-Host "  Sync Operations:" -ForegroundColor Cyan
    Write-Host "    Total:        $($testSession.SyncStats.TotalOperations)" -ForegroundColor Gray
    Write-Host "    Site 1112:    $($testSession.SyncStats.Site1112Operations)" -ForegroundColor Gray
    Write-Host "    Site 7000:    $($testSession.SyncStats.Site7000Operations)" -ForegroundColor Gray
    Write-Host "    Files Copied: $($testSession.SyncStats.TotalFilesCopied)" -ForegroundColor Gray
    Write-Host ""
}

Write-Host "  Events:         $($testSession.Events.Count)" -ForegroundColor Gray
$errorCount = ($testSession.Events | Where-Object { $_.Type -eq "Error" }).Count
$warningCount = ($testSession.Events | Where-Object { $_.Type -eq "Warning" }).Count
if ($errorCount -gt 0) {
    Write-Host "    Errors:       $errorCount" -ForegroundColor Red
}
if ($warningCount -gt 0) {
    Write-Host "    Warnings:     $warningCount" -ForegroundColor Yellow
}
Write-Host ""

# Step 8: Cleanup (optional)
if (-not $KeepServicesRunning) {
    Add-TestEvent "Stopping all services..." "Info"
    Write-Host ""
    & "$ScriptsDir\stop-all-services.ps1"
    Write-Host ""
} else {
    Write-Host "Services are still running (use -KeepServicesRunning=`$false to stop)" -ForegroundColor Yellow
    Write-Host ""
}

# Step 9: Generate report (optional)
if ($GenerateReport) {
    $reportPath = Join-Path $LogsDir "test-report-$testSessionId.html"
    Add-TestEvent "Generating HTML report..." "Info"

    $html = @"
<!DOCTYPE html>
<html>
<head>
    <title>Test Report - $testSessionId</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        h1 { color: #0066cc; }
        table { border-collapse: collapse; width: 100%; margin: 20px 0; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #0066cc; color: white; }
        .error { color: red; }
        .warning { color: orange; }
        .success { color: green; }
    </style>
</head>
<body>
    <h1>Remote Sync Test Report</h1>
    <h2>Session: $testSessionId</h2>
    <p><strong>Start Time:</strong> $($testSession.StartTime.ToString('yyyy-MM-dd HH:mm:ss'))</p>
    <p><strong>End Time:</strong> $($testSession.EndTime.ToString('yyyy-MM-dd HH:mm:ss'))</p>
    <p><strong>Duration:</strong> $([int]$actualDuration) seconds</p>
    <p><strong>Status:</strong> $($testSession.Status)</p>

    <h2>Sync Statistics</h2>
    <table>
        <tr><th>Metric</th><th>Value</th></tr>
        <tr><td>Total Operations</td><td>$($testSession.SyncStats.TotalOperations)</td></tr>
        <tr><td>Site 1112 Operations</td><td>$($testSession.SyncStats.Site1112Operations)</td></tr>
        <tr><td>Site 7000 Operations</td><td>$($testSession.SyncStats.Site7000Operations)</td></tr>
        <tr><td>Total Files Copied</td><td>$($testSession.SyncStats.TotalFilesCopied)</td></tr>
    </table>

    <h2>Events</h2>
    <table>
        <tr><th>Time</th><th>Type</th><th>Message</th></tr>
$(
    $testSession.Events | ForEach-Object {
        $class = $_.Type.ToLower()
        $time = ([datetime]$_.Timestamp).ToString('HH:mm:ss')
        "        <tr><td>$time</td><td class='$class'>$($_.Type)</td><td>$($_.Message)</td></tr>"
    }
) -join "`n"
    </table>
</body>
</html>
"@

    $html | Out-File -FilePath $reportPath -Encoding UTF8
    Add-TestEvent "Report generated: $reportPath" "Success"
    Write-Host ""
}

Write-Host "========================================" -ForegroundColor Green
Write-Host "Test Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Session file: $testSessionFile" -ForegroundColor Cyan
if ($GenerateReport) {
    Write-Host "Report: $reportPath" -ForegroundColor Cyan
}
Write-Host ""
Write-Host "View logs: .\scripts\test-real\view-logs.ps1" -ForegroundColor Gray
Write-Host ""
