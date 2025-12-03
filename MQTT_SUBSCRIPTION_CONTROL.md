# MQTT 订阅控制功能

## 功能概述

新增了 MQTT 订阅客户端的手动启动/停止控制功能，允许用户通过 Web 界面一键启动 MQTT 订阅并成为主节点，实现完整的异地协同消息接收能力。

## 核心功能

### 1. 主节点启动
- **一键启动**：点击按钮启动 MQTT 订阅客户端
- **自动注册**：启动后自动注册到监控系统
- **心跳维持**：每 10 秒自动发送心跳保持在线状态
- **状态同步**：实时更新节点在线状态到前端

### 2. 订阅控制
- **启动订阅**：创建 MQTT 客户端，订阅 `Sync/E3d` 主题
- **停止订阅**：安全停止订阅客户端和相关任务
- **状态查询**：查询当前订阅状态（是否运行、是否为主节点）
- **重复检测**：防止重复启动

### 3. 心跳机制
- **定时心跳**：每 10 秒自动更新节点状态
- **在线检测**：30 秒内无心跳自动标记为离线
- **订阅追踪**：记录订阅的 MQTT 主题列表
- **消息统计**：自动累计接收消息数量

### 4. 消息监控集成
- **接收记录**：接收到消息时自动记录到监控系统
- **投递追踪**：更新消息投递状态（Pending → Received）
- **时间戳记录**：记录消息接收时间
- **节点统计**：更新节点消息接收计数

## API 端点

### 后端 API

| 端点 | 方法 | 功能 | 请求体 | 响应 |
|------|------|------|--------|------|
| `/api/mqtt/subscription/start` | POST | 启动 MQTT 订阅 | - | `{ status, message }` |
| `/api/mqtt/subscription/stop` | POST | 停止 MQTT 订阅 | - | `{ status, message }` |
| `/api/mqtt/subscription/status` | GET | 查询订阅状态 | - | `{ status, is_running, location, is_master }` |

#### API 响应示例

**启动成功**：
```json
{
  "status": "success",
  "message": "MQTT 订阅已启动，当前节点已成为主节点"
}
```

**状态查询**：
```json
{
  "status": "success",
  "is_running": true,
  "location": "bj",
  "is_master": true
}
```

## 前端界面

### MQTT 节点监控页面增强

在 **MQTT 节点实时监控** 页面的顶部添加了订阅控制区域：

```
┌─────────────────────────────────────────────────┐
│ MQTT 节点实时监控                                │
│                                                  │
│  [●未启动] [启动订阅]  在线:0  离线:0  [刷新]   │
└─────────────────────────────────────────────────┘
```

#### 状态指示器

- **未启动状态**：
  - 灰色圆点
  - 显示 "未启动"
  - 显示 "启动订阅" 绿色按钮

- **主节点状态**：
  - 绿色脉冲圆点
  - 显示 "主节点"
  - 显示 "停止订阅" 红色按钮

### 交互流程

1. **启动订阅**：
   ```
   用户点击 "启动订阅" 按钮
     ↓
   前端调用 POST /api/mqtt/subscription/start
     ↓
   后端启动 MQTT 客户端和 watcher
     ↓
   注册节点到监控系统
     ↓
   启动心跳任务（每 10 秒）
     ↓
   返回成功消息
     ↓
   前端显示成功提示并刷新状态
     ↓
   状态指示器变为 "主节点"（绿色脉冲）
   ```

2. **停止订阅**：
   ```
   用户点击 "停止订阅" 按钮
     ↓
   前端弹出确认对话框
     ↓
   用户确认后调用 POST /api/mqtt/subscription/stop
     ↓
   后端停止 MQTT 客户端和 watcher
     ↓
   终止心跳任务
     ↓
   返回成功消息
     ↓
   前端显示成功提示并刷新状态
     ↓
   状态指示器变为 "未启动"（灰色）
   ```

## 技术实现

### 后端实现

#### 1. 订阅启动逻辑

**文件位置**：`src/web_server/sync_control_handlers.rs`

```rust
pub async fn start_mqtt_subscription_api(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::remote_runtime;

    // 检查是否已经启动
    let guard = remote_runtime::REMOTE_RUNTIME.read().await;
    if guard.is_some() {
        return Ok(Json(json!({
            "status": "error",
            "message": "MQTT 订阅已经在运行中"
        })));
    }
    drop(guard);

    // 启动运行时（使用默认 env_id）
    match remote_runtime::start_runtime("default".to_string()).await {
        Ok(_) => {
            // 注册节点到监控系统
            #[cfg(feature = "web_server")]
            {
                use crate::web_server::mqtt_monitor_handlers;
                use aios_core::get_db_option;

                let db_option = get_db_option();
                let location = db_option.location.clone();
                let node_name = format!("{}-{}", location, db_option.project_code);

                mqtt_monitor_handlers::update_node_heartbeat(
                    location.clone(),
                    node_name,
                    vec!["Sync/E3d".to_string()],
                ).await;
            }

            Ok(Json(json!({
                "status": "success",
                "message": "MQTT 订阅已启动，当前节点已成为主节点"
            })))
        }
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("启动 MQTT 订阅失败: {}", e)
        }))),
    }
}
```

#### 2. 运行时状态管理

**文件位置**：`src/web_server/remote_runtime.rs`

```rust
pub struct RuntimeState {
    pub env_id: String,
    pub mgr: Arc<AiosDBManager>,
    pub watcher_handle: Option<tokio::task::JoinHandle<()>>,
    pub mqtt_handle: Option<tokio::task::JoinHandle<()>>,
}

pub static REMOTE_RUNTIME: Lazy<RwLock<Option<RuntimeState>>> =
    Lazy::new(|| RwLock::new(None));

pub async fn start_runtime(env_id: String) -> anyhow::Result<()> {
    let (init_ms, max_ms) = query_backoff_ms(&env_id).unwrap_or((1000, 30_000));
    let mgr = Arc::new(AiosDBManager::init_form_config().await?);

    mgr.init_watcher().await.ok();

    // 启动文件监听
    let mgr_clone = mgr.clone();
    let watcher_handle = tokio::spawn(async move {
        let _ = mgr_clone.async_watch().await;
    });

    // 启动 MQTT 订阅
    let watcher_arc = mgr.watcher.clone();
    let mqtt_handle = tokio::spawn(async move {
        AiosDBManager::poll_sync_e3d_mqtt_events_with_backoff(
            watcher_arc, init_ms, max_ms
        ).await;
    });

    let mut guard = REMOTE_RUNTIME.write().await;
    *guard = Some(RuntimeState {
        env_id,
        mgr,
        watcher_handle: Some(watcher_handle),
        mqtt_handle: Some(mqtt_handle),
    });
    Ok(())
}
```

#### 3. 心跳机制

**文件位置**：`src/data_interface/db_model.rs::poll_sync_e3d_mqtt_events_with_backoff()`

```rust
// 订阅主题
let _ = mqtt_inst
    .client
    .subscribe("Sync/E3d", QoS::ExactlyOnce)
    .await;

// 启动心跳任务
#[cfg(feature = "web_server")]
{
    let location_clone = location.clone();
    let node_name = format!("{}-{}", db_option.location, db_option.project_code);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;

            use crate::web_server::mqtt_monitor_handlers;
            mqtt_monitor_handlers::update_node_heartbeat(
                location_clone.clone(),
                node_name.clone(),
                vec!["Sync/E3d".to_string()],
            ).await;
        }
    });
}
```

#### 4. 消息接收记录

**文件位置**：`src/data_interface/db_model.rs` (消息接收处理)

```rust
Incoming(Packet::Publish(p)) => {
    let sync_e3d = SyncE3dFileMsg::from(p.payload.to_vec());
    if sync_e3d.location != location {
        // 保存到 SurrealDB
        let _ = SUL_DB
            .query(format!(
                "INSERT IGNORE INTO e3d_sync {} ",
                serde_json::to_string(&sync_e3d).unwrap()
            ))
            .await;

        // 记录到监控系统
        #[cfg(feature = "web_server")]
        {
            use crate::web_server::mqtt_monitor_handlers;
            let message_id = format!("{:?}_{}", sync_e3d.timestamp, sync_e3d.location);
            mqtt_monitor_handlers::record_message_received(
                location.clone(),
                message_id
            ).await;
        }

        // 执行远程克隆
        let _ = Self::exec_delta_clone_remotes(&watcher, sync_e3d).await;
    }
    backoff = initial_backoff_ms.max(100);
}
```

### 前端实现

#### 1. 状态管理

**文件位置**：`frontend/src/components/views/MqttNodeMonitor.vue`

```vue
<script setup>
const mqttStatus = ref({
  is_running: false,
  location: '',
  is_master: false
});

const mqttLoading = ref(false);

async function loadData() {
  // 加载 MQTT 订阅状态
  const statusRes = await fetch('/api/mqtt/subscription/status');
  const statusData = await statusRes.json();

  if (statusData.status === 'success') {
    mqttStatus.value = {
      is_running: statusData.is_running || false,
      location: statusData.location || '',
      is_master: statusData.is_master || false
    };
  }

  // ... 加载其他数据
}
</script>
```

#### 2. 启动/停止按钮

```vue
<template>
  <div class="flex items-center gap-2 px-3 py-2 rounded-lg bg-white border">
    <div class="flex items-center gap-2">
      <div
        class="w-2 h-2 rounded-full"
        :class="mqttStatus.is_running ? 'bg-success animate-pulse' : 'bg-slate-300'"
      ></div>
      <span class="text-xs font-semibold text-slate-700">
        {{ mqttStatus.is_running ? '主节点' : '未启动' }}
      </span>
    </div>

    <!-- 启动按钮 -->
    <button
      v-if="!mqttStatus.is_running"
      @click="startMqttSubscription"
      class="btn btn-xs btn-success gap-1"
      :disabled="mqttLoading"
    >
      <i class="fas fa-play" :class="{ 'fa-spin': mqttLoading }"></i>
      启动订阅
    </button>

    <!-- 停止按钮 -->
    <button
      v-else
      @click="stopMqttSubscription"
      class="btn btn-xs btn-error gap-1"
      :disabled="mqttLoading"
    >
      <i class="fas fa-stop"></i>
      停止订阅
    </button>
  </div>
</template>
```

#### 3. 启动/停止函数

```javascript
async function startMqttSubscription() {
  mqttLoading.value = true;
  try {
    const res = await fetch('/api/mqtt/subscription/start', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    });
    const data = await res.json();

    if (data.status === 'success') {
      alert('✅ ' + data.message);
      await loadData(); // 刷新状态
    } else {
      alert('❌ ' + data.message);
    }
  } catch (error) {
    console.error('启动 MQTT 订阅失败:', error);
    alert('❌ 启动失败: ' + error.message);
  } finally {
    mqttLoading.value = false;
  }
}

async function stopMqttSubscription() {
  if (!confirm('确定要停止 MQTT 订阅吗？停止后将不再接收远程消息。')) {
    return;
  }

  mqttLoading.value = true;
  try {
    const res = await fetch('/api/mqtt/subscription/stop', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    });
    const data = await res.json();

    if (data.status === 'success') {
      alert('✅ ' + data.message);
      await loadData(); // 刷新状态
    } else {
      alert('❌ ' + data.message);
    }
  } catch (error) {
    console.error('停止 MQTT 订阅失败:', error);
    alert('❌ 停止失败: ' + error.message);
  } finally {
    mqttLoading.value = false;
  }
}
```

## 数据流程

### 启动流程

```
用户点击 "启动订阅"
  ↓
前端调用 /api/mqtt/subscription/start
  ↓
后端检查是否已运行
  ↓
创建 AiosDBManager 实例
  ↓
初始化文件 watcher
  ↓
启动文件监听任务（后台）
  ↓
启动 MQTT 订阅任务（后台）
  ↓
MQTT 客户端连接 broker
  ↓
订阅 "Sync/E3d" 主题
  ↓
启动心跳任务（每 10 秒）
  ↓
注册节点到监控系统
  ↓
保存运行时状态到全局变量
  ↓
返回成功响应
  ↓
前端刷新状态显示 "主节点"
```

### 心跳流程

```
心跳定时器触发（每 10 秒）
  ↓
调用 update_node_heartbeat()
  ↓
获取 MQTT_NODES 写锁
  ↓
如果节点存在：
  • 更新 last_heartbeat = Utc::now()
  • 设置 is_online = true
  • 更新 subscribed_topics
如果节点不存在：
  • 创建新节点记录
  • 初始化统计信息
  ↓
释放写锁
  ↓
前端定期轮询（5 秒）
  ↓
检测离线节点（30 秒超时）
  ↓
更新前端显示
```

### 消息接收流程

```
MQTT broker 推送消息
  ↓
poll_sync_e3d_mqtt_events 接收
  ↓
反序列化为 SyncE3dFileMsg
  ↓
检查是否为其他位置发送
  ↓
保存到 SurrealDB e3d_sync 表
  ↓
调用 record_message_received()
  ↓
更新节点统计：
  • messages_received += 1
  • last_message_time = Utc::now()
  ↓
更新消息投递状态：
  • 查找对应消息 ID
  • 更新接收者状态为 Received
  • 记录接收时间
  ↓
执行远程增量克隆
  ↓
前端轮询查询显示更新
```

## 配置要求

### DbOption.toml

```toml
# 节点位置标识（必需）
location = "bj"

# 项目代码（必需）
project_code = "1112"

# MQTT Broker 配置（必需）
mqtt_host = "127.0.0.1"
mqtt_port = 1883

# 部署站点数据库路径（可选）
deployment_sites_sqlite_path = "deployment_sites.sqlite"
```

### 数据库要求

**remote_sync_envs 表**（在 `deployment_sites.sqlite` 中）：
```sql
CREATE TABLE IF NOT EXISTS remote_sync_envs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    mqtt_host TEXT NOT NULL,
    mqtt_port INTEGER NOT NULL DEFAULT 1883,
    reconnect_initial_ms INTEGER DEFAULT 1000,
    reconnect_max_ms INTEGER DEFAULT 30000,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

需要至少有一条 `id = 'default'` 的记录（或在启动时指定 env_id）。

## 使用指南

### 启动步骤

1. **确保配置正确**：
   - 检查 `DbOption.toml` 中的 `location` 和 `mqtt_host` 配置
   - 确认 MQTT broker 正在运行（默认端口 1883）

2. **启动 Web 服务器**：
   ```bash
   cargo run --bin web_server --features web_server
   ```

3. **访问 MQTT 节点监控页面**：
   - 打开浏览器访问 `http://localhost:8080/incremental-vue`
   - 点击左侧导航栏的 **"MQTT 节点"**

4. **启动 MQTT 订阅**：
   - 查看顶部状态指示器显示 "未启动"
   - 点击 **"启动订阅"** 按钮
   - 等待成功提示：`✅ MQTT 订阅已启动，当前节点已成为主节点`
   - 状态指示器变为 "主节点"（绿色脉冲）

5. **验证运行状态**：
   - 查看节点列表，应该出现当前节点
   - 在线节点数应该 ≥ 1
   - 节点显示绿色脉冲圆点

### 停止步骤

1. **停止 MQTT 订阅**：
   - 点击 **"停止订阅"** 按钮
   - 在确认对话框中点击 "确定"
   - 等待成功提示：`✅ MQTT 订阅已停止`
   - 状态指示器变为 "未启动"（灰色）

2. **验证停止状态**：
   - 当前节点从节点列表中消失或变为离线
   - 在线节点数减少

## 监控指标

系统自动追踪以下指标：

### 节点级别
- **订阅状态**：是否为主节点
- **在线状态**：是否在线（30 秒心跳检测）
- **连接时间**：首次连接时间
- **最后心跳**：最近一次心跳时间
- **订阅主题**：订阅的 MQTT 主题列表
- **消息统计**：接收消息总数、最后接收时间

### 系统级别
- **总节点数**：注册的节点总数
- **在线节点数**：当前在线的节点数
- **离线节点数**：离线的节点数
- **消息总数**：历史消息记录总数
- **投递统计**：已完成投递数、待投递数

## 故障排查

### 问题 1：启动失败 - "启动 MQTT 订阅失败"

**可能原因**：
- MQTT broker 未运行
- `DbOption.toml` 配置错误
- 数据库连接失败

**解决方法**：
1. 检查 MQTT broker 是否运行：
   ```bash
   netstat -an | findstr 1883  # Windows
   netstat -an | grep 1883     # Linux
   ```
2. 验证 `DbOption.toml` 配置：
   ```toml
   mqtt_host = "127.0.0.1"
   mqtt_port = 1883
   location = "bj"
   ```
3. 检查后端日志输出

### 问题 2：启动后节点未出现在列表

**可能原因**：
- 心跳任务未启动
- 监控系统未正确初始化

**解决方法**：
1. 等待 10 秒（第一次心跳发送）
2. 手动刷新页面
3. 检查浏览器控制台是否有错误
4. 检查后端日志是否有心跳相关输出

### 问题 3：节点显示离线

**可能原因**：
- 心跳任务停止
- 网络连接中断
- MQTT 连接断开

**解决方法**：
1. 停止并重新启动订阅
2. 检查网络连接
3. 重启 Web 服务器

### 问题 4：消息未被接收

**可能原因**：
- 订阅未成功
- 消息主题不匹配
- 消息发送者位置与接收者相同

**解决方法**：
1. 确认订阅状态显示 "主节点"
2. 检查消息主题是否为 `Sync/E3d`
3. 确认发送者 `location` 与接收者不同
4. 查看后端日志确认消息接收

## 安全注意事项

1. **权限控制**：当前版本未实现 API 权限验证，建议添加身份认证
2. **单例保护**：系统自动防止重复启动，避免资源冲突
3. **安全停止**：停止前显示确认对话框，防止误操作
4. **状态一致性**：使用 RwLock 保护运行时状态，保证并发安全

## 性能考虑

- **心跳频率**：10 秒间隔，平衡实时性和网络开销
- **离线检测**：30 秒超时，避免误判
- **异步处理**：所有 MQTT 操作使用异步任务，不阻塞主线程
- **内存管理**：使用 Arc 共享数据，避免大量复制

## 后续优化建议

### 1. 添加身份认证
```rust
// 示例：添加 JWT 认证
pub async fn start_mqtt_subscription_api(
    _state: State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 验证 JWT token
    let token = headers.get("Authorization")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    verify_jwt(token)?;

    // ... 现有逻辑
}
```

### 2. 增强错误提示
将错误详细信息返回给前端：
```rust
Err(e) => Ok(Json(json!({
    "status": "error",
    "message": format!("启动 MQTT 订阅失败: {}", e),
    "error_code": "MQTT_START_FAILED",
    "details": format!("{:?}", e)
})))
```

### 3. 添加重启机制
自动检测异常停止并重启：
```rust
tokio::spawn(async move {
    loop {
        let guard = REMOTE_RUNTIME.read().await;
        if guard.is_none() {
            // 异常停止，尝试重启
            let _ = start_runtime("default".to_string()).await;
        }
        drop(guard);
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
});
```

### 4. WebSocket 实时推送
替代当前的 5 秒轮询：
```rust
// 当心跳更新时推送
mqtt_monitor_handlers::update_node_heartbeat(...).await;
broadcast_to_websocket_clients(node_status).await;
```

## 相关文档

- [MQTT_NODE_MONITORING.md](frontend/MQTT_NODE_MONITORING.md) - MQTT 节点监控详细文档
- [MQTT_MESSAGE_VIEWER.md](frontend/MQTT_MESSAGE_VIEWER.md) - MQTT 消息查看器文档
- [MQTT_MONITORING_SUMMARY.md](MQTT_MONITORING_SUMMARY.md) - 总体实现总结
- [MQTT_TOPOLOGY_VISUALIZATION.md](frontend/MQTT_TOPOLOGY_VISUALIZATION.md) - 拓扑可视化文档
- [REMOTE_SYNC_DEVELOPMENT_GUIDE.md](docs/REMOTE_SYNC_DEVELOPMENT_GUIDE.md) - 异地同步开发指南

## 更新日志

### 2025-11-20
- 初始版本发布
- 实现 MQTT 订阅启动/停止控制
- 添加主节点状态指示
- 实现心跳机制（10 秒间隔）
- 集成消息接收记录
- 前端界面集成完成
- 编译测试通过

---

**创建日期**：2025-11-20
**版本**：v1.0
**状态**：已完成开发，待用户测试
