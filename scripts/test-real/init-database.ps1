# 初始化数据库（解析 AVEVA 数据库文件）
# 用于异地协同测试环境

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("1112", "7000", "all")]
    [string]$Site = "all"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "初始化测试数据库" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 获取项目根目录
$ProjectRoot = Join-Path $PSScriptRoot ".." ".." | Resolve-Path
$TestRealDir = Join-Path $ProjectRoot "remote-test-dir" "test-real"

Write-Host "项目根目录: $ProjectRoot" -ForegroundColor Green
Write-Host "测试环境目录: $TestRealDir" -ForegroundColor Green
Write-Host ""

function Initialize-Site {
    param(
        [string]$SiteNum,
        [string]$SiteDir,
        [string]$Port
    )

    Write-Host "初始化站点 $SiteNum..." -ForegroundColor Yellow
    Write-Host "  站点目录: $SiteDir" -ForegroundColor Gray
    Write-Host "  SurrealDB 端口: $Port" -ForegroundColor Gray
    Write-Host ""

    # 检查配置文件
    $ConfigPath = Join-Path $SiteDir "DbOption.toml"
    if (-not (Test-Path $ConfigPath)) {
        Write-Host "  错误: 配置文件不存在: $ConfigPath" -ForegroundColor Red
        return $false
    }

    # 切换到站点目录
    $OriginalLocation = Get-Location
    Set-Location $SiteDir

    Write-Host "  当前工作目录: $(Get-Location)" -ForegroundColor Gray
    Write-Host "  执行数据库初始化..." -ForegroundColor Gray
    Write-Host ""

    # 运行数据库初始化
    # 注意：这里需要使用实际的数据库初始化工具
    # 根据项目实际情况，可能是 init_test_db 或其他工具
    try {
        # 选项 1: 使用 init_test_db 工具
        $InitDbExe = Join-Path $ProjectRoot "target" "release" "init_test_db.exe"
        if (Test-Path $InitDbExe) {
            Write-Host "  使用 init_test_db 工具初始化..." -ForegroundColor Gray
            & $InitDbExe
        } else {
            # 选项 2: 使用 cargo run
            Write-Host "  使用 cargo run 初始化（开发模式）..." -ForegroundColor Gray
            cargo run --bin init_test_db
        }

        if ($LASTEXITCODE -ne 0) {
            Write-Host "  警告: 数据库初始化返回错误代码 $LASTEXITCODE" -ForegroundColor Yellow
        } else {
            Write-Host "  ✓ 站点 $SiteNum 数据库初始化完成" -ForegroundColor Green
        }
    } catch {
        Write-Host "  错误: 数据库初始化失败: $_" -ForegroundColor Red
    } finally {
        # 恢复原始目录
        Set-Location $OriginalLocation
    }

    Write-Host ""
    return $true
}

# 根据参数初始化站点
if ($Site -eq "1112" -or $Site -eq "all") {
    $Site1112Dir = Join-Path $TestRealDir "site-1112"
    Initialize-Site -SiteNum "1112" -SiteDir $Site1112Dir -Port "8021"
}

if ($Site -eq "7000" -or $Site -eq "all") {
    $Site7000Dir = Join-Path $TestRealDir "site-7000"
    Initialize-Site -SiteNum "7000" -SiteDir $Site7000Dir -Port "8022"
}

Write-Host "========================================" -ForegroundColor Green
Write-Host "数据库初始化完成！" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "提示:" -ForegroundColor Yellow
Write-Host "  - 确保 SurrealDB 服务已启动（端口 8021 和 8022）" -ForegroundColor Gray
Write-Host "  - 确保 AVEVA 项目文件存在于 D:\AVEVA\Projects\E3D2.1" -ForegroundColor Gray
Write-Host "  - 如果初始化失败，请检查配置文件和数据库文件路径" -ForegroundColor Gray
Write-Host ""
