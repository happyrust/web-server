# 浏览器开发者工具检查指南

## 📊 查看 API 请求

### 步骤 1: 打开 Network 标签并筛选

1. **按 F12** 打开开发者工具
2. **切换到 "Network" 标签**
3. **在 Filter 框中输入**: `status`
   - 这会筛选出包含 "status" 的请求
4. **或者点击 "Fetch/XHR" 按钮** 只看 AJAX 请求

### 步骤 2: 刷新页面并观察请求

1. **按 Ctrl+R** 刷新页面
2. 你应该看到这些请求：
   ```
   status          GET    200    /api/incremental/status
   logs            GET    200    /api/incremental/logs
   history?page=1  GET    200    /api/incremental/history
   ```

### 步骤 3: 查看 status 请求的响应

1. **点击 `status` 这一行**
2. **切换到 "Response" 或"Preview" 标签**
3. **展开 JSON 数据**，查找：
   ```json
   {
     "success": true,
     "sites": [...],
     "sync_history_from_db": [
       {
         "session_range": "1182-1182",  ← 最新的增量更新
         "file_names": ["ams1112_0001"],
         "timestamp": "2025-11-19T03:05:..."
       },
       ...
     ]
   }
   ```

## 🔍 验证增量更新是否被检测到

### 方法 1: 通过 Network 标签

**步骤**:
1. 打开 Network 标签
2. **清空请求列表** (点击 🚫 图标)
3. **修改 PDMS 文件并保存**
4. **等待 5-10 秒**
5. **手动点击浏览器的刷新按钮**
6. **查看新的 `status` 请求响应**

**预期结果**:
- `sync_history_from_db` 数组的**第一个元素**应该是新的记录
- `session_range` 应该是新的会话号
- `timestamp` 应该是刚才的时间

### 方法 2: 通过 Console 标签

**步骤**:
1. 切换到 "Console" 标签
2. 输入以下代码并按回车：
   ```javascript
   fetch('http://localhost:8080/api/incremental/status')
     .then(r => r.json())
     .then(data => {
       console.log('最新同步记录:', data.sync_history_from_db[0]);
       console.log('会话范围:', data.sync_history_from_db[0].session_range);
       console.log('文件名:', data.sync_history_from_db[0].file_names);
       console.log('时间戳:', data.sync_history_from_db[0].timestamp);
     });
   ```

3. **修改 PDMS 文件**
4. **等待 10 秒**
5. **再次运行上面的代码**
6. **对比两次的结果**

**预期**:
```javascript
// 第一次
最新同步记录: {session_range: "1182-1182", ...}

// 修改文件后，第二次
最新同步记录: {session_range: "1183-1183", ...} ← 会话号增加
```

## 🎯 查看 WebSocket 连接

### 步骤 1: 筛选 WebSocket

1. 在 Network 标签中
2. **点击 "WS" 或 "WebSocket" 筛选按钮**
3. 你应该看到：
   ```
   tasks    101 Switching Protocols    /ws/tasks
   ```

### 步骤 2: 查看 WebSocket 消息

1. **点击 `tasks` 这一行**
2. **切换到 "Messages" 标签**
3. 你应该看到消息列表：

```
↑ {"action":"list"}                        ← 前端发送的命令
↓ {"type":"handshake","message":"..."}    ← 后端的握手响应
↓ {"type":"task_list","tasks":[]}         ← 任务列表响应
```

### 步骤 3: 监控实时消息

**实验**:
1. **保持 Messages 标签打开**
2. **在前端点击"检测变更"按钮**
3. **观察是否有新消息出现**

**预期 (如果 ProgressHub 已集成)**:
```
↓ {"task_id":"detect-xxx","status":"Running","percentage":0,...}
↓ {"task_id":"detect-xxx","status":"Running","percentage":50,...}
↓ {"task_id":"detect-xxx","status":"Completed","percentage":100,...}
```

**实际 (当前情况)**:
```
(无新消息) ← 因为 increment_manager 没有调用 ProgressHub.broadcast()
```

## 📸 截图重点

### 有用的截图 1: API 响应
**位置**: Network → status → Response

**显示内容**:
```json
{
  "sync_history_from_db": [
    {
      "session_range": "1182-1182",  ← 关键信息
      "timestamp": "...",
      "file_names": ["ams1112_0001"]
    }
  ]
}
```

### 有用的截图 2: WebSocket Messages
**位置**: Network → WS → tasks → Messages

**显示内容**:
```
Time    Length  Data
11:05   45      ↓ {"type":"handshake",...}
11:05   30      ↑ {"action":"list"}
11:05   52      ↓ {"type":"task_list","tasks":[]}
```

### 有用的截图 3: Console 日志
**位置**: Console 标签

**显示内容**:
```javascript
[WebSocket] 收到消息: {type: "handshake", ...}
[WebSocket] 任务列表更新: []
```

## 🧪 完整测试流程

### 测试 1: 验证后端检测正常

1. **打开 Console 标签**
2. **粘贴并执行**:
   ```javascript
   // 保存初始状态
   let before = null;
   fetch('http://localhost:8080/api/incremental/status')
     .then(r => r.json())
     .then(data => {
       before = data.sync_history_from_db[0];
       console.log('修改前最新记录:', before.session_range, before.timestamp);
       console.log('✅ 准备就绪，现在请修改 PDMS 文件...');
     });
   ```

3. **修改 PDMS 文件并保存**

4. **等待 15 秒，然后执行**:
   ```javascript
   // 检查变化
   fetch('http://localhost:8080/api/incremental/status')
     .then(r => r.json())
     .then(data => {
       const after = data.sync_history_from_db[0];
       console.log('修改后最新记录:', after.session_range, after.timestamp);

       if (after.timestamp !== before.timestamp) {
         console.log('✅ 后端检测正常！新增了记录');
         console.log('   文件:', after.file_names);
         console.log('   会话范围:', after.session_range);
       } else {
         console.log('❌ 未检测到变化，请等待更长时间');
       }
     });
   ```

**预期结果**:
```
修改前最新记录: 1182-1182  2025-11-19T03:05:28...
✅ 准备就绪，现在请修改 PDMS 文件...
修改后最新记录: 1183-1183  2025-11-19T03:15:42...
✅ 后端检测正常！新增了记录
   文件: ["ams1112_0001"]
   会话范围: 1183-1183
```

### 测试 2: 验证 WebSocket 推送 (预期失败)

1. **打开 test_websocket.html**:
   ```
   http://localhost:8080/test_websocket.html
   ```

2. **确认连接成功** (状态显示"已连接")

3. **点击"手动触发检测"按钮**

4. **观察日志区域**

**当前预期**:
```
🤝 握手成功
✅ 检测触发成功
(然后就没有进度消息了) ← 因为缺少 ProgressHub 广播
```

**修复后预期**:
```
🤝 握手成功
✅ 检测触发成功
🔄 [detect-xxx] Running: 10% - 正在扫描文件...
🔄 [detect-xxx] Running: 50% - 正在处理会话...
✅ [detect-xxx] Completed: 100% - 检测完成
```

## 💡 快速诊断命令

### 一键检查所有状态

**粘贴到 Console**:
```javascript
(async () => {
  console.log('🔍 开始诊断...\n');

  // 1. 检查后端连接
  try {
    const r = await fetch('http://localhost:8080/api/incremental/status');
    const data = await r.json();
    console.log('✅ 后端连接正常');
    console.log('   监控站点数:', data.sites?.length || 0);
    console.log('   历史记录数:', data.sync_history_from_db?.length || 0);
    console.log('   最新会话:', data.sync_history_from_db?.[0]?.session_range || '无');
  } catch (e) {
    console.log('❌ 后端连接失败:', e.message);
  }

  // 2. 检查 WebSocket (如果已连接)
  if (window.ws && window.ws.readyState === WebSocket.OPEN) {
    console.log('✅ WebSocket 已连接');
  } else {
    console.log('❌ WebSocket 未连接');
  }

  // 3. 检查前端版本
  console.log('ℹ️  前端地址:', window.location.href);
  console.log('ℹ️  User Agent:', navigator.userAgent.split(' ').pop());

  console.log('\n✅ 诊断完成');
})();
```

## 📋 问题排查清单

- [ ] Network 标签能看到 `status` 请求
- [ ] `status` 请求返回 200 状态码
- [ ] `sync_history_from_db` 不为空
- [ ] 修改文件后，历史记录会增加
- [ ] WebSocket 标签能看到 `tasks` 连接
- [ ] `tasks` 显示 "101 Switching Protocols"
- [ ] Messages 标签有握手消息
- [ ] 点击"获取任务列表"有响应
- [ ] **修改文件后，WebSocket 没有收到进度消息** ← 当前问题

## 🎯 下一步

确认上述检查点后，你可以选择：

1. **等待修复**: 我可以修改 `increment_manager.rs` 集成 ProgressHub
2. **使用轮询**: 前端每5秒刷新一次状态 (临时方案)
3. **接受现状**: 手动刷新页面查看更新

你希望选择哪个方案？
