# 测试异地同步功能
Write-Host "测试异地同步功能..." -ForegroundColor Cyan

$receivedDir = "D:\work\plant\web-server\test-remote-site\received_cba"

Write-Host "`n等待接收CBA文件..."
Write-Host "接收目录: $receivedDir" -ForegroundColor Yellow

$timeout = 60
$elapsed = 0

while ($elapsed -lt $timeout) {
    $files = Get-ChildItem -Path $receivedDir -Filter "*.cba" -ErrorAction SilentlyContinue
    if ($files.Count -gt 0) {
        Write-Host "`n[OK] 成功接收 $($files.Count) 个CBA文件:" -ForegroundColor Green
        foreach ($file in $files) {
            $fileSize = $file.Length
            $fileTime = $file.LastWriteTime
            Write-Host "  - $($file.Name) ($fileSize bytes, $fileTime)" -ForegroundColor Gray
        }
        exit 0
    }
    Start-Sleep -Seconds 1
    $elapsed++
    Write-Host "." -NoNewline
}

Write-Host "`n[ERROR] 超时: 未接收到CBA文件" -ForegroundColor Red
exit 1
