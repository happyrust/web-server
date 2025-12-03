# 增量更新实时监测测试指南

## 📋 测试目标

验证增量更新系统能够实时检测到 PDMS/E3D 数据库文件的变化，并通过 WebSocket 实时推送更新到前端界面。

## 🏗️ 系统架构

```
PDMS 文件系统 (notify监听)
    ↓
IncrementManager (增量检测)
    ↓
ProgressHub (广播中心)
    ↓
WebSocket (/ws/tasks) ← 前端 Vue 应用
    ↓
UI 实时更新 (App.vue)
```

## ✅ 测试前提条件

1. **后端服务器已启动** (`cargo run --bin web_server --features web_server`)
   - 运行在 `http://localhost:8080`
   - WebSocket 端点: `ws://localhost:8080/ws/tasks`

2. **前端开发服务器已启动** (`npm run dev` in `frontend/`)
   - 运行在 `http://localhost:3000`
   - 配置了代理转发到后端

3. **SurrealDB 正常运行**
   - 默认地址: `ws://localhost:8020`
   - 已加载 PDMS 数据库数据

4. **配置文件正确**
   - `DbOption.toml` 中 `enable_web_server_auto_detection = true`
   - `sync_live = true`
   - `project_path` 指向有效的 PDMS 项目目录

## 🧪 测试步骤

### 步骤 1: 启动服务并验证基础连接

1. **启动后端**:
   ```bash
   cargo run --bin web_server --features web_server
   ```

   预期输出:
   ```
   ✅ 数据库连接成功！
   🔍 启动增量监测后台任务...
   ✅ 增量监测后台任务已启动
   📋 正在初始化数据库文件监听器...
   📡 增量监测任务已启动，正在监听文件变化...
   ```

2. **启动前端**:
   ```bash
   cd frontend
   npm run dev
   ```

   预期输出:
   ```
   VITE v5.4.21  ready in 472 ms
   ➜  Local:   http://localhost:3000/
   ```

3. **验证 API 连接**:
   ```bash
   curl http://localhost:8080/api/incremental/status
   ```

   应返回包含 `"success": true` 的 JSON 响应

### 步骤 2: 使用测试页面验证 WebSocket

1. **打开测试页面**:
   在浏览器中打开: `http://localhost:8080/test_websocket.html`

   或直接使用文件: `file:///d:/work/plant/web-server/test_websocket.html`

2. **验证连接状态**:
   - 页面加载后应自动连接
   - 状态应显示为"已连接"（绿色）
   - 日志应显示握手成功消息

3. **测试命令交互**:
   - 点击"获取任务列表" - 应收到任务列表响应
   - 点击"手动触发检测" - 应触发增量检测

### 步骤 3: 使用 Vue 前端界面

1. **打开前端应用**:
   访问 `http://localhost:3000`

2. **检查连接状态**:
   - 右上角应显示 "ONLINE" 绿色状态指示器
   - 侧边栏底部应显示"实时通知 开启"

3. **查看监控面板**:
   - 主页应显示"监控站点"统计卡片
   - 应显示当前环境的站点卡片

### 步骤 4: 触发增量更新并验证实时推送

#### 方法 1: 手动触发检测

1. 在前端界面，找到站点卡片
2. 点击"检测变更"按钮
3. **预期结果**:
   - 界面应显示通知："开始检测变更"
   - 站点状态应变为 "Scanning"
   - 系统日志应显示检测进度
   - WebSocket 应推送进度消息

#### 方法 2: 修改 PDMS 文件（真实场景）

**⚠️ 注意**: 这需要有真实的 PDMS 环境和数据

1. **打开 PDMS/E3D 软件**

2. **修改任意对象**:
   - 例如修改一个设备的属性
   - 或创建一个新的管道元素

3. **保存修改**:
   - PDMS 会更新 `.cdf` 数据库文件
   - 文件的 session number 会递增

4. **观察系统响应**:

   **后端日志应显示**:
   ```
   发现需要增量更新的文件: "ams1112_0001"
   当前数据库属性最大sesno: 1153
   文件属性对应sesno: 1154
   Path: "D:/AVEVA/Projects/E3D2.1\AvevaMarineSample\ams000\ams1112_0001"
   Sesno Range: 1154..=1154
   ```

   **前端界面应显示**:
   - 待同步项目数增加
   - 变更文件列表更新
   - 系统日志新增检测记录
   - 实时通知弹窗

#### 方法 3: 使用文件触碰（模拟变化）

如果没有 PDMS 环境，可以手动触碰文件来模拟:

```bash
# Windows
copy /b "D:\AVEVA\Projects\E3D2.1\AvevaMarineSample\ams000\ams1112_0001" +,,

# Linux/Mac
touch /path/to/pdms/file
```

**注意**: 这只会触发文件系统事件，不会真正改变 session number

### 步骤 5: 验证 WebSocket 消息流

使用浏览器开发者工具监控 WebSocket 消息:

1. **打开开发者工具** (F12)
2. **切换到 Network 标签**
3. **筛选 WS (WebSocket)**
4. **查看消息流**

**预期消息类型**:

1. **握手消息** (连接建立时):
   ```json
   {
     "type": "handshake",
     "message": "多任务订阅连接成功",
     "active_tasks": []
   }
   ```

2. **任务列表响应** (请求 `{ "action": "list" }` 后):
   ```json
   {
     "type": "task_list",
     "tasks": [...]
   }
   ```

3. **进度更新消息** (检测/同步过程中):
   ```json
   {
     "task_id": "detect-1234",
     "status": "Running",
     "percentage": 50,
     "current_step": "正在扫描文件...",
     "message": "Processing session 1154"
   }
   ```

4. **完成消息**:
   ```json
   {
     "task_id": "detect-1234",
     "status": "Completed",
     "percentage": 100,
     "message": "检测完成"
   }
   ```

## 🔍 关键检查点

### ✅ WebSocket 连接

- [ ] 前端能成功连接到 `ws://localhost:8080/ws/tasks`
- [ ] 收到握手成功消息
- [ ] 连接状态显示为"已连接"

### ✅ 实时消息推送

- [ ] 触发检测后能收到进度消息
- [ ] 消息包含 task_id、status、percentage
- [ ] 完成后状态更新为 Completed

### ✅ 前端界面响应

- [ ] 站点状态实时更新 (Idle → Scanning → Completed)
- [ ] 待同步项目数动态变化
- [ ] 系统日志实时追加新记录
- [ ] Toast 通知正确显示

### ✅ 历史记录持久化

- [ ] 同步历史页面显示最新记录
- [ ] 历史记录包含文件名、时间戳、session范围
- [ ] 分页功能正常

## 🐛 常见问题排查

### 问题 1: WebSocket 连接失败

**症状**: 前端显示"实时监听已断开"

**排查步骤**:
1. 检查后端是否启动: `curl http://localhost:8080`
2. 检查 WebSocket 路由是否注册 (查看 `src/web_server/mod.rs`)
3. 查看浏览器控制台错误信息
4. 确认防火墙没有阻止 WebSocket

### 问题 2: 没有收到实时消息

**症状**: 触发检测后前端无响应

**排查步骤**:
1. 检查后端日志是否有 `[WebSocket]` 相关输出
2. 确认 `ProgressHub` 是否正确广播消息
3. 验证前端 `setupWebSocket()` 函数是否被调用
4. 检查 `useWebSocket.js` 的事件监听器

### 问题 3: 增量检测未触发

**症状**: 修改文件后没有检测到变化

**排查步骤**:
1. 确认 `DbOption.toml` 中 `enable_web_server_auto_detection = true`
2. 检查 `project_path` 是否正确
3. 查看后端日志中的 `watch_dirs` 列表
4. 确认文件修改确实改变了 session number (不是只触碰文件)

### 问题 4: 前端代理错误

**症状**: 前端显示 `http proxy error: /api/...`

**排查步骤**:
1. 确认后端已启动 (`localhost:8080`)
2. 检查 `vite.config.js` 代理配置
3. 尝试直接访问后端 API: `curl http://localhost:8080/api/incremental/status`

## 📊 性能验证

### 响应时间测试

使用以下脚本测试实时性:

```javascript
// 在浏览器控制台执行
const startTime = Date.now();
fetch('http://localhost:8080/api/incremental/detect/default', { method: 'POST' })
  .then(() => {
    console.log('API 响应时间:', Date.now() - startTime, 'ms');
  });

// 同时监听 WebSocket 消息到达时间
```

**预期指标**:
- API 响应时间: < 100ms
- WebSocket 首条消息延迟: < 200ms
- 进度更新频率: 每 100-500ms

### 并发测试

模拟多个客户端同时连接:

```bash
# 使用 wscat 工具
npm install -g wscat
wscat -c ws://localhost:8080/ws/tasks
```

**预期**: 支持至少 10 个并发 WebSocket 连接

## 📝 测试记录模板

```markdown
### 测试执行记录

**日期**: 2025-11-19
**测试人员**: [你的名字]
**测试环境**: Windows / Linux / macOS

#### 测试结果

- [ ] WebSocket 连接成功
- [ ] 实时消息推送正常
- [ ] 前端界面实时更新
- [ ] 增量检测触发正常
- [ ] 历史记录持久化

#### 发现的问题

1. [描述问题]
2. [截图或日志]

#### 性能数据

- API 响应时间: ___ ms
- WebSocket 首条消息延迟: ___ ms
- 并发连接数: ___

#### 备注

[任何其他观察或建议]
```

## 🎯 下一步优化建议

1. **添加心跳机制**: 定期发送 ping/pong 保持连接活跃
2. **消息确认机制**: ACK 确保重要消息不丢失
3. **断线重连优化**: 指数退避策略
4. **消息队列**: 使用 Redis 存储未送达消息
5. **压力测试**: 使用 Apache Bench 或 wrk 进行负载测试

## 📚 相关文档

- [WebSocket API 参考](src/web_server/ws/progress.rs)
- [增量检测实现](src/data_interface/increment_manager.rs)
- [前端 WebSocket 封装](frontend/src/composables/useWebSocket.js)
- [Vue 应用主逻辑](frontend/src/App.vue)
