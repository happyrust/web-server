# 合并到 only-csg 分支指南

## 当前状态

- **源分支**: `egui-ui-dev` (已推送到远程)
- **目标分支**: `only-csg` (在工作树 `/Volumes/DPC/work/plant-code/gen-model-fork`)
- **提交**: a799972

## 合并步骤

### 方法 1: 在 gen-model-fork 工作树中合并（推荐）

```bash
# 1. 切换到 gen-model-fork 目录
cd /Volumes/DPC/work/plant-code/gen-model-fork

# 2. 确认当前在 only-csg 分支
git branch

# 3. 拉取最新的 egui-ui-dev 分支
git fetch origin egui-ui-dev

# 4. 合并 egui-ui-dev 到 only-csg
git merge origin/egui-ui-dev

# 5. 解决可能的冲突（如果有）
# 编辑冲突文件，然后：
git add <冲突文件>
git commit

# 6. 推送合并后的 only-csg 分支
git push origin only-csg
```

### 方法 2: 使用 cherry-pick

如果只想合并特定的提交：

```bash
cd /Volumes/DPC/work/plant-code/gen-model-fork

# Cherry-pick 特定提交
git cherry-pick a799972

# 推送
git push origin only-csg
```

### 方法 3: 创建 Pull Request

1. 访问: https://github.com/happyrust/web-server/pull/new/egui-ui-dev
2. 选择 base 分支为 `only-csg`
3. 选择 compare 分支为 `egui-ui-dev`
4. 创建 PR 并合并

## 合并内容

### 新增文件 (73 个)

#### 核心代码
- `src/gui/embedded_server/` (7 个文件)
  - mod.rs
  - server_handle.rs
  - metrics.rs
  - middleware.rs
  - config_export.rs
  - system_monitor.rs
  - network_utils.rs

#### GUI 页面
- `src/gui/pages/` (6 个新页面)
  - config_editor.rs
  - database_manage.rs
  - log_query.rs
  - site_config.rs
  - task_creation.rs
  - task_monitor.rs
  - topology_canvas.rs

#### Canvas 组件
- `src/gui/canvas/` (4 个文件)
  - edge.rs
  - layout.rs
  - node.rs
  - renderer.rs

#### 文档
- `.kiro/specs/egui-web-server-integration/` (11 个文档)
- `.kiro/specs/egui-config-wizard/` (5 个文档)
- `.kiro/specs/egui-remote-sync-ui/` (3 个文档)
- `docs/guides/` (5 个指南)

#### 测试脚本
- `scripts/test_web_server_integration.sh`
- `scripts/quick_test.sh`

### 修改文件

- `Cargo.toml` - 添加新依赖
- `Cargo.lock` - 依赖更新
- `src/gui/mod.rs` - 添加 embedded_server 模块
- `src/gui/app.rs` - 集成新页面
- `src/gui/pages/mod.rs` - 导出新页面
- `src/gui/pages/web_server.rs` - 完全重写
- `.gitignore` - 忽略大文件

## 新增依赖

```toml
parking_lot = { version = "0.12", optional = true }
arboard = { version = "3.4", optional = true }
qrcode = { version = "0.14", optional = true }
sysinfo = { version = "0.37", optional = true }
```

## 功能清单

### ✅ 已实现
1. 嵌入式 Web Server 核心
2. 实时指标收集
3. 请求日志记录
4. 系统性能监控
5. WebServerPage 控制面板
6. 配置导入导出框架
7. 自动获取本机 IP
8. 环境选择功能

### ⚠️ 部分完成
1. 图表可视化（简化版本）
2. 配置导入导出（需数据库集成）
3. 二维码显示（对话框完成）

## 潜在冲突

### 可能的冲突文件
1. `Cargo.toml` - 依赖可能冲突
2. `src/gui/mod.rs` - 模块导出可能冲突
3. `src/gui/app.rs` - 页面集成可能冲突

### 解决建议
1. **Cargo.toml**: 保留两边的依赖，合并 features
2. **mod.rs**: 合并模块导出
3. **app.rs**: 合并页面初始化和渲染逻辑

## 验证步骤

合并后验证：

```bash
# 1. 编译检查
cargo check --features gui

# 2. 编译 GUI 二进制
cargo build --bin egui_remote_sync --features gui

# 3. 运行测试
cargo test --features gui

# 4. 启动应用
cargo run --bin egui_remote_sync --features gui
```

## 回滚方案

如果合并出现问题：

```bash
# 查看合并前的提交
git reflog

# 回滚到合并前
git reset --hard HEAD@{1}

# 或者创建新分支保存当前状态
git branch backup-before-merge
git reset --hard <合并前的commit>
```

## 注意事项

1. **大文件**: `web_server` 和 `web-test/web_server` 已被移除并添加到 .gitignore
2. **测试数据**: `data/test_db/` 目录包含测试数据库文件
3. **文档**: 大量文档文件（~4000 行），确保不会覆盖现有文档

## 联系方式

如有问题，请查看：
- 实现总结: `.kiro/specs/egui-web-server-integration/IMPLEMENTATION_SUMMARY.md`
- 完成报告: `.kiro/specs/egui-web-server-integration/COMPLETION_REPORT.md`
- 新功能说明: `.kiro/specs/egui-web-server-integration/NEW_FEATURES.md`
