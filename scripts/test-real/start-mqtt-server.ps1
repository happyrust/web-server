# 启动 MQTT 服务器（异地协同测试专用）
# 使用 rumqttd 作为本地 MQTT broker

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "启动 MQTT 服务器" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 获取项目根目录
$ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
$TestRealDir = Join-Path $ProjectRoot "remote-test-dir" "test-real"
$MqttConfigPath = Join-Path $TestRealDir "rumqttd.toml"

Write-Host "项目根目录: $ProjectRoot" -ForegroundColor Green
Write-Host "MQTT 配置文件: $MqttConfigPath" -ForegroundColor Green
Write-Host ""

# 检查配置文件
if (-not (Test-Path $MqttConfigPath)) {
    Write-Host "错误: MQTT 配置文件不存在: $MqttConfigPath" -ForegroundColor Red
    Write-Host "请先运行初始化脚本: .\scripts\test-real\init_test_env.ps1" -ForegroundColor Yellow
    exit 1
}

# 查找 rumqttd 可执行文件
$RumqttdExe = $null
$PossiblePaths = @(
    (Join-Path $ProjectRoot "rumqtt" "target" "release" "rumqttd.exe"),
    (Join-Path $ProjectRoot "rumqttd-server" "target" "release" "mqtt-server.exe"),
    "rumqttd.exe"  # 系统 PATH 中
)

foreach ($Path in $PossiblePaths) {
    if (Test-Path $Path) {
        $RumqttdExe = $Path
        break
    }
}

if (-not $RumqttdExe) {
    # 尝试使用 where 命令查找
    $WhereResult = where.exe rumqttd.exe 2>$null
    if ($LASTEXITCODE -eq 0 -and $WhereResult) {
        $RumqttdExe = $WhereResult[0]
    }
}

if (-not $RumqttdExe) {
    Write-Host "错误: 找不到 rumqttd 可执行文件" -ForegroundColor Red
    Write-Host ""
    Write-Host "请先编译 rumqttd:" -ForegroundColor Yellow
    Write-Host "  选项 1: 如果有 rumqtt submodule:" -ForegroundColor Gray
    Write-Host "    cd rumqtt" -ForegroundColor Gray
    Write-Host "    cargo build --release --bin rumqttd" -ForegroundColor Gray
    Write-Host ""
    Write-Host "  选项 2: 如果有 rumqttd-server:" -ForegroundColor Gray
    Write-Host "    cd rumqttd-server" -ForegroundColor Gray
    Write-Host "    cargo build --release --bin mqtt-server" -ForegroundColor Gray
    Write-Host ""
    Write-Host "  选项 3: 使用 cargo install:" -ForegroundColor Gray
    Write-Host "    cargo install rumqttd" -ForegroundColor Gray
    Write-Host ""
    exit 1
}

Write-Host "使用 rumqttd: $RumqttdExe" -ForegroundColor Green
Write-Host ""

# 切换到配置文件目录
Set-Location $TestRealDir

# 启动 MQTT 服务器
Write-Host "启动 MQTT 服务器..." -ForegroundColor Yellow
Write-Host "  MQTT 端口: 1883" -ForegroundColor Gray
Write-Host "  控制台端口: 18083" -ForegroundColor Gray
Write-Host "  按 Ctrl+C 停止服务器" -ForegroundColor Gray
Write-Host ""

& $RumqttdExe -c rumqttd.toml
