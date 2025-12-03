# 一键构建→停站→替换二进制→重启三套站点
param(
    [switch]$SkipBuild = $false,
    [switch]$SkipFrontendBuild = $false,
    [switch]$Release = $false,
    [string]$BinaryPath = "",
    [switch]$StartSites = $false,  # 默认不启动站点，需要显式指定 -StartSites 才会启动
    [switch]$SkipCdnResources = $false  # 跳过 CDN 资源拷贝
)

$ErrorActionPreference = "Stop"

function Build-Binary {
    param([bool]$IsRelease = $false)
    $buildType = if ($IsRelease) { "release" } else { "debug" }
    Write-Host "==> 开始编译 web_server ($buildType) ..."
    if ($IsRelease) {
        cargo build --release --bin web_server --features web_server | Write-Host
    } else {
        cargo build --bin web_server --features web_server | Write-Host
    }
}

function Build-Frontend {
    Write-Host "==> 开始编译 frontend ..."
    $frontendDir = "frontend"
    if (-not (Test-Path $frontendDir)) {
        throw "frontend 目录不存在: $frontendDir"
    }
    
    Push-Location $frontendDir
    try {
        # 检查 node_modules 是否存在
        if (-not (Test-Path "node_modules")) {
            Write-Host "检测到缺少 node_modules，正在安装依赖..."
            npm install
        }
        
        Write-Host "执行 npm run build ..."
        npm run build
        
        $distDir = "dist"
        if (-not (Test-Path $distDir)) {
            throw "构建失败：未找到 dist 目录"
        }
        
        $jsFile = Join-Path $distDir "incremental_update.js"
        $cssFile = Join-Path $distDir "incremental_update.css"
        
        if (-not (Test-Path $jsFile)) {
            throw "构建失败：未找到 $jsFile"
        }
        if (-not (Test-Path $cssFile)) {
            throw "构建失败：未找到 $cssFile"
        }
        
        Write-Host "前端构建完成"
    } finally {
        Pop-Location
    }
}

function Resolve-Binary {
    param([string]$PathOverride, [bool]$IsRelease = $false)
    if (-not [string]::IsNullOrWhiteSpace($PathOverride)) {
        if (-not (Test-Path $PathOverride)) {
            throw "指定的 BinaryPath 不存在: $PathOverride"
        }
        return (Resolve-Path $PathOverride).Path
    }
    $buildType = if ($IsRelease) { "release" } else { "debug" }
    $defaultPath = "target/$buildType/web_server.exe"
    if (-not (Test-Path $defaultPath)) {
        throw "未找到可执行文件: $defaultPath (可用 -BinaryPath 指定路径，或取消 -SkipBuild)"
    }
    return (Resolve-Path $defaultPath).Path
}

function Stop-Sites {
    param([string[]]$Sites)
    foreach ($s in $Sites) {
        $stop = Join-Path $s "scripts/stop.ps1"
        if (Test-Path $stop) {
            Write-Host "==> 停止 $s ..."
            & $stop -KillAll:$true
        } else {
            Write-Warning "未找到停止脚本: $stop"
        }
    }
}

function Copy-ToSites {
    param([string]$Binary, [string[]]$Sites)
    foreach ($s in $Sites) {
        $destDir = Join-Path $s "bin"
        if (-not (Test-Path $destDir)) {
            throw "目标目录不存在: $destDir"
        }
        $destFile = Join-Path $destDir "web_server.exe"
        Copy-Item $Binary -Destination $destFile -Force
        Write-Host ("已更新 {0}" -f $destFile)
    }
}

function Copy-FrontendToSites {
    param([string[]]$Sites)
    $frontendDist = "frontend/dist"
    if (-not (Test-Path $frontendDist)) {
        throw "前端构建目录不存在: $frontendDist (请先运行前端构建)"
    }
    
    $jsFile = Join-Path $frontendDist "incremental_update.js"
    $cssFile = Join-Path $frontendDist "incremental_update.css"
    
    if (-not (Test-Path $jsFile)) {
        throw "前端 JS 文件不存在: $jsFile"
    }
    if (-not (Test-Path $cssFile)) {
        throw "前端 CSS 文件不存在: $cssFile"
    }
    
    # 首先拷贝到项目根目录的 src/web_server/static/（开发环境使用）
    $projectStaticDir = "src/web_server/static"
    if (-not (Test-Path $projectStaticDir)) {
        Write-Host "创建项目静态文件目录: $projectStaticDir"
        New-Item -ItemType Directory -Path $projectStaticDir -Force | Out-Null
    }
    
    $projectJs = Join-Path $projectStaticDir "incremental_update.js"
    $projectCss = Join-Path $projectStaticDir "incremental_update.css"
    
    Copy-Item $jsFile -Destination $projectJs -Force
    Copy-Item $cssFile -Destination $projectCss -Force
    Write-Host ("已更新项目静态文件: {0}" -f $projectStaticDir)
    
    # 拷贝到各站点目录的 bin/static/（便携式部署，与可执行文件同目录）
    foreach ($s in $Sites) {
        $binStaticDir = Join-Path $s "bin/static"
        if (-not (Test-Path $binStaticDir)) {
            Write-Host "创建站点 bin/static 目录: $binStaticDir"
            New-Item -ItemType Directory -Path $binStaticDir -Force | Out-Null
        }
        
        $destJs = Join-Path $binStaticDir "incremental_update.js"
        $destCss = Join-Path $binStaticDir "incremental_update.css"
        
        Copy-Item $jsFile -Destination $destJs -Force
        Copy-Item $cssFile -Destination $destCss -Force
        
        Write-Host ("已更新站点前端文件到: {0}" -f $binStaticDir)
    }
}

function Copy-TemplatesToSites {
    param([string[]]$Sites)
    $templatesDir = "src/web_server/templates"
    if (-not (Test-Path $templatesDir)) {
        Write-Warning "模板目录不存在: $templatesDir，跳过模板文件拷贝"
        return
    }
    
    # 拷贝到各站点目录的 bin/templates/（便携式部署）
    foreach ($s in $Sites) {
        $binTemplatesDir = Join-Path $s "bin/templates"
        if (-not (Test-Path $binTemplatesDir)) {
            Write-Host "创建站点 bin/templates 目录: $binTemplatesDir"
            New-Item -ItemType Directory -Path $binTemplatesDir -Force | Out-Null
        }
        
        # 拷贝所有 HTML 模板文件
        $templateFiles = Get-ChildItem -Path $templatesDir -Filter "*.html" -File
        foreach ($template in $templateFiles) {
            $destFile = Join-Path $binTemplatesDir $template.Name
            Copy-Item $template.FullName -Destination $destFile -Force
        }
        
        Write-Host ("已更新站点模板文件到: {0} ({1} 个文件)" -f $binTemplatesDir, $templateFiles.Count)
    }
}

function Copy-CdnResourcesToSites {
    param([string[]]$Sites)
    $staticDir = "src/web_server/static"
    $libsDir = Join-Path $staticDir "libs"
    
    if (-not (Test-Path $libsDir)) {
        Write-Warning "CDN 资源目录不存在: $libsDir，跳过 CDN 资源拷贝"
        Write-Host "提示: 请先运行 scripts/download_cdn_resources.ps1 下载资源"
        return
    }
    
    # 首先拷贝到项目根目录的 src/web_server/static/libs/（开发环境使用）
    Write-Host "CDN 资源已存在于项目目录: $libsDir"
    
    # 拷贝到各站点目录的 bin/static/libs/（便携式部署）
    foreach ($s in $Sites) {
        $binStaticDir = Join-Path $s "bin/static"
        $binLibsDir = Join-Path $binStaticDir "libs"
        
        if (-not (Test-Path $binStaticDir)) {
            Write-Host "创建站点 bin/static 目录: $binStaticDir"
            New-Item -ItemType Directory -Path $binStaticDir -Force | Out-Null
        }
        
        if (-not (Test-Path $binLibsDir)) {
            Write-Host "创建站点 bin/static/libs 目录: $binLibsDir"
            New-Item -ItemType Directory -Path $binLibsDir -Force | Out-Null
        }
        
        # 拷贝所有 libs 目录下的文件
        if (Test-Path $libsDir) {
            $libFiles = Get-ChildItem -Path $libsDir -Recurse -File
            foreach ($file in $libFiles) {
                $relativePath = $file.FullName.Substring($libsDir.Length + 1)
                $destFile = Join-Path $binLibsDir $relativePath
                $destDir = Split-Path $destFile -Parent
                
                if (-not (Test-Path $destDir)) {
                    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
                }
                
                Copy-Item $file.FullName -Destination $destFile -Force
            }
            
            Write-Host ("已更新站点 CDN 资源到: {0} ({1} 个文件)" -f $binLibsDir, $libFiles.Count)
        }
        
        # 同时拷贝 chart.umd.min.js（如果存在）
        $chartJs = Join-Path $staticDir "chart.umd.min.js"
        if (Test-Path $chartJs) {
            $destChartJs = Join-Path $binStaticDir "chart.umd.min.js"
            Copy-Item $chartJs -Destination $destChartJs -Force
            Write-Host ("已更新 Chart.js 到: {0}" -f $destChartJs)
        }
    }
}

function Start-Sites {
    param([string[]]$Sites)
    foreach ($s in $Sites) {
        $start = Join-Path $s "scripts/start_all.ps1"
        if (Test-Path $start) {
            Write-Host "==> 启动 $s ..."
            & $start
        } else {
            Write-Warning "未找到启动脚本: $start"
        }
    }
}

try {
    $sites = @("site-main", "site-custom", "site-marine")

    Write-Host "==> 停止所有站点..."
    Stop-Sites -Sites $sites

    # 编译前端
    if (-not $SkipFrontendBuild) {
        Build-Frontend
        # 拷贝前端文件到各站点
        Copy-FrontendToSites -Sites $sites
        # 拷贝模板文件到各站点
        Copy-TemplatesToSites -Sites $sites
        # 拷贝 CDN 资源到各站点（除非跳过）
        if (-not $SkipCdnResources) {
            Copy-CdnResourcesToSites -Sites $sites
        } else {
            Write-Host "跳过 CDN 资源拷贝"
        }
    } else {
        Write-Host "跳过前端编译和拷贝"
    }

    # 编译后端
    if (-not $SkipBuild) {
        Build-Binary -IsRelease $Release
        Write-Host "编译完成，构建类型: $(if ($Release) { 'release' } else { 'debug' })"
    } else {
        Write-Host "跳过后端编译（使用现有或自定义 BinaryPath）"
    }
    $binPath = Resolve-Binary -PathOverride $BinaryPath -IsRelease $Release
    Write-Host "使用可执行文件: $binPath"
    Write-Host "构建类型: $(if ($Release) { 'release' } else { 'debug' })"

    # 拷贝后端二进制到各站点
    Copy-ToSites -Binary $binPath -Sites $sites

    # 只有在指定了 -StartSites 选项时才启动站点
    if ($StartSites) {
        Write-Host "==> 重启所有站点..."
        Start-Sites -Sites $sites
        Write-Host "全部站点更新并重启完成。"
    } else {
        Write-Host "站点更新完成（未启动，使用 -StartSites 选项可启动站点）。"
    }
} catch {
    Write-Error $_
    exit 1
}
