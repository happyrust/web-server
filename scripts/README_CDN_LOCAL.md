# CDN 资源本地化说明

本项目已将所有 CDN 资源替换为本地文件，以提高加载速度和离线可用性。

## 📦 已本地化的资源

1. **Font Awesome 6.4.0** - 图标库
   - CSS: `/static/libs/font-awesome-6.4.0.min.css`
   - Webfonts: `/static/libs/webfonts/`

2. **Chart.js 4.4.0** - 图表库
   - JS: `/static/chart.umd.min.js`

3. **ECharts 5.4.3** - 图表库
   - JS: `/static/libs/echarts-5.4.3.min.js`

4. **DaisyUI 4.6.0** - UI 框架（部分模板使用）
   - CSS: `/static/libs/daisyui-4.6.0.min.css`

## 🚀 使用步骤

### 第一步：下载 CDN 资源

首次使用前，需要先下载所有 CDN 资源到本地：

```powershell
.\scripts\download_cdn_resources.ps1
```

这个脚本会：
- 下载所有 CDN 资源到 `src/web_server/static/libs/`
- 自动修复 Font Awesome CSS 中的 webfonts 路径
- 下载必要的字体文件

### 第二步：构建和部署

运行 `update_sites.ps1` 时，CDN 资源会自动复制到各站点：

```powershell
.\scripts\update_sites.ps1
```

或者跳过前端构建但保留 CDN 资源拷贝：

```powershell
.\scripts\update_sites.ps1 -SkipFrontendBuild
```

如果只想跳过 CDN 资源拷贝：

```powershell
.\scripts\update_sites.ps1 -SkipCdnResources
```

## 📁 目录结构

```
src/web_server/static/
├── chart.umd.min.js          # Chart.js（根目录）
└── libs/                      # CDN 资源目录
    ├── font-awesome-6.4.0.min.css
    ├── echarts-5.4.3.min.js
    ├── daisyui-4.6.0.min.css
    └── webfonts/              # Font Awesome 字体文件
        ├── fa-solid-900.woff2
        ├── fa-regular-400.woff2
        └── fa-brands-400.woff2

站点目录结构（部署后）:
site-*/bin/static/
├── chart.umd.min.js
└── libs/
    └── ... (同上)
```

## 🔄 更新资源

如果需要更新 CDN 资源版本：

1. 删除旧的资源文件
2. 运行 `download_cdn_resources.ps1` 重新下载
3. 更新模板文件中的版本号（如需要）
4. 运行 `update_sites.ps1` 重新部署

## ✅ 已更新的模板文件

以下模板文件已更新为使用本地资源：

- `src/web_server/templates/incremental_update_vue.html`
- `src/web_server/templates/incremental_update.html`
- `src/web_server/templates/incremental_update_original.html`
- `src/web_server/templates/CameraControl_firstPerson_Duplex.html`
- `src/web_server/dashboard_template.rs`
- `src/web_server/templates.rs`
- `frontend/index.html`

## 🐛 故障排查

### 问题 1：图标不显示

**原因**：Font Awesome webfonts 路径不正确

**解决**：
1. 检查 `src/web_server/static/libs/font-awesome-6.4.0.min.css` 中的路径是否为 `/static/libs/webfonts/`
2. 确认 webfonts 文件已下载到 `src/web_server/static/libs/webfonts/`
3. 重新运行 `download_cdn_resources.ps1`

### 问题 2：图表不显示

**原因**：Chart.js 或 ECharts 文件缺失

**解决**：
1. 检查 `src/web_server/static/chart.umd.min.js` 是否存在
2. 检查 `src/web_server/static/libs/echarts-5.4.3.min.js` 是否存在
3. 重新运行 `download_cdn_resources.ps1`

### 问题 3：站点部署后资源 404

**原因**：CDN 资源未复制到站点目录

**解决**：
1. 确认已运行 `download_cdn_resources.ps1`
2. 运行 `update_sites.ps1` 时不要使用 `-SkipCdnResources` 参数
3. 检查站点目录 `site-*/bin/static/libs/` 是否存在

## 📝 注意事项

1. **首次使用必须下载**：首次使用前必须运行 `download_cdn_resources.ps1`
2. **版本一致性**：确保所有模板文件使用的版本号与下载的版本一致
3. **网络要求**：下载脚本需要网络连接，建议在网络良好的环境下运行
4. **文件大小**：Font Awesome webfonts 文件较大（约 1MB），下载可能需要一些时间

