# egui 异地协同运维界面 - 快速启动指南

## 快速开始

### 1. 准备中文字体（可选）

应用会自动尝试加载系统中文字体。如果需要使用特定字体：

```bash
# 下载 Noto Sans SC 字体（可选）
./assets/fonts/download_font.sh
```

或者直接使用系统字体（推荐）：
- **macOS**: 自动使用 PingFang SC
- **Windows**: 自动使用 Microsoft YaHei
- **Linux**: 自动使用 Noto Sans CJK 或 WenQuanYi

详细说明请查看：[assets/fonts/README.md](assets/fonts/README.md)

### 2. 编译应用（Debug 模式）

```bash
cargo build --bin egui_remote_sync --features gui
```

### 3. 运行应用

```bash
cargo run --bin egui_remote_sync --features gui
```

### 3. 发布构建（可选）

```bash
cargo build --bin egui_remote_sync --features gui --release
```

运行发布版本：
```bash
./target/release/egui_remote_sync
```

## 功能概览

### 已实现的核心功能

1. **环境管理** - 创建、编辑、删除异地协同环境
2. **站点配置** - 配置站点的 E3D 目录和 SurrealDB 连接 ⭐ 新增
3. **拓扑配置** - 可视化配置环境和站点连接关系
4. **实时监控** - 查看同步状态、任务进度
5. **日志查询** - 筛选和查看同步日志
6. **Web Server 管理** - 启动和管理内置 Web 服务器
7. **CBA 文件服务器** - 提供对 CBA 目录的 HTTP 访问 ⭐ 新增

### 界面导航

```
左侧导航栏：
├── 异地协同
│   ├── 环境列表      ✅ 已实现
│   ├── 站点配置      ✅ 已实现 ⭐ 新增
│   ├── 拓扑配置      ✅ 已实现
│   ├── 监控面板      ✅ 已实现
│   └── 日志查询      ✅ 已实现
└── 系统管理
    ├── Web Server    ✅ 已实现（含 CBA 服务器）⭐ 增强
    └── 设置          ✅ 已实现
```

## 使用示例

### 创建环境

1. 点击左侧"环境列表"
2. 点击"➕ 添加环境"
3. 填写配置：
   - 环境名称：测试环境
   - MQTT 主机：localhost
   - MQTT 端口：1883
   - 文件服务器：http://localhost:8080
   - 地区：北京
   - 数据库编号：7999,8001
4. 点击"保存"

### 配置拓扑

1. 点击左侧"拓扑配置"
2. 点击"➕ 添加环境"，在画布上点击创建环境节点
3. 点击"➕ 添加站点"，在画布上点击创建站点节点
4. 点击"🔄 自动布局"优化节点位置
5. 点击"💾 保存拓扑"保存配置

### 查看监控

1. 点击左侧"监控面板"
2. 查看状态卡片（运行状态、MQTT 连接、队列大小、活跃任务）
3. 查看任务列表和进度
4. 勾选"自动刷新"启用 5 秒自动更新

### 查询日志

1. 点击左侧"日志查询"
2. 选择筛选条件（环境、站点、状态）
3. 点击"🔍 查询"
4. 点击"详情"查看完整日志
5. 点击"📤 导出 CSV"导出结果

### 配置站点（新功能）⭐

1. 点击左侧"站点配置"
2. 点击"➕ 添加站点"
3. 填写基本信息（名称、环境、HTTP 地址等）
4. 配置 E3D 监听目录
5. 配置 SurrealDB 连接（主机、端口、数据库、用户名、密码）
6. 点击"💾 保存"

### 管理 Web Server（增强功能）⭐

1. 点击左侧"Web Server"
2. 配置 Web Server 二进制路径（新增）
3. 配置监听地址和端口
4. 点击"▶️ 启动 Web Server"
5. 配置 CBA 目录和端口（新增）
6. 点击"▶️ 启动 CBA 服务器"（新增）
7. 查看服务器日志
8. 点击"⏹️ 停止服务器"停止服务

## 配置说明

### API 地址

默认 API 地址：`http://localhost:3000`

如需修改，编辑 `src/gui/app.rs` 第 48 行：
```rust
api_client: ApiClient::new("http://localhost:3000"),
```

### 配置文件位置

应用配置自动保存在：
- **macOS**: `~/Library/Application Support/egui_remote_sync/`
- **Linux**: `~/.config/egui_remote_sync/`
- **Windows**: `%APPDATA%\egui_remote_sync\`

### 主题设置

1. 点击左侧"设置"
2. 选择主题（浅色/深色）
3. 调整字体大小（10-20px）
4. 点击"应用"

## 故障排除

### 编译失败

```bash
# 清理并重新编译
cargo clean
cargo build --bin egui_remote_sync --features gui
```

### 无法连接后端

1. 确认后端服务已启动
2. 检查 API 地址配置
3. 查看防火墙设置

### 界面显示异常

1. 尝试切换主题
2. 调整字体大小
3. 删除配置文件重置

## 开发调试

### 启用日志

```bash
RUST_LOG=debug cargo run --bin egui_remote_sync --features gui
```

### 查看编译警告

```bash
cargo build --bin egui_remote_sync --features gui 2>&1 | grep warning
```

## 文档链接

- **用户指南**: [docs/guides/EGUI_REMOTE_SYNC_UI_GUIDE.md](docs/guides/EGUI_REMOTE_SYNC_UI_GUIDE.md)
- **新功能指南**: [docs/guides/NEW_FEATURES_GUIDE.md](docs/guides/NEW_FEATURES_GUIDE.md) ⭐ 新增
- **开发文档**: [src/gui/README.md](src/gui/README.md)
- **实现总结**: [.kiro/specs/egui-remote-sync-ui/IMPLEMENTATION_SUMMARY.md](.kiro/specs/egui-remote-sync-ui/IMPLEMENTATION_SUMMARY.md)

## 技术支持

如有问题，请查看：
1. 实现总结文档了解已知问题
2. 用户指南了解详细功能
3. 开发文档了解架构设计

---

快速启动指南 | 最后更新：2025-01-17
