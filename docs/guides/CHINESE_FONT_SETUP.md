# 中文字体配置指南

## 概述

egui 异地协同运维界面使用中文界面，需要正确配置中文字体才能显示中文字符。本指南介绍如何配置和验证中文字体。

## 字体加载机制

应用按以下优先级自动加载字体：

1. **嵌入式字体** - 编译时嵌入到可执行文件（需启用 `embed_fonts` feature）
2. **本地字体文件** - 从 `assets/fonts/NotoSansSC-Regular.ttf` 加载
3. **系统字体** - 自动检测并加载系统中的中文字体

## 推荐方案：使用系统字体

### macOS

macOS 自带优秀的中文字体，无需额外配置：

- **PingFang SC** (苹方-简) - 默认使用
- **STHeiti** (华文黑体) - 备用
- **Arial Unicode MS** - 备用

**验证系统字体：**
```bash
ls /System/Library/Fonts/PingFang.ttc
```

### Windows

Windows 自带中文字体：

- **Microsoft YaHei** (微软雅黑) - 默认使用
- **SimHei** (黑体) - 备用
- **SimSun** (宋体) - 备用

**验证系统字体：**
```cmd
dir C:\Windows\Fonts\msyh.ttc
```

### Linux

大多数 Linux 发行版需要安装中文字体：

**Ubuntu/Debian:**
```bash
sudo apt-get install fonts-noto-cjk
# 或
sudo apt-get install fonts-wqy-microhei
```

**Fedora/RHEL:**
```bash
sudo dnf install google-noto-sans-cjk-fonts
# 或
sudo dnf install wqy-microhei-fonts
```

**Arch Linux:**
```bash
sudo pacman -S noto-fonts-cjk
# 或
sudo pacman -S wqy-microhei
```

**验证安装：**
```bash
fc-list :lang=zh
```

## 方案二：使用本地字体文件

如果系统字体不可用或希望使用特定字体：

### 自动下载（推荐）

```bash
# 在项目根目录执行
./assets/fonts/download_font.sh
```

脚本会自动从以下源下载 Noto Sans SC：
1. GitHub (googlefonts/noto-cjk)
2. Google Fonts API
3. jsDelivr CDN

### 手动下载

1. 访问 [Google Fonts - Noto Sans SC](https://fonts.google.com/noto/specimen/Noto+Sans+SC)
2. 点击 "Download family" 下载字体包
3. 解压后找到 `NotoSansSC-Regular.ttf`
4. 复制到项目的 `assets/fonts/` 目录

### 使用其他字体

您可以使用任何 TrueType 或 OpenType 中文字体：

1. 将字体文件复制到 `assets/fonts/`
2. 重命名为 `NotoSansSC-Regular.ttf`
3. 重新运行应用

**推荐字体：**
- **思源黑体** (Source Han Sans): https://github.com/adobe-fonts/source-han-sans
- **文泉驿微米黑**: http://wenq.org/wqy2/index.cgi?MicroHei
- **阿里巴巴普惠体**: https://www.alibabafonts.com/

## 方案三：嵌入字体到应用

将字体嵌入到可执行文件，无需外部字体文件（会增加约 6-8 MB）：

### 步骤

1. 确保 `assets/fonts/NotoSansSC-Regular.ttf` 存在
2. 在 `Cargo.toml` 中添加 feature：
   ```toml
   [features]
   embed_fonts = []
   gui = ["dep:eframe", "dep:egui", "dep:egui_extras", "dep:csv", "embed_fonts"]
   ```
3. 编译时启用 feature：
   ```bash
   cargo build --bin egui_remote_sync --features gui,embed_fonts --release
   ```

### 优缺点

**优点：**
- 无需外部字体文件
- 跨平台一致性
- 部署简单

**缺点：**
- 可执行文件增大 6-8 MB
- 编译时间增加
- 无法动态更换字体

## 验证字体加载

### 方法 1：查看日志

启用日志输出运行应用：

```bash
RUST_LOG=info cargo run --bin egui_remote_sync --features gui
```

成功加载字体会显示：
```
INFO egui_remote_sync: Loaded system Chinese font from: /System/Library/Fonts/PingFang.ttc
```

### 方法 2：检查界面

运行应用后检查：
- 导航栏中文是否正常显示
- 按钮文字是否清晰
- 表格内容是否正确显示

如果看到方块（□）或乱码，说明字体加载失败。

## 故障排除

### 问题 1：中文显示为方块

**原因：** 未找到可用的中文字体

**解决方案：**
1. 检查系统是否安装中文字体
2. 下载并放置本地字体文件
3. 查看应用日志确认字体加载状态

### 问题 2：字体模糊或锯齿

**原因：** 字体渲染设置或缩放问题

**解决方案：**
1. 在设置页面调整字体大小
2. 检查系统显示缩放设置
3. 尝试使用不同的字体

### 问题 3：部分中文字符显示异常

**原因：** 字体不包含某些生僻字

**解决方案：**
1. 使用更完整的字体（如 Noto Sans CJK）
2. 使用系统字体作为备用

### 问题 4：Linux 下无法加载字体

**原因：** 未安装中文字体包

**解决方案：**
```bash
# Ubuntu/Debian
sudo apt-get install fonts-noto-cjk

# Fedora/RHEL
sudo dnf install google-noto-sans-cjk-fonts

# Arch Linux
sudo pacman -S noto-fonts-cjk
```

### 问题 5：字体文件过大

**原因：** 完整的 CJK 字体包含大量字符

**解决方案：**
1. 使用字体子集化工具（fonttools）
2. 只保留常用汉字（GB2312 或 GBK）
3. 使用压缩格式

**字体子集化示例：**
```bash
# 安装 fonttools
pip install fonttools

# 创建子集（只保留常用汉字）
pyftsubset NotoSansSC-Regular.ttf \
    --unicodes="U+4E00-9FFF,U+3400-4DBF" \
    --output-file="NotoSansSC-Regular-Subset.ttf"
```

## 性能优化

### 字体加载时间

- **系统字体**: 最快（约 50-100ms）
- **本地文件**: 中等（约 100-200ms）
- **嵌入字体**: 最快（编译时加载）

### 内存占用

- **系统字体**: 约 6-8 MB
- **本地文件**: 约 6-8 MB
- **嵌入字体**: 约 6-8 MB（包含在可执行文件中）

### 建议

1. **开发环境**: 使用系统字体（快速迭代）
2. **生产环境**: 使用嵌入字体（部署简单）
3. **跨平台**: 使用本地字体文件（一致性）

## 字体许可证

### Noto Sans SC
- **许可证**: SIL Open Font License 1.1
- **商用**: ✅ 允许
- **修改**: ✅ 允许
- **分发**: ✅ 允许
- **详情**: https://scripts.sil.org/OFL

### 思源黑体 (Source Han Sans)
- **许可证**: SIL Open Font License 1.1
- **商用**: ✅ 允许
- **修改**: ✅ 允许
- **分发**: ✅ 允许

### 文泉驿微米黑
- **许可证**: GPL v3 with font embedding exception
- **商用**: ✅ 允许
- **分发**: ✅ 允许
- **嵌入**: ✅ 允许（有例外条款）

### 系统字体
- **Windows**: 仅限个人使用，商用需授权
- **macOS**: 仅限 macOS 系统使用
- **Linux**: 取决于具体字体的许可证

## 最佳实践

1. **优先使用系统字体** - 性能最好，用户体验一致
2. **提供本地字体作为备用** - 确保在所有环境都能正常显示
3. **生产环境嵌入字体** - 简化部署，避免依赖问题
4. **选择合适的字体** - 平衡文件大小和字符覆盖范围
5. **测试多平台** - 确保在 Windows、macOS、Linux 都能正常显示

## 参考资源

- [egui 字体文档](https://docs.rs/egui/latest/egui/struct.FontDefinitions.html)
- [Google Fonts](https://fonts.google.com/)
- [Noto CJK GitHub](https://github.com/googlefonts/noto-cjk)
- [思源黑体](https://github.com/adobe-fonts/source-han-sans)
- [文泉驿](http://wenq.org/)

## 技术支持

如有字体相关问题：

1. 查看应用日志（`RUST_LOG=info`）
2. 检查 `assets/fonts/README.md`
3. 验证系统字体安装
4. 提交 Issue 并附上日志

---

最后更新：2025-01-17
