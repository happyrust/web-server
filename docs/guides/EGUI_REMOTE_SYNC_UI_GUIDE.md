# egui 异地协同运维界面使用指南

## 概述

egui 异地协同运维界面是一个基于 Rust egui 框架开发的原生桌面应用，用于管理和监控异地协同数据同步系统。该应用提供了直观的图形界面，支持环境配置、拓扑可视化、实时监控和日志查询等功能。

## 功能特性

### 1. 环境管理
- 创建、编辑和删除异地协同环境
- 配置 MQTT 服务器和文件服务器
- 激活和管理多个环境
- 查看环境状态和连接信息

### 2. 拓扑配置
- 可视化拓扑画布编辑器
- 拖拽式节点创建和连接
- 自动布局算法
- 导入/导出 JSON 配置

### 3. 实时监控
- 查看同步服务运行状态
- 监控 MQTT 连接状态
- 查看队列大小和活跃任务
- 自动刷新（5秒间隔）

### 4. 日志查询
- 按环境、站点、状态筛选日志
- 分页浏览历史记录
- 查看详细日志信息
- 导出 CSV 格式

### 5. Web Server 管理
- 启动/停止内置 Web 服务器
- 配置监听地址和端口
- 查看服务器日志
- 保存配置到文件

## 构建和运行

### 构建应用

```bash
# 开发构建
cargo build --bin egui_remote_sync --features gui

# 发布构建（优化）
cargo build --bin egui_remote_sync --features gui --release
```

### 运行应用

```bash
# 开发模式
cargo run --bin egui_remote_sync --features gui

# 发布模式
./target/release/egui_remote_sync
```

## 配置

### API 地址配置

默认 API 地址为 `http://localhost:3000`。如需修改，请编辑 `src/gui/app.rs` 中的 `ApiClient::new()` 调用。

### 窗口布局

应用会自动保存窗口位置、大小和当前页面状态。配置存储在系统默认位置：
- macOS: `~/Library/Application Support/egui_remote_sync/`
- Linux: `~/.config/egui_remote_sync/`
- Windows: `%APPDATA%\egui_remote_sync\`

## 使用说明

### 环境管理

1. 点击左侧导航栏的"环境列表"
2. 点击"➕ 添加环境"按钮
3. 填写环境配置信息：
   - 环境名称（必填）
   - MQTT 主机和端口
   - 文件服务器地址
   - 地区标识
   - 数据库编号列表（逗号分隔）
4. 点击"保存"完成创建

### 拓扑配置

1. 点击左侧导航栏的"拓扑配置"
2. 使用工具栏按钮：
   - 🖱️ 选择：选择和拖拽节点
   - ➕ 添加环境：点击画布添加环境节点
   - ➕ 添加站点：点击画布添加站点节点
   - 🔗 连接：连接环境和站点
3. 使用"🔄 自动布局"优化节点位置
4. 使用"💾 保存拓扑"保存配置到后端
5. 使用"📥 导入 JSON"/"📤 导出 JSON"进行配置迁移

### 实时监控

1. 点击左侧导航栏的"监控面板"
2. 查看状态卡片：
   - 运行状态
   - MQTT 连接状态
   - 队列大小
   - 活跃任务数
3. 查看任务列表，包含文件名、源环境、目标站点、状态和进度
4. 勾选"自动刷新"启用自动更新（5秒间隔）

### 日志查询

1. 点击左侧导航栏的"日志查询"
2. 使用筛选条件：
   - 选择环境
   - 选择站点
   - 选择状态（待处理/运行中/完成/失败）
3. 点击"🔍 查询"执行搜索
4. 点击"详情"按钮查看完整日志信息
5. 使用"📤 导出 CSV"导出查询结果

### Web Server 管理

1. 点击左侧导航栏的"Web Server"
2. 配置服务器参数：
   - 监听地址（默认 0.0.0.0）
   - 监听端口（默认 3000）
   - 数据库路径
   - 静态文件目录
3. 点击"▶️ 启动服务器"启动服务
4. 查看服务器日志输出
5. 点击"⏹️ 停止服务器"停止服务

## 主题设置

1. 点击左侧导航栏的"设置"
2. 选择主题：
   - 浅色主题
   - 深色主题
3. 调整字体大小（10-20px）
4. 点击"应用"保存设置

## 技术架构

### 核心技术栈

- **GUI 框架**: egui 0.33 + eframe
- **渲染后端**: glow (OpenGL)
- **HTTP 客户端**: reqwest
- **异步运行时**: tokio
- **序列化**: serde_json
- **文件对话框**: rfd

### 模块结构

```
src/gui/
├── mod.rs              # 模块导出
├── app.rs              # 主应用结构
├── state.rs            # 全局状态管理
├── api_client.rs       # API 客户端
├── theme.rs            # 主题配置
├── pages/              # 页面组件
│   ├── environment_list.rs
│   ├── topology_canvas.rs
│   ├── monitor_dashboard.rs
│   ├── log_query.rs
│   └── web_server.rs
├── components/         # 可复用组件
│   ├── toast.rs
│   ├── confirm_dialog.rs
│   └── env_form.rs
└── canvas/             # 拓扑画布
    ├── node.rs
    ├── edge.rs
    ├── layout.rs
    └── renderer.rs
```

### API 集成

应用通过 REST API 与后端通信，支持以下端点：

- `GET /api/remote-sync/envs` - 获取环境列表
- `POST /api/remote-sync/envs` - 创建环境
- `PUT /api/remote-sync/envs/{id}` - 更新环境
- `DELETE /api/remote-sync/envs/{id}` - 删除环境
- `POST /api/remote-sync/envs/{id}/activate` - 激活环境
- `GET /api/remote-sync/sites` - 获取站点列表
- `POST /api/remote-sync/sites` - 创建站点
- `POST /api/remote-sync/sites/{id}/test` - 测试站点连接
- `POST /api/sync/start` - 启动同步服务
- `POST /api/sync/stop` - 停止同步服务
- `GET /api/sync/status` - 获取同步状态
- `GET /api/remote-sync/logs` - 查询日志
- `POST /api/topology/save` - 保存拓扑配置
- `GET /api/topology/load` - 加载拓扑配置

## 故障排除

### 应用无法启动

1. 检查是否安装了必要的系统依赖
2. 确认 Rust 工具链版本（推荐 1.70+）
3. 清理并重新构建：`cargo clean && cargo build --features gui`

### 无法连接到后端

1. 确认后端服务已启动
2. 检查 API 地址配置是否正确
3. 查看网络防火墙设置

### 界面显示异常

1. 尝试切换主题（浅色/深色）
2. 调整字体大小
3. 重置窗口布局（删除配置文件）

## 开发指南

### 添加新页面

1. 在 `src/gui/pages/` 创建新页面文件
2. 实现页面结构和 `render()` 方法
3. 在 `src/gui/pages/mod.rs` 导出页面
4. 在 `src/gui/app.rs` 中添加页面枚举和渲染逻辑
5. 更新导航栏

### 添加新组件

1. 在 `src/gui/components/` 创建组件文件
2. 实现组件结构和 `render()` 方法
3. 在 `src/gui/components/mod.rs` 导出组件
4. 在需要的页面中使用组件

### 调试技巧

1. 使用 `env_logger` 查看日志：`RUST_LOG=debug cargo run --features gui`
2. 使用 egui 内置的调试工具（按 F12）
3. 使用 `println!` 或 `dbg!` 宏输出调试信息

## 未来计划

- [ ] 添加站点配置管理页面
- [ ] 实现解析任务管理页面
- [ ] 实现模型生成配置页面
- [ ] 实现一键部署页面
- [ ] 添加 WebSocket 实时更新
- [ ] 支持多语言界面
- [ ] 添加性能图表和统计
- [ ] 支持插件系统

## 许可证

本项目遵循与主项目相同的许可证。

## 联系方式

如有问题或建议，请通过以下方式联系：
- 提交 Issue
- 发送邮件
- 参与讨论

---

最后更新：2025-01-17
