# 启动 SurrealDB 服务器（站点 7000）
# 端口: 8022

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "启动 SurrealDB 服务器 - 站点 7000" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 获取项目根目录
$ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
$TestRealDir = Join-Path $ProjectRoot "remote-test-dir" "test-real"
$Site7000Dir = Join-Path $TestRealDir "site-7000"
$DataDir = Join-Path $Site7000Dir "surrealdb-data"

# 创建数据目录
if (-not (Test-Path $DataDir)) {
    Write-Host "创建 SurrealDB 数据目录: $DataDir" -ForegroundColor Green
    New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
}

Write-Host "站点目录: $Site7000Dir" -ForegroundColor Green
Write-Host "数据目录: $DataDir" -ForegroundColor Green
Write-Host ""

# 检查 surreal 命令
$SurrealCmd = Get-Command surreal -ErrorAction SilentlyContinue
if (-not $SurrealCmd) {
    Write-Host "错误: 找不到 surreal 命令" -ForegroundColor Red
    Write-Host ""
    Write-Host "请先安装 SurrealDB:" -ForegroundColor Yellow
    Write-Host "  选项 1: 使用 PowerShell 安装（Windows）:" -ForegroundColor Gray
    Write-Host "    iwr https://windows.surrealdb.com -useb | iex" -ForegroundColor Gray
    Write-Host ""
    Write-Host "  选项 2: 使用 Cargo 安装:" -ForegroundColor Gray
    Write-Host "    cargo install --locked surrealdb" -ForegroundColor Gray
    Write-Host ""
    Write-Host "  选项 3: 从官网下载:" -ForegroundColor Gray
    Write-Host "    https://surrealdb.com/install" -ForegroundColor Gray
    Write-Host ""
    exit 1
}

Write-Host "使用 SurrealDB: $($SurrealCmd.Source)" -ForegroundColor Green
Write-Host ""

# 启动 SurrealDB
Write-Host "启动 SurrealDB 服务器..." -ForegroundColor Yellow
Write-Host "  命名空间: 1516" -ForegroundColor Gray
Write-Host "  数据库: ams" -ForegroundColor Gray
Write-Host "  端口: 8022" -ForegroundColor Gray
Write-Host "  用户: root / root" -ForegroundColor Gray
Write-Host "  按 Ctrl+C 停止服务器" -ForegroundColor Gray
Write-Host ""

# 切换到数据目录
Set-Location $DataDir

# 启动 SurrealDB（使用文件存储）
surreal start --log trace --user root --pass root file://surrealdb --bind 127.0.0.1:8022
