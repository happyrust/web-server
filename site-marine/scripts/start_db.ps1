param(
    [string]$Command = ""
)

$siteRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$logDir = Join-Path $siteRoot "logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$logFile = Join-Path $logDir "db_start.log"
$errFile = Join-Path $logDir "db_start.err.log"
$pidFile = Join-Path $logDir "db.pid"

if ([string]::IsNullOrWhiteSpace($Command)) {
    Write-Host 'DB start command not provided, skip. Use -Command "your_db_start_cmd" when needed.'
    exit 0
}

Write-Host "Running DB start command: $Command"
$process = Start-Process -FilePath "pwsh" -ArgumentList "-NoProfile","-Command",$Command -WorkingDirectory $siteRoot -PassThru -RedirectStandardOutput $logFile -RedirectStandardError $errFile
$process.Id | Set-Content $pidFile
Write-Host "DB start attempted, PID=$($process.Id), log=$logFile, errLog=$errFile"
