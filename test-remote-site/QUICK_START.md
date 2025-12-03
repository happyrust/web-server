# 异地同步测试环境 - 快速启动指南

## 🎯 一分钟启动

### 前置条件
- ✅ Rust 环境已安装
- ✅ 测试站点已注册到数据库
- ✅ 测试目录结构已创建

### 启动步骤

#### 方式 A: 完整测试 (推荐)

**终端 1 - 启动 Web 服务器**:
```bash
cd D:\work\plant\web-server
cargo run --bin web_server --features web_server
```

等待输出:
```
🌐 Web server started at http://0.0.0.0:8080
📡 SSE event stream available at /api/sync/events/stream
🔍 Watching 4 databases for incremental updates...
```

**终端 2 - 启动模拟接收器** (可选):
```powershell
cd D:\work\plant\web-server\test-remote-site
.\start_test.ps1
```

或直接运行:
```powershell
.\mock_receiver.ps1
```

**浏览器 - 打开监控页面**:
```
http://localhost:8080
```

检查右上角状态:
- ✅ 🟢 "实时连接" - SSE 正常工作
- ⚠️ 🔴 "离线(轮询模式)" - 需要刷新页面或检查服务器

#### 方式 B: 仅验证配置

```bash
# 验证测试站点注册
cargo run --bin register_test_sites

# 检查目录结构
ls -la test-remote-site/
ls -la cba_files/
```

## 🧪 触发测试

### 测试 1: 手动创建 CBA 文件
```bash
cd D:\work\plant\web-server\cba_files
echo "Test increment $(date +%s)" > increment_test_$(date +%s).cba
```

观察:
1. Web 服务器控制台: `📡 已通过 SSE 推送增量更新通知给 N 个客户端`
2. 浏览器页面: 自动显示新的增量记录 (无需刷新)
3. 模拟接收器: `[OK] 文件已同步: increment_test_xxx.cba`

### 测试 2: 修改实际 PDMS 文件
如果有实际的 PDMS 数据库:
1. 修改监控目录中的 `.cdf` 文件
2. 系统自动检测 session 变化
3. 生成 CBA 文件到 `cba_files/`
4. 触发 SSE 推送到前端
5. 模拟站点接收文件

### 测试 3: 验证 SSE 实时推送
1. 打开浏览器开发者工具 (F12)
2. 切换到 Console 标签
3. 刷新页面
4. 观察输出:
   ```
   ✅ SSE 实时连接已建立
   📨 收到实时事件: {type: "SyncCompleted", ...}
   ```
5. 触发增量更新
6. 验证页面**无需刷新**即可更新

## 📊 监控和调试

### 前端监控
- **URL**: http://localhost:8080
- **实时状态**: 页面右上角连接指示器
- **增量列表**: 主页面表格
- **最近事件**: 状态栏显示 `最近事件: SyncCompleted`

### 后端日志
```
📡 已通过 SSE 推送增量更新通知给 1 个客户端
✅ Increment detected: sessions 1191-1192
📦 Generated CBA: increment_1191.cba (37 bytes)
```

### 浏览器控制台
```javascript
// SSE 连接成功
✅ SSE 实时连接已建立

// 收到事件
📨 收到实时事件: {
  type: "SyncCompleted",
  task_id: "increment-updates",
  file_path: "increment_1191.cba, increment_1192.cba",
  timestamp: "2025-11-21T19:56:00Z"
}
```

## 🔧 故障排查

### 问题 1: 前端显示"离线(轮询模式)"

**原因**: SSE 连接失败

**解决**:
1. 确认 Web 服务器正在运行
2. 检查浏览器控制台错误信息
3. 刷新页面重新建立连接
4. 系统会每 5 秒自动尝试重连

### 问题 2: 模拟接收器未自动同步文件

**原因**: FileSystemWatcher 未正常启动

**解决**:
1. 在 PowerShell 中直接运行 `.\mock_receiver.ps1`
2. 检查源目录路径是否正确: `cba_files/`
3. 手动复制文件测试:
   ```powershell
   Copy-Item cba_files\*.cba test-remote-site\received_cba\
   ```

### 问题 3: Web 服务器启动失败

**原因**: 端口占用或依赖问题

**解决**:
1. 检查 8080 端口是否被占用:
   ```bash
   netstat -ano | findstr :8080
   ```
2. 重新构建项目:
   ```bash
   cargo clean
   cargo build --bin web_server --features web_server
   ```
3. 检查 `DbOption.toml` 配置是否正确

### 问题 4: SSE 推送不触发

**原因**: 增量检测未触发或 SSE 代码未执行

**解决**:
1. 检查 [`increment_manager.rs:1347`](../src/data_interface/increment_manager.rs#L1347) 的 SSE 发送代码
2. 验证 `SYNC_EVENT_TX` 已初始化
3. 查看后端日志: `⚠️ SSE 推送失败: ...`
4. 确认有客户端订阅 `/api/sync/events/stream`

## 📁 文件位置速查

### 核心代码
- SSE 后端推送: [`src/data_interface/increment_manager.rs:1347-1362`](../src/data_interface/increment_manager.rs#L1347)
- SSE 广播中心: [`src/web_server/sync_control_center.rs`](../src/web_server/sync_control_center.rs)
- 前端订阅: [`frontend/src/components/IncrementalUpdateMonitor.vue`](../frontend/src/components/IncrementalUpdateMonitor.vue)

### 测试工具
- 站点注册: [`src/bin/register_test_sites.rs`](../src/bin/register_test_sites.rs)
- SQL 配置: [`test-remote-site/setup_test_site.sql`](setup_test_site.sql)
- 模拟接收器: [`test-remote-site/mock_receiver.ps1`](mock_receiver.ps1)
- 同步测试: [`test-remote-site/test_sync.ps1`](test_sync.ps1)

### 文档
- 完整说明: [`test-remote-site/README.md`](README.md)
- 测试结果: [`test-remote-site/TEST_RESULTS.md`](TEST_RESULTS.md)
- 本文档: [`test-remote-site/QUICK_START.md`](QUICK_START.md)

## 🎉 成功标志

当您看到以下现象时,说明系统运行正常:

1. ✅ Web 服务器控制台输出: `📡 已通过 SSE 推送增量更新通知给 N 个客户端`
2. ✅ 浏览器右上角显示: 🟢 "实时连接"
3. ✅ 增量更新后页面**自动刷新**无需手动刷新
4. ✅ 浏览器控制台输出: `📨 收到实时事件: ...`
5. ✅ `received_cba/` 目录收到同步的 CBA 文件

## 🚀 下一步

测试环境已完全就绪!您可以:

1. **开发新功能**: 在此基础上实现 MQTT 集成、多站点路由等
2. **压力测试**: 测试多客户端并发连接和大量增量更新
3. **集成测试**: 验证完整的端到端同步流程
4. **性能优化**: 监控 SSE 连接开销和推送延迟

## 💡 提示

- SSE 连接是长连接,浏览器会自动重连
- 每个浏览器 tab 是一个独立的 SSE 客户端
- 后端日志显示当前连接的客户端数量
- 模拟接收器使用 FileSystemWatcher,有约 500ms 延迟

## 📞 问题反馈

如果遇到问题:
1. 查看 [TEST_RESULTS.md](TEST_RESULTS.md) 的故障排查部分
2. 检查 [README.md](README.md) 的技术架构说明
3. 查看后端和前端的日志输出
