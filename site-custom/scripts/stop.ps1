param(
    [switch]$KillAll = $false
)

$siteRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$pidFile = Join-Path $siteRoot "logs/web_server.pid"

if (Test-Path $pidFile) {
    $pidFromFile = Get-Content $pidFile | Select-Object -First 1
    if ($pidFromFile) {
        try {
            Stop-Process -Id $pidFromFile -ErrorAction Stop
            Write-Host "Stopped web_server PID=$pidFromFile"
        } catch {
            Write-Warning "Stop by PID failed: $_"
        }
    }
    Remove-Item $pidFile -ErrorAction SilentlyContinue
} else {
    Write-Host "No PID file found at $pidFile"
}

if ($KillAll) {
    # Optional fallback: stop any remaining processes by name
    Get-Process -Name "web_server" -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            Stop-Process -Id $_.Id -ErrorAction Stop
            Write-Host "Killed web_server process $($_.Id)"
        } catch {
            Write-Warning "Failed to kill process $($_.Id): $_"
        }
    }
}
