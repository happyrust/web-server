# 异地同步测试环境

本目录包含用于测试异地同步功能的模拟节点配置。

## 环境概览

### 主站点 (Beijing)
- **Location**: beijing
- **HTTP**: http://localhost:8080
- **MQTT**: localhost:1883
- **数据库**: 1112
- **角色**: 源站点，生成增量更新CBA文件

### 测试站点 1 (Shanghai)
- **ID**: test-site-shanghai
- **Location**: shanghai
- **HTTP**: http://localhost:9090
- **订阅数据库**: 1112
- **接收目录**: `test-remote-site/received_cba`

### 测试站点 2 (Shenzhen)
- **ID**: test-site-shenzhen
- **Location**: shenzhen
- **HTTP**: http://localhost:9091
- **订阅数据库**: 1112

## 目录结构

```
test-remote-site/
├── README.md                    # 本文档
├── setup_test_site.sql         # SQL站点注册脚本
├── mock_receiver.ps1           # 模拟CBA文件接收器
├── test_sync.ps1               # 同步功能测试脚本
├── db_data/                    # 测试站点数据库目录
├── received_cba/               # 接收的CBA文件存放目录
└── logs/                       # 测试日志目录
```

## 快速开始

### 1. 注册测试站点 (已完成)

测试站点已通过 `register_test_sites` 工具注册到 `deployment_sites.sqlite`。

验证注册:
```bash
cargo run --bin register_test_sites
```

### 2. 启动主站点 Web 服务器

```bash
cargo run --bin web_server --features web_server
```

服务器将在 http://localhost:8080 启动,并开始监听增量更新。

### 3. 启动模拟接收器

在**新的终端窗口**中运行:

```powershell
cd test-remote-site
.\mock_receiver.ps1
```

这个脚本会:
- 监听 `cba_files/` 目录的变化
- 自动将新生成的 `.cba` 文件复制到 `received_cba/` 目录
- 模拟远程站点的HTTP接收功能

### 4. 触发增量更新

有两种方式触发增量更新测试:

**方式 A: 修改PDMS文件** (如果有实际的PDMS数据库)
- 修改监控的PDMS `.cdf` 文件
- 系统会自动检测到变化并生成CBA文件

**方式 B: 手动触发** (通过Web UI或API)
- 访问 http://localhost:8080
- 使用增量更新监控界面手动触发

### 5. 验证同步结果

运行测试脚本验证文件是否成功同步:

```powershell
.\test_sync.ps1
```

或手动检查:
```powershell
ls received_cba/*.cba
```

## 实时监控

### Web UI 监控

访问主站点的监控界面:
- **增量更新监控**: http://localhost:8080 (主页)
- **实时连接状态**: 页面右上角显示 "实时连接" 或 "离线(轮询模式)"
- **SSE事件流**: 前端通过 EventSource 订阅 `/api/sync/events/stream`

### 后端日志

Web服务器控制台会输出:
```
📡 已通过 SSE 推送增量更新通知给 N 个客户端
```

### 模拟接收器日志

mock_receiver.ps1 会输出:
```
[15:24:35] 检测到文件变化: increment_1190.cba (Created)
  [OK] 文件已同步: increment_1190.cba
  大小: 12345 bytes
```

## 测试场景

### 场景 1: 基本同步测试
1. 启动 Web 服务器
2. 启动 mock_receiver.ps1
3. 触发增量更新
4. 验证 received_cba/ 目录收到文件
5. 检查 Web UI 显示更新通知

### 场景 2: 实时推送测试
1. 打开浏览器访问 http://localhost:8080
2. 观察右上角显示 "实时连接"
3. 触发增量更新
4. 验证页面**无需刷新**即可显示新的增量记录
5. 检查浏览器控制台输出: `📨 收到实时事件: ...`

### 场景 3: 多站点测试
1. 同时模拟多个接收器 (修改 DestDir 参数)
2. 验证不同 location 的站点都能收到对应的更新

### 场景 4: 断线重连测试
1. 启动系统后关闭 Web 服务器
2. 观察前端状态变为 "离线(轮询模式)"
3. 重启 Web 服务器
4. 验证前端自动重连并显示 "实时连接"

## 技术架构

### SSE实时推送流程

```
增量检测 (increment_manager.rs)
    ↓
生成CBA文件
    ↓
发送SSE事件 (SYNC_EVENT_TX.send)
    ↓
Web服务器广播 (/api/sync/events/stream)
    ↓
前端EventSource接收
    ↓
Vue组件更新UI (IncrementalUpdateMonitor.vue)
```

### 文件同步流程

```
主站点生成CBA → cba_files/ 目录
    ↓
mock_receiver.ps1 监听文件变化
    ↓
自动复制到 received_cba/ 目录
    ↓
模拟远程站点HTTP接收完成
```

### 数据库配置

测试站点配置存储在 `deployment_sites.sqlite`:

```sql
-- 环境表
remote_sync_envs (
    id: 'test-env-001',
    location: 'beijing',
    mqtt_host: 'localhost',
    file_server_host: 'http://localhost:8080'
)

-- 站点表
remote_sync_sites (
    id: 'test-site-shanghai',
    location: 'shanghai',
    http_host: 'http://localhost:9090',
    dbnums: '1112'
)
```

## 故障排查

### 问题: 模拟接收器无法启动
- 确保 `cba_files/` 目录存在
- 检查 PowerShell 执行策略: `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned`

### 问题: 前端显示"离线(轮询模式)"
- 检查 Web 服务器是否正常运行
- 浏览器控制台查看 SSE 连接错误: `❌ SSE 连接错误: ...`
- 5秒后会自动尝试重连

### 问题: 未收到CBA文件
- 确认主站点的 `DbOption.toml` 配置了正确的 PDMS 路径
- 检查增量检测是否触发 (查看 Web 服务器日志)
- 验证 mock_receiver.ps1 正在运行并监听正确的目录

### 问题: SSE推送未触发
- 检查 `increment_manager.rs:1347-1362` 的 SSE 发送代码
- 验证 `SYNC_EVENT_TX` 广播通道是否初始化
- 查看后端日志: `📡 已通过 SSE 推送增量更新通知给 N 个客户端`

## 清理测试数据

```powershell
# 清理接收的CBA文件
Remove-Item test-remote-site\received_cba\*.cba

# 清理日志
Remove-Item test-remote-site\logs\*.log

# 删除测试站点配置 (可选)
sqlite3 deployment_sites.sqlite "DELETE FROM remote_sync_sites WHERE env_id = 'test-env-001'; DELETE FROM remote_sync_envs WHERE id = 'test-env-001';"
```

## 相关文档

- [远程同步开发指南](../docs/REMOTE_SYNC_DEVELOPMENT_GUIDE.md)
- [增量检测流程](../docs/INCREMENT_DETECTION_FLOWCHART.md)
- [Web UI设计规范](../docs/GUI_DESIGN_SPEC.md)

## 下一步

测试环境已就绪！现在可以:

1. ✅ 测试实时SSE推送功能
2. ✅ 验证多站点同步逻辑
3. ✅ 开发实际的HTTP接收端点 (替代mock_receiver.ps1)
4. ⬜ 实现MQTT订阅和消息处理
5. ⬜ 添加断点续传和错误恢复机制
