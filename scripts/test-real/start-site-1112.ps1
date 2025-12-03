# 启动站点 1112 Web 服务器
# HTTP 端口: 8081

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "启动站点 1112 Web 服务器" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 获取项目根目录
$ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
$TestRealDir = Join-Path $ProjectRoot "remote-test-dir" "test-real"
$Site1112Dir = Join-Path $TestRealDir "site-1112"

Write-Host "项目根目录: $ProjectRoot" -ForegroundColor Green
Write-Host "站点目录: $Site1112Dir" -ForegroundColor Green
Write-Host ""

# 检查配置文件
$ConfigPath = Join-Path $Site1112Dir "DbOption.toml"
if (-not (Test-Path $ConfigPath)) {
    Write-Host "错误: 配置文件不存在: $ConfigPath" -ForegroundColor Red
    Write-Host "请先运行初始化脚本: .\scripts\test-real\init_test_env.ps1" -ForegroundColor Yellow
    exit 1
}

Write-Host "配置文件: $ConfigPath" -ForegroundColor Green
Write-Host ""

# 切换到站点目录
Set-Location $Site1112Dir

Write-Host "当前工作目录: $(Get-Location)" -ForegroundColor Green
Write-Host ""

# 显示启动信息
Write-Host "启动站点 1112 Web 服务器..." -ForegroundColor Yellow
Write-Host "  HTTP 端口: 8081" -ForegroundColor Gray
Write-Host "  文件接收端点: http://127.0.0.1:8081/files" -ForegroundColor Gray
Write-Host "  文件服务端点: http://127.0.0.1:8081/assets/archives" -ForegroundColor Gray
Write-Host "  Location: SITE1112" -ForegroundColor Gray
Write-Host "  Database: 1112" -ForegroundColor Gray
Write-Host "  SurrealDB: 127.0.0.1:8021" -ForegroundColor Gray
Write-Host "  MQTT: 127.0.0.1:1883" -ForegroundColor Gray
Write-Host "  按 Ctrl+C 停止服务器" -ForegroundColor Gray
Write-Host ""

# 启动 web_server
# 注意：需要在项目根目录编译 web_server
Write-Host "执行命令: cargo run --bin web_server --features web_server -- --port 8081" -ForegroundColor Gray
Write-Host ""

Set-Location $ProjectRoot
cargo run --bin web_server --features web_server -- --port 8081
