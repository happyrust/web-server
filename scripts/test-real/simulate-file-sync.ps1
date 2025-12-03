# Simulate File Synchronization Between Sites
# This script periodically copies AVEVA database files to trigger sync

param(
    [int]$IntervalSeconds = 30,  # Time between sync operations
    [int]$Duration = 0,           # Total duration in seconds (0 = infinite)
    [ValidateSet("1112", "7000", "both", "random")]
    [string]$Direction = "random", # Which direction to sync
    [int]$FilesPerSync = 1,       # Number of files to copy per sync
    [switch]$Continuous          # Keep running until manually stopped
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "File Sync Simulator" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Get project root
$ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
$TestRealDir = Join-Path $ProjectRoot "remote-test-dir" "test-real"
$LogsDir = Join-Path $TestRealDir "logs"

# Source directory (AVEVA project files)
$SourceDir = "D:\AVEVA\Projects\E3D2.1\AvevaMarineSample\ams000"

# Target directories for each site
$Site1112Dir = Join-Path $TestRealDir "site-1112"
$Site7000Dir = Join-Path $TestRealDir "site-7000"

# Database directories within sites
$Site1112DbDir = Join-Path $Site1112Dir "AvevaMarineSample\ams000"
$Site7000DbDir = Join-Path $Site7000Dir "AvevaMarineSample\ams000"

Write-Host "Configuration:" -ForegroundColor Cyan
Write-Host "  Source Directory:  $SourceDir" -ForegroundColor Gray
Write-Host "  Site 1112 DB Dir:  $Site1112DbDir" -ForegroundColor Gray
Write-Host "  Site 7000 DB Dir:  $Site7000DbDir" -ForegroundColor Gray
Write-Host "  Interval:          $IntervalSeconds seconds" -ForegroundColor Gray
Write-Host "  Direction:         $Direction" -ForegroundColor Gray
Write-Host "  Files Per Sync:    $FilesPerSync" -ForegroundColor Gray
Write-Host ""

# Check if source directory exists
if (-not (Test-Path $SourceDir)) {
    Write-Host "Error: Source directory not found: $SourceDir" -ForegroundColor Red
    Write-Host ""
    Write-Host "This simulator requires real AVEVA project files." -ForegroundColor Yellow
    Write-Host "Please ensure D:\AVEVA\Projects\E3D2.1 is installed." -ForegroundColor Yellow
    Write-Host ""
    exit 1
}

# Create database directories if they don't exist
if (-not (Test-Path $Site1112DbDir)) {
    New-Item -ItemType Directory -Force -Path $Site1112DbDir | Out-Null
    Write-Host "Created Site 1112 database directory" -ForegroundColor Green
}

if (-not (Test-Path $Site7000DbDir)) {
    New-Item -ItemType Directory -Force -Path $Site7000DbDir | Out-Null
    Write-Host "Created Site 7000 database directory" -ForegroundColor Green
}

# Function to get random database files
function Get-RandomDbFiles {
    param([int]$Count = 1)

    # Find all database directories (ams1112_0001, ams7000_0001, etc.)
    $dbDirs = Get-ChildItem -Path $SourceDir -Directory | Where-Object { $_.Name -match "^ams\d+_\d+$" }

    if ($dbDirs.Count -eq 0) {
        Write-Host "Warning: No database directories found in $SourceDir" -ForegroundColor Yellow
        return @()
    }

    # Randomly select database directories
    $selectedDirs = $dbDirs | Get-Random -Count ([Math]::Min($Count, $dbDirs.Count))

    return $selectedDirs
}

# Function to copy database directory to a site
function Copy-DbToSite {
    param(
        [string]$SourceDbPath,
        [string]$TargetBasePath,
        [string]$SiteName
    )

    $dbName = Split-Path -Leaf $SourceDbPath
    $targetPath = Join-Path $TargetBasePath $dbName

    Write-Host "[$SiteName] Copying database: $dbName" -ForegroundColor Yellow

    try {
        # Create target directory if it doesn't exist
        if (-not (Test-Path $targetPath)) {
            New-Item -ItemType Directory -Force -Path $targetPath | Out-Null
        }

        # Get all files in source directory
        $files = Get-ChildItem -Path $SourceDbPath -File

        if ($files.Count -eq 0) {
            Write-Host "[$SiteName] Warning: No files found in $dbName" -ForegroundColor Yellow
            return 0
        }

        # Copy files
        $copiedCount = 0
        foreach ($file in $files) {
            $destFile = Join-Path $targetPath $file.Name
            Copy-Item -Path $file.FullName -Destination $destFile -Force
            $copiedCount++
        }

        # Update modification time to trigger file watcher
        $touchFile = Join-Path $targetPath ".sync_trigger"
        $timestamp = Get-Date
        Set-Content -Path $touchFile -Value $timestamp
        (Get-Item $touchFile).LastWriteTime = $timestamp

        Write-Host "[$SiteName] Copied $copiedCount files from $dbName" -ForegroundColor Green

        # Log the sync operation
        $logEntry = @{
            Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
            Site = $SiteName
            Database = $dbName
            FilesCount = $copiedCount
            SourcePath = $SourceDbPath
            TargetPath = $targetPath
        } | ConvertTo-Json -Compress

        Add-Content -Path (Join-Path $LogsDir "sync-simulator.log") -Value $logEntry

        return $copiedCount

    } catch {
        Write-Host "[$SiteName] Error copying $dbName : $_" -ForegroundColor Red
        return 0
    }
}

# Function to perform one sync operation
function Invoke-SyncOperation {
    param([string]$Direction)

    $dbDirs = Get-RandomDbFiles -Count $FilesPerSync

    if ($dbDirs.Count -eq 0) {
        Write-Host "No database directories to sync" -ForegroundColor Yellow
        return 0
    }

    $totalCopied = 0

    foreach ($dbDir in $dbDirs) {
        $actualDirection = $Direction

        if ($Direction -eq "random") {
            $actualDirection = @("1112", "7000", "both") | Get-Random
        }

        switch ($actualDirection) {
            "1112" {
                $copied = Copy-DbToSite -SourceDbPath $dbDir.FullName -TargetBasePath $Site1112DbDir -SiteName "Site-1112"
                $totalCopied += $copied
            }
            "7000" {
                $copied = Copy-DbToSite -SourceDbPath $dbDir.FullName -TargetBasePath $Site7000DbDir -SiteName "Site-7000"
                $totalCopied += $copied
            }
            "both" {
                $copied1 = Copy-DbToSite -SourceDbPath $dbDir.FullName -TargetBasePath $Site1112DbDir -SiteName "Site-1112"
                $copied2 = Copy-DbToSite -SourceDbPath $dbDir.FullName -TargetBasePath $Site7000DbDir -SiteName "Site-7000"
                $totalCopied += ($copied1 + $copied2)
            }
        }
    }

    return $totalCopied
}

# Main simulation loop
Write-Host "========================================" -ForegroundColor Green
Write-Host "Starting File Sync Simulation" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""

if ($Duration -gt 0) {
    Write-Host "Will run for $Duration seconds" -ForegroundColor Cyan
} else {
    Write-Host "Running continuously (press Ctrl+C to stop)" -ForegroundColor Cyan
}
Write-Host ""

$startTime = Get-Date
$syncCount = 0
$totalFilesCopied = 0

try {
    while ($true) {
        $syncCount++
        $elapsed = ((Get-Date) - $startTime).TotalSeconds

        Write-Host "----------------------------------------" -ForegroundColor Cyan
        Write-Host "Sync Operation #$syncCount (Elapsed: $([int]$elapsed)s)" -ForegroundColor Cyan
        Write-Host "----------------------------------------" -ForegroundColor Cyan

        $filesCopied = Invoke-SyncOperation -Direction $Direction
        $totalFilesCopied += $filesCopied

        Write-Host ""
        Write-Host "Sync #$syncCount complete: $filesCopied files copied" -ForegroundColor Green
        Write-Host "Total files copied so far: $totalFilesCopied" -ForegroundColor Gray
        Write-Host ""

        # Check if we should stop
        if ($Duration -gt 0 -and $elapsed -ge $Duration) {
            Write-Host "Duration limit reached ($Duration seconds)" -ForegroundColor Yellow
            break
        }

        if (-not $Continuous -and $Duration -eq 0 -and $syncCount -ge 1) {
            Write-Host "Single sync operation completed. Use -Continuous for continuous operation." -ForegroundColor Yellow
            break
        }

        # Wait for next sync
        Write-Host "Waiting $IntervalSeconds seconds until next sync..." -ForegroundColor Gray
        Write-Host "(Press Ctrl+C to stop)" -ForegroundColor Gray
        Start-Sleep -Seconds $IntervalSeconds
    }
} catch {
    if ($_.Exception.Message -match "terminate") {
        Write-Host ""
        Write-Host "Simulation stopped by user" -ForegroundColor Yellow
    } else {
        Write-Host ""
        Write-Host "Error: $_" -ForegroundColor Red
    }
}

# Summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Simulation Summary" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Sync Operations:   $syncCount" -ForegroundColor Cyan
Write-Host "  Total Files Copied: $totalFilesCopied" -ForegroundColor Cyan
Write-Host "  Duration:          $([int]((Get-Date) - $startTime).TotalSeconds) seconds" -ForegroundColor Cyan
Write-Host ""

Write-Host "Check sync results:" -ForegroundColor Yellow
Write-Host "  Site 1112: $Site1112DbDir" -ForegroundColor Gray
Write-Host "  Site 7000: $Site7000DbDir" -ForegroundColor Gray
Write-Host "  Logs:      $LogsDir\sync-simulator.log" -ForegroundColor Gray
Write-Host ""
