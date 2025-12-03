# 异地同步测试启动脚本
# 用于快速启动完整的测试环境

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  异地同步测试环境启动" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. 验证环境
Write-Host "[1/3] 验证测试环境..." -ForegroundColor Yellow

$cbaDir = "..\cba_files"
$receivedDir = "received_cba"

if (-not (Test-Path $cbaDir)) {
    Write-Host "  [WARN] cba_files目录不存在,正在创建..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Path $cbaDir -Force | Out-Null
    Write-Host "  [OK] cba_files目录已创建" -ForegroundColor Green
} else {
    Write-Host "  [OK] cba_files目录存在" -ForegroundColor Green
}

if (-not (Test-Path $receivedDir)) {
    Write-Host "  [WARN] received_cba目录不存在,正在创建..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Path $receivedDir -Force | Out-Null
    Write-Host "  [OK] received_cba目录已创建" -ForegroundColor Green
} else {
    Write-Host "  [OK] received_cba目录存在" -ForegroundColor Green
}

# 2. 检查测试站点注册
Write-Host "`n[2/3] 检查测试站点配置..." -ForegroundColor Yellow

if (Test-Path "..\deployment_sites.sqlite") {
    Write-Host "  [OK] deployment_sites.sqlite存在" -ForegroundColor Green
    Write-Host "  提示: 测试站点已注册 (test-site-shanghai, test-site-shenzhen)" -ForegroundColor Gray
} else {
    Write-Host "  [WARN] deployment_sites.sqlite不存在" -ForegroundColor Yellow
    Write-Host "  提示: 请先运行 Web 服务器创建数据库" -ForegroundColor Gray
}

# 3. 启动模拟接收器
Write-Host "`n[3/3] 启动模拟接收器..." -ForegroundColor Yellow
Write-Host "  监听目录: $cbaDir" -ForegroundColor Gray
Write-Host "  接收目录: $receivedDir" -ForegroundColor Gray
Write-Host ""

Write-Host "========================================" -ForegroundColor Green
Write-Host "  环境检查完成,开始监听文件..." -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "测试提示:" -ForegroundColor Cyan
Write-Host "  1. 在另一个终端运行 Web 服务器:" -ForegroundColor White
Write-Host "     cd .. && cargo run --bin web_server --features web_server" -ForegroundColor Gray
Write-Host ""
Write-Host "  2. 复制测试CBA文件到 cba_files/ 目录触发同步" -ForegroundColor White
Write-Host "     或等待系统自动生成增量更新" -ForegroundColor Gray
Write-Host ""
Write-Host "  3. 观察本窗口的文件同步日志" -ForegroundColor White
Write-Host ""
Write-Host "  4. 按 Ctrl+C 停止监听" -ForegroundColor White
Write-Host ""

# 启动mock_receiver
.\mock_receiver.ps1 -SourceDir $cbaDir -DestDir $receivedDir
