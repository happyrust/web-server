param(
    [int]$Port = 18081
)

$siteRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$configFile = Get-ChildItem -Path (Join-Path $siteRoot "config") -Filter *.toml | Select-Object -First 1
if (-not $configFile) {
    Write-Error "Config file not found under $siteRoot\\config"
    exit 1
}

$exe = Join-Path $siteRoot "bin/web_server.exe"
if (-not (Test-Path $exe)) {
    Write-Error "Missing web_server.exe at $exe"
    exit 1
}

$logDir = Join-Path $siteRoot "logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$logFile = Join-Path $logDir ("web_server_{0}.log" -f $Port)
$errFile = Join-Path $logDir ("web_server_{0}.err.log" -f $Port)
$pidFile = Join-Path $logDir "web_server.pid"

$configBase = [System.IO.Path]::Combine($configFile.DirectoryName, [System.IO.Path]::GetFileNameWithoutExtension($configFile.Name))
$workingDir = $siteRoot

$env:DB_OPTION_FILE = $configBase
$env:PORT = $Port

if (-not (Test-Path (Join-Path $siteRoot "rs_surreal"))) {
    $fallback = (Resolve-Path (Join-Path $siteRoot "..")).Path
    $workingDir = $fallback
}

Write-Host "Starting web_server: port $Port, config $($configFile.Name), workdir $workingDir"
$process = Start-Process -FilePath $exe -ArgumentList @("--config", $configBase) -NoNewWindow -PassThru -RedirectStandardOutput $logFile -RedirectStandardError $errFile -WorkingDirectory $workingDir
$process.Id | Set-Content $pidFile
Write-Host "Started, PID=$($process.Id), log=$logFile, errLog=$errFile"
