# 下载 CDN 资源到本地
# 用于替换所有 CDN 链接为本地文件

$ErrorActionPreference = "Stop"

$staticDir = "src/web_server/static"
$libsDir = Join-Path $staticDir "libs"

# 创建目录
if (-not (Test-Path $staticDir)) {
    New-Item -ItemType Directory -Path $staticDir -Force | Out-Null
}
if (-not (Test-Path $libsDir)) {
    New-Item -ItemType Directory -Path $libsDir -Force | Out-Null
}

Write-Host "==> 开始下载 CDN 资源..."

# 1. Font Awesome 6.4.0 CSS
Write-Host "下载 Font Awesome 6.4.0..."
$fontAwesomeUrl = "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"
$fontAwesomeDest = Join-Path $libsDir "font-awesome-6.4.0.min.css"
try {
    Invoke-WebRequest -Uri $fontAwesomeUrl -OutFile $fontAwesomeDest -UseBasicParsing
    Write-Host "  ✓ Font Awesome CSS 下载完成: $fontAwesomeDest"
    
    # 修复 CSS 中的 webfonts 路径，从 CDN 路径改为本地路径
    Write-Host "  修复 Font Awesome CSS 中的 webfonts 路径..."
    $cssContent = Get-Content $fontAwesomeDest -Raw -Encoding UTF8
    # 将 ../webfonts/ 替换为 /static/libs/webfonts/
    $cssContent = $cssContent -replace '\.\./webfonts/', '/static/libs/webfonts/'
    Set-Content -Path $fontAwesomeDest -Value $cssContent -Encoding UTF8 -NoNewline
    Write-Host "  ✓ Font Awesome CSS 路径修复完成"
} catch {
    Write-Error "下载 Font Awesome 失败: $_"
}

# Font Awesome 还需要下载 webfonts
Write-Host "下载 Font Awesome Webfonts..."
$webfontsDir = Join-Path $libsDir "webfonts"
if (-not (Test-Path $webfontsDir)) {
    New-Item -ItemType Directory -Path $webfontsDir -Force | Out-Null
}

# 下载主要的 webfonts 文件
$webfontsFiles = @(
    "fa-solid-900.woff2",
    "fa-regular-400.woff2",
    "fa-brands-400.woff2"
)

foreach ($file in $webfontsFiles) {
    $webfontUrl = "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/webfonts/$file"
    $webfontDest = Join-Path $webfontsDir $file
    try {
        Invoke-WebRequest -Uri $webfontUrl -OutFile $webfontDest -UseBasicParsing
        Write-Host "  ✓ $file 下载完成"
    } catch {
        Write-Warning "下载 $file 失败: $_"
    }
}

# 2. Chart.js 4.4.0 (如果还没有)
Write-Host "检查 Chart.js 4.4.0..."
$chartJsDest = Join-Path $staticDir "chart.umd.min.js"
if (-not (Test-Path $chartJsDest)) {
    $chartJsUrl = "https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"
    try {
        Invoke-WebRequest -Uri $chartJsUrl -OutFile $chartJsDest -UseBasicParsing
        Write-Host "  ✓ Chart.js 下载完成: $chartJsDest"
    } catch {
        Write-Error "下载 Chart.js 失败: $_"
    }
} else {
    Write-Host "  ✓ Chart.js 已存在，跳过下载"
}

# 3. ECharts 5.4.3
Write-Host "下载 ECharts 5.4.3..."
$echartsUrl = "https://cdn.jsdelivr.net/npm/echarts@5.4.3/dist/echarts.min.js"
$echartsDest = Join-Path $libsDir "echarts-5.4.3.min.js"
try {
    Invoke-WebRequest -Uri $echartsUrl -OutFile $echartsDest -UseBasicParsing
    Write-Host "  ✓ ECharts 下载完成: $echartsDest"
} catch {
    Write-Error "下载 ECharts 失败: $_"
}

# 4. DaisyUI 4.6.0 (可选，部分模板使用)
Write-Host "下载 DaisyUI 4.6.0..."
$daisyUiUrl = "https://cdn.jsdelivr.net/npm/daisyui@4.6.0/dist/full.min.css"
$daisyUiDest = Join-Path $libsDir "daisyui-4.6.0.min.css"
try {
    Invoke-WebRequest -Uri $daisyUiUrl -OutFile $daisyUiDest -UseBasicParsing
    Write-Host "  ✓ DaisyUI 下载完成: $daisyUiDest"
} catch {
    Write-Warning "下载 DaisyUI 失败: $_ (可选资源)"
}

Write-Host "`n==> CDN 资源下载完成！"
Write-Host "资源位置: $libsDir"
Write-Host "`n下一步："
Write-Host "1. 更新模板文件中的 CDN 链接为本地路径"
Write-Host "2. 运行 update_sites.ps1 将资源复制到各站点"

