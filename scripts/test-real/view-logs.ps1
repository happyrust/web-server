# View Logs for Test Services

param(
    [ValidateSet("mqtt", "surreal-1112", "surreal-7000", "web-1112", "web-7000", "sync-simulator", "all")]
    [string]$Service = "all",
    [int]$Tail = 50,      # Number of lines to show from end
    [switch]$Follow,      # Follow log file (like tail -f)
    [switch]$Clear        # Clear log file
)

$ErrorActionPreference = "Continue"

# Get project root and logs directory
$ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
$LogsDir = Join-Path $ProjectRoot "remote-test-dir" "test-real" "logs"

# Log file mapping
$LogFiles = @{
    "mqtt" = "mqtt.log"
    "surreal-1112" = "surreal-1112.log"
    "surreal-7000" = "surreal-7000.log"
    "web-1112" = "web-1112.log"
    "web-7000" = "web-7000.log"
    "sync-simulator" = "sync-simulator.log"
}

function Show-LogHeader {
    param($ServiceName, $LogPath)

    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "Log: $ServiceName" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "File: $LogPath" -ForegroundColor Gray

    if (Test-Path $LogPath) {
        $fileInfo = Get-Item $LogPath
        Write-Host "Size: $([int]($fileInfo.Length / 1KB)) KB" -ForegroundColor Gray
        Write-Host "Modified: $($fileInfo.LastWriteTime)" -ForegroundColor Gray
    } else {
        Write-Host "Status: File does not exist" -ForegroundColor Yellow
    }

    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
}

function Show-Log {
    param($ServiceName, $LogFile)

    $logPath = Join-Path $LogsDir $LogFile

    Show-LogHeader -ServiceName $ServiceName -LogPath $logPath

    if (-not (Test-Path $logPath)) {
        Write-Host "Log file not found: $logPath" -ForegroundColor Yellow
        Write-Host "The service may not have been started yet." -ForegroundColor Gray
        Write-Host ""
        return
    }

    if ($Clear) {
        Write-Host "Clearing log file..." -ForegroundColor Yellow
        Clear-Content -Path $logPath
        Write-Host "Log file cleared" -ForegroundColor Green
        Write-Host ""
        return
    }

    if ($Follow) {
        Write-Host "Following log (Press Ctrl+C to stop)..." -ForegroundColor Yellow
        Write-Host ""
        Get-Content -Path $logPath -Tail $Tail -Wait
    } else {
        $content = Get-Content -Path $logPath -Tail $Tail -ErrorAction SilentlyContinue

        if ($content) {
            $content | ForEach-Object {
                # Colorize output based on keywords
                $line = $_

                if ($line -match "error|failed|exception" -and $line -notmatch "0 error") {
                    Write-Host $line -ForegroundColor Red
                } elseif ($line -match "warning|warn") {
                    Write-Host $line -ForegroundColor Yellow
                } elseif ($line -match "success|completed|started|ready") {
                    Write-Host $line -ForegroundColor Green
                } elseif ($line -match "info|debug") {
                    Write-Host $line -ForegroundColor Gray
                } else {
                    Write-Host $line
                }
            }
        } else {
            Write-Host "Log file is empty" -ForegroundColor Gray
        }

        Write-Host ""
        Write-Host "Showing last $Tail lines. Use -Tail <n> to show more/less." -ForegroundColor Gray
        Write-Host "Use -Follow to continuously monitor the log." -ForegroundColor Gray
        Write-Host ""
    }
}

# Main execution
if ($Service -eq "all") {
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "All Service Logs" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""

    if ($Clear) {
        Write-Host "Clearing all log files..." -ForegroundColor Yellow

        foreach ($key in $LogFiles.Keys) {
            $logPath = Join-Path $LogsDir $LogFiles[$key]
            if (Test-Path $logPath) {
                Clear-Content -Path $logPath
                Write-Host "  Cleared: $($LogFiles[$key])" -ForegroundColor Green
            }
        }

        Write-Host ""
        Write-Host "All logs cleared" -ForegroundColor Green
        return
    }

    if ($Follow) {
        Write-Host "Cannot follow all logs simultaneously." -ForegroundColor Yellow
        Write-Host "Please specify a specific service with -Service parameter." -ForegroundColor Gray
        Write-Host ""
        Write-Host "Available services:" -ForegroundColor Cyan
        foreach ($key in $LogFiles.Keys) {
            Write-Host "  - $key" -ForegroundColor Gray
        }
        return
    }

    # Show summary of all logs
    foreach ($key in $LogFiles.Keys) {
        $logPath = Join-Path $LogsDir $LogFiles[$key]

        if (Test-Path $logPath) {
            $fileInfo = Get-Item $logPath
            $size = [int]($fileInfo.Length / 1KB)
            $modified = $fileInfo.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss")

            Write-Host "  [$key]" -ForegroundColor Cyan
            Write-Host "    File: $($LogFiles[$key])" -ForegroundColor Gray
            Write-Host "    Size: $size KB | Modified: $modified" -ForegroundColor Gray

            # Show last few lines
            $lastLines = Get-Content -Path $logPath -Tail 3 -ErrorAction SilentlyContinue
            if ($lastLines) {
                Write-Host "    Last lines:" -ForegroundColor DarkGray
                $lastLines | ForEach-Object {
                    $truncated = if ($_.Length -gt 80) { $_.Substring(0, 77) + "..." } else { $_ }
                    Write-Host "      $truncated" -ForegroundColor DarkGray
                }
            }
            Write-Host ""
        } else {
            Write-Host "  [$key] - No log file" -ForegroundColor Yellow
            Write-Host ""
        }
    }

    Write-Host "Use -Service <name> to view a specific log" -ForegroundColor Gray
    Write-Host "Example: .\view-logs.ps1 -Service web-1112 -Follow" -ForegroundColor Gray
    Write-Host ""

} else {
    if (-not $LogFiles.ContainsKey($Service)) {
        Write-Host "Unknown service: $Service" -ForegroundColor Red
        Write-Host ""
        Write-Host "Available services:" -ForegroundColor Cyan
        foreach ($key in $LogFiles.Keys) {
            Write-Host "  - $key" -ForegroundColor Gray
        }
        exit 1
    }

    Show-Log -ServiceName $Service -LogFile $LogFiles[$Service]
}
