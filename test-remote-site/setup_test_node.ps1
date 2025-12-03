# 测试远程站点配置脚本
# 用于模拟异地同步功能测试

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  远程站点模拟测试环境配置" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 1. 检查必要的文件和目录
Write-Host "[1/4] 检查目录结构..." -ForegroundColor Yellow
$testDir = "D:\work\plant\web-server\test-remote-site"
$dbFile = "D:\work\plant\web-server\deployment_sites.sqlite"

if (-not (Test-Path $testDir)) {
    Write-Host "  × 测试目录不存在: $testDir" -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $dbFile)) {
    Write-Host "  ! 警告: deployment_sites.sqlite 不存在，将在首次运行时创建" -ForegroundColor Yellow
} else {
    Write-Host "  √ 找到配置数据库: $dbFile" -ForegroundColor Green
}

# 2. 执行SQL配置
Write-Host "`n[2/4] 注册测试站点到数据库..." -ForegroundColor Yellow

# 检查是否有 sqlite3 命令
$sqliteCmd = Get-Command sqlite3 -ErrorAction SilentlyContinue
if ($null -eq $sqliteCmd) {
    Write-Host "  ! SQLite3 命令未找到，请手动执行 setup_test_site.sql" -ForegroundColor Yellow
    Write-Host "    或通过 Web UI 手动添加站点配置" -ForegroundColor Yellow
} else {
    try {
        & sqlite3 $dbFile ".read $testDir\setup_test_site.sql"
        Write-Host "  √ 测试站点配置已写入数据库" -ForegroundColor Green
    } catch {
        Write-Host "  × SQL 执行失败: $_" -ForegroundColor Red
    }
}

# 3. 显示配置摘要
Write-Host "`n[3/4] 测试站点配置摘要" -ForegroundColor Yellow
Write-Host "  主站点 (Beijing):" -ForegroundColor Cyan
Write-Host "    - Location: beijing" -ForegroundColor White
Write-Host "    - HTTP: http://localhost:8080" -ForegroundColor White
Write-Host "    - MQTT: localhost:1883" -ForegroundColor White
Write-Host ""
Write-Host "  测试站点 1 (Shanghai):" -ForegroundColor Cyan
Write-Host "    - ID: test-site-shanghai" -ForegroundColor White
Write-Host "    - Location: shanghai" -ForegroundColor White
Write-Host "    - HTTP: http://localhost:9090" -ForegroundColor White
Write-Host "    - 订阅数据库: 1112" -ForegroundColor White
Write-Host "    - 接收目录: $testDir\received_cba" -ForegroundColor White
Write-Host ""
Write-Host "  测试站点 2 (Shenzhen):" -ForegroundColor Cyan
Write-Host "    - ID: test-site-shenzhen" -ForegroundColor White
Write-Host "    - Location: shenzhen" -ForegroundColor White
Write-Host "    - HTTP: http://localhost:9091" -ForegroundColor White
Write-Host "    - 订阅数据库: 1112" -ForegroundColor White
Write-Host ""

# 4. 创建辅助脚本
Write-Host "[4/4] 创建测试辅助脚本..." -ForegroundColor Yellow

# 已经手动创建 mock_receiver.ps1 和 test_sync.ps1
if (Test-Path "$testDir\mock_receiver.ps1") {
    Write-Host "  √ mock_receiver.ps1 已存在" -ForegroundColor Green
} else {
    Write-Host "  ! mock_receiver.ps1 不存在，需要手动创建" -ForegroundColor Yellow
}

if (Test-Path "$testDir\test_sync.ps1") {
    Write-Host "  √ test_sync.ps1 已存在" -ForegroundColor Green
} else {
    Write-Host "  ! test_sync.ps1 不存在，需要手动创建" -ForegroundColor Yellow
}

# 完成
Write-Host "`n========================================" -ForegroundColor Green
Write-Host "  配置完成！" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "接下来的步骤:" -ForegroundColor Yellow
Write-Host "  1. 启动 Web 服务器 (如果尚未启动)" -ForegroundColor White
Write-Host "       cargo run --bin web_server --features web_server" -ForegroundColor Gray
Write-Host ""
Write-Host "  2. 在新的终端运行模拟接收器:" -ForegroundColor White
Write-Host "       cd test-remote-site" -ForegroundColor Gray
Write-Host "       .\mock_receiver.ps1" -ForegroundColor Gray
Write-Host ""
Write-Host "  3. 触发增量更新 (修改PDMS文件或手动触发)" -ForegroundColor White
Write-Host ""
Write-Host "  4. 验证同步结果:" -ForegroundColor White
Write-Host "       .\test_sync.ps1" -ForegroundColor Gray
Write-Host ""
Write-Host "  5. 在 Web UI 查看同步状态:" -ForegroundColor White
Write-Host "       http://localhost:8080" -ForegroundColor Gray
Write-Host ""
