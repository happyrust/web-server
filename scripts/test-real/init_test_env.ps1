# 初始化异地协同测试环境（单机多节点模拟）
# 此脚本会创建两个站点的完整测试环境

param(
    [switch]$Force  # 强制重新初始化（会删除现有数据）
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "异地协同测试环境初始化脚本" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 获取项目根目录
$ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
$TestRealDir = Join-Path $ProjectRoot "remote-test-dir" "test-real"
$Site1112Dir = Join-Path $TestRealDir "site-1112"
$Site7000Dir = Join-Path $TestRealDir "site-7000"

Write-Host "项目根目录: $ProjectRoot" -ForegroundColor Green
Write-Host "测试环境目录: $TestRealDir" -ForegroundColor Green
Write-Host ""

# 1. 检查目录结构
Write-Host "[1/6] 检查目录结构..." -ForegroundColor Yellow
if (-not (Test-Path $Site1112Dir)) {
    Write-Host "  创建站点 1112 目录..." -ForegroundColor Gray
    New-Item -ItemType Directory -Force -Path "$Site1112Dir\assets\archives" | Out-Null
    New-Item -ItemType Directory -Force -Path "$Site1112Dir\output\remote_sync" | Out-Null
}
if (-not (Test-Path $Site7000Dir)) {
    Write-Host "  创建站点 7000 目录..." -ForegroundColor Gray
    New-Item -ItemType Directory -Force -Path "$Site7000Dir\assets\archives" | Out-Null
    New-Item -ItemType Directory -Force -Path "$Site7000Dir\output\remote_sync" | Out-Null
}
Write-Host "  ✓ 目录结构检查完成" -ForegroundColor Green
Write-Host ""

# 2. 初始化 SQLite 数据库（站点 1112）
Write-Host "[2/6] 初始化站点 1112 的 SQLite 数据库..." -ForegroundColor Yellow
$Db1112Path = Join-Path $Site1112Dir "deployment_sites.sqlite"
if ($Force -and (Test-Path $Db1112Path)) {
    Write-Host "  删除现有数据库..." -ForegroundColor Gray
    Remove-Item $Db1112Path -Force
}

if (-not (Test-Path $Db1112Path)) {
    Write-Host "  创建数据库表结构..." -ForegroundColor Gray

    sqlite3 $Db1112Path "CREATE TABLE IF NOT EXISTS deployment_environments (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, location TEXT NOT NULL, location_dbs TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP);"

    sqlite3 $Db1112Path "CREATE TABLE IF NOT EXISTS deployment_sites (id INTEGER PRIMARY KEY AUTOINCREMENT, environment_id INTEGER NOT NULL, name TEXT NOT NULL, location TEXT NOT NULL, http_host TEXT NOT NULL, dbnums TEXT NOT NULL, enabled INTEGER DEFAULT 1, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP, FOREIGN KEY (environment_id) REFERENCES deployment_environments(id));"

    sqlite3 $Db1112Path "INSERT INTO deployment_environments (name, location, location_dbs) VALUES ('Site1112-TestEnv', 'SITE1112', '1112');"

    sqlite3 $Db1112Path "INSERT INTO deployment_sites (environment_id, name, location, http_host, dbnums, enabled) VALUES (1, 'Site-7000', 'SITE7000', 'http://127.0.0.1:8082/files', '7000', 1);"

    Write-Host "  ✓ 站点 1112 数据库初始化完成" -ForegroundColor Green
} else {
    Write-Host "  数据库已存在，跳过初始化（使用 -Force 强制重新创建）" -ForegroundColor Gray
}
Write-Host ""

# 3. 初始化 SQLite 数据库（站点 7000）
Write-Host "[3/6] 初始化站点 7000 的 SQLite 数据库..." -ForegroundColor Yellow
$Db7000Path = Join-Path $Site7000Dir "deployment_sites.sqlite"
if ($Force -and (Test-Path $Db7000Path)) {
    Write-Host "  删除现有数据库..." -ForegroundColor Gray
    Remove-Item $Db7000Path -Force
}

if (-not (Test-Path $Db7000Path)) {
    Write-Host "  创建数据库表结构..." -ForegroundColor Gray

    sqlite3 $Db7000Path "CREATE TABLE IF NOT EXISTS deployment_environments (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, location TEXT NOT NULL, location_dbs TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP);"

    sqlite3 $Db7000Path "CREATE TABLE IF NOT EXISTS deployment_sites (id INTEGER PRIMARY KEY AUTOINCREMENT, environment_id INTEGER NOT NULL, name TEXT NOT NULL, location TEXT NOT NULL, http_host TEXT NOT NULL, dbnums TEXT NOT NULL, enabled INTEGER DEFAULT 1, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP, FOREIGN KEY (environment_id) REFERENCES deployment_environments(id));"

    sqlite3 $Db7000Path "INSERT INTO deployment_environments (name, location, location_dbs) VALUES ('Site7000-TestEnv', 'SITE7000', '7000');"

    sqlite3 $Db7000Path "INSERT INTO deployment_sites (environment_id, name, location, http_host, dbnums, enabled) VALUES (1, 'Site-1112', 'SITE1112', 'http://127.0.0.1:8081/files', '1112', 1);"

    Write-Host "  ✓ 站点 7000 数据库初始化完成" -ForegroundColor Green
} else {
    Write-Host "  数据库已存在，跳过初始化（使用 -Force 强制重新创建）" -ForegroundColor Gray
}
Write-Host ""

# 4. 检查 MQTT 服务器
Write-Host "[4/6] 检查 MQTT 服务器..." -ForegroundColor Yellow
$MqttConfigPath = Join-Path $TestRealDir "rumqttd.toml"
if (Test-Path $MqttConfigPath) {
    Write-Host "  ✓ MQTT 配置文件已存在: rumqttd.toml" -ForegroundColor Green
} else {
    Write-Host "  警告: MQTT 配置文件不存在，需要手动创建" -ForegroundColor Red
}
Write-Host ""

# 5. 拷贝 AVEVA 项目文件（可选）
Write-Host "[5/6] 检查 AVEVA 项目文件..." -ForegroundColor Yellow
$AvevaProjectPath = "D:\AVEVA\Projects\E3D2.1"
if (Test-Path $AvevaProjectPath) {
    Write-Host "  ✓ AVEVA 项目路径存在: $AvevaProjectPath" -ForegroundColor Green
} else {
    Write-Host "  警告: AVEVA 项目路径不存在，请手动配置" -ForegroundColor Yellow
}
Write-Host ""

# 6. 总结
Write-Host "[6/6] 初始化总结" -ForegroundColor Yellow
Write-Host "  站点 1112:" -ForegroundColor Cyan
Write-Host "    - 配置文件: $Site1112Dir\DbOption.toml" -ForegroundColor Gray
Write-Host "    - SQLite 数据库: $Db1112Path" -ForegroundColor Gray
Write-Host "    - HTTP 端口: 8081" -ForegroundColor Gray
Write-Host "    - SurrealDB 端口: 8021" -ForegroundColor Gray
Write-Host ""
Write-Host "  站点 7000:" -ForegroundColor Cyan
Write-Host "    - 配置文件: $Site7000Dir\DbOption.toml" -ForegroundColor Gray
Write-Host "    - SQLite 数据库: $Db7000Path" -ForegroundColor Gray
Write-Host "    - HTTP 端口: 8082" -ForegroundColor Gray
Write-Host "    - SurrealDB 端口: 8022" -ForegroundColor Gray
Write-Host ""
Write-Host "  MQTT 服务器:" -ForegroundColor Cyan
Write-Host "    - 配置文件: $MqttConfigPath" -ForegroundColor Gray
Write-Host "    - MQTT 端口: 1883" -ForegroundColor Gray
Write-Host "    - 控制台端口: 18083" -ForegroundColor Gray
Write-Host ""

Write-Host "========================================" -ForegroundColor Green
Write-Host "初始化完成！" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "下一步操作：" -ForegroundColor Yellow
Write-Host "  1. 启动 MQTT 服务器: .\scripts\test-real\start-mqtt-server.ps1" -ForegroundColor Gray
Write-Host "  2. 启动 SurrealDB（站点 1112）: .\scripts\test-real\start-surreal-1112.ps1" -ForegroundColor Gray
Write-Host "  3. 启动 SurrealDB（站点 7000）: .\scripts\test-real\start-surreal-7000.ps1" -ForegroundColor Gray
Write-Host "  4. 初始化数据库: .\scripts\test-real\init-database.ps1" -ForegroundColor Gray
Write-Host "  5. 启动站点 1112: .\scripts\test-real\start-site-1112.ps1" -ForegroundColor Gray
Write-Host "  6. 启动站点 7000: .\scripts\test-real\start-site-7000.ps1" -ForegroundColor Gray
Write-Host ""
