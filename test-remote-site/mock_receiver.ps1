# 模拟远程站点接收CBA文件的脚本
# 监听本地目录变化，模拟HTTP接收文件

param(
    [string]$SourceDir = "D:\work\plant\web-server\cba_files",
    [string]$DestDir = "D:\work\plant\web-server\test-remote-site\received_cba"
)

Write-Host "开始监听源目录: $SourceDir" -ForegroundColor Green
Write-Host "目标目录: $DestDir" -ForegroundColor Green

# 确保目标目录存在
if (-not (Test-Path $DestDir)) {
    New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
    Write-Host "已创建目标目录: $DestDir" -ForegroundColor Yellow
}

$watcher = New-Object System.IO.FileSystemWatcher
$watcher.Path = $SourceDir
$watcher.Filter = "*.cba"
$watcher.IncludeSubdirectories = $false
$watcher.EnableRaisingEvents = $true

$action = {
    $path = $Event.SourceEventArgs.FullPath
    $name = $Event.SourceEventArgs.Name
    $changeType = $Event.SourceEventArgs.ChangeType

    Write-Host "[$(Get-Date -Format 'HH:mm:ss')] 检测到文件变化: $name ($changeType)" -ForegroundColor Cyan

    if ($changeType -eq 'Created' -or $changeType -eq 'Changed') {
        Start-Sleep -Milliseconds 500
        try {
            $destPath = Join-Path $using:DestDir $name
            Copy-Item -Path $path -Destination $destPath -Force
            $fileSize = (Get-Item $path).Length
            Write-Host "  [OK] 文件已同步: $name" -ForegroundColor Green
            Write-Host "  大小: $fileSize bytes" -ForegroundColor Gray
        } catch {
            Write-Host "  [ERROR] 同步失败: $_" -ForegroundColor Red
        }
    }
}

Register-ObjectEvent -InputObject $watcher -EventName Created -Action $action | Out-Null
Register-ObjectEvent -InputObject $watcher -EventName Changed -Action $action | Out-Null

Write-Host "`n监听中... 按 Ctrl+C 停止" -ForegroundColor Yellow
Write-Host "================================================`n"

try {
    while ($true) {
        Start-Sleep -Seconds 1
    }
} finally {
    $watcher.Dispose()
}
