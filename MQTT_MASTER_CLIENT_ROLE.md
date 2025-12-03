# MQTT 主从节点角色管理

## 功能概述

新增了 MQTT 节点角色管理功能，支持将节点设置为**主节点（Master）**或**从节点（Client）**，实现灵活的异地协同架构：

- **主节点**：可以启动 MQTT Broker（服务器），为其他节点提供消息中转服务
- **从节点**：只能作为 MQTT Client（客户端），订阅消息并接收增量更新

## 核心架构

### 角色定义

| 角色 | 功能 | MQTT Broker | MQTT 订阅 | 适用场景 |
|------|------|-------------|-----------|----------|
| **主节点** | 消息中转中心 | ✅ 可启动 | ✅ 可订阅 | 中心机房、稳定网络环境 |
| **从节点** | 消息接收者 | ❌ 不可启动 | ✅ 可订阅 | 远程站点、边缘节点 |

### 典型拓扑

```
┌──────────────────────────────────────────────────┐
│             主节点 (BJ)                           │
│  ┌────────────────────────────────────────┐     │
│  │  MQTT Broker (端口 1883)                │     │
│  │  • 接收所有节点的增量消息                │     │
│  │  • 转发消息到订阅的从节点                │     │
│  └────────────────────────────────────────┘     │
│          ▲                    ▲                  │
│          │                    │                  │
│  MQTT 订阅 (Sync/E3d)         │                  │
│          │                    │                  │
└──────────┼────────────────────┼──────────────────┘
           │                    │
    发送增量消息            发送增量消息
           │                    │
┌──────────┴────────┐  ┌────────┴──────────┐
│  从节点 (SJZ)      │  │  从节点 (TJ)       │
│  • MQTT Client     │  │  • MQTT Client     │
│  • 订阅 Sync/E3d   │  │  • 订阅 Sync/E3d   │
│  • 发送增量到 BJ    │  │  • 发送增量到 BJ    │
│  • 接收其他节点增量 │  │  • 接收其他节点增量 │
└───────────────────┘  └───────────────────┘
```

## API 端点

### 1. 节点角色管理

| 端点 | 方法 | 功能 | 响应 |
|------|------|------|------|
| `/api/mqtt/node/set-master` | POST | 设置为主节点 | `{ status, message }` |
| `/api/mqtt/node/set-client` | POST | 设置为从节点 | `{ status, message }` |

### 2. 订阅状态查询（增强）

| 端点 | 方法 | 功能 | 响应字段 |
|------|------|------|---------|
| `/api/mqtt/subscription/status` | GET | 查询节点状态 | `is_subscription_running`, `is_server_running`, `is_master_node`, `node_role` |

#### 响应示例

**主节点（Broker 运行中）**：
```json
{
  "status": "success",
  "is_subscription_running": true,
  "is_server_running": true,
  "location": "bj",
  "is_master_node": true,
  "node_role": "master"
}
```

**从节点（仅订阅）**：
```json
{
  "status": "success",
  "is_subscription_running": true,
  "is_server_running": false,
  "location": "sjz",
  "is_master_node": false,
  "node_role": "client"
}
```

### 3. MQTT Server 控制（仅主节点可用）

| 端点 | 方法 | 功能 | 请求体 |
|------|------|------|--------|
| `/api/sync/mqtt/start` | POST | 启动 Broker | `{ port?: 1883 }` |
| `/api/sync/mqtt/stop` | POST | 停止 Broker | - |

## 前端界面

### MQTT 节点监控页面布局

```
┌─────────────────────────────────────────────────────────────────┐
│ MQTT 节点实时监控                                                │
│                                                                  │
│  [角色:主节点👥] [Broker运行中●] [订阅中●] 在线:2  离线:0  [刷新] │
└─────────────────────────────────────────────────────────────────┘
```

#### 控制区域组成

**1. 节点角色指示器**
```
┌────────────────────────┐
│ 角色: 主节点  👑        │  ← 主节点（紫色）
└────────────────────────┘

┌────────────────────────┐
│ 角色: 从节点  👥        │  ← 从节点（蓝色）
└────────────────────────┘
```

**2. MQTT Broker 控制（仅主节点显示）**
```
┌──────────────────────────────────────┐
│  ●Broker运行中   [停止Broker]        │  ← Broker 已启动
└──────────────────────────────────────┘

┌──────────────────────────────────────┐
│  ○Broker未启动   [启动Broker]        │  ← Broker 未启动
└──────────────────────────────────────┘
```

**3. MQTT 订阅控制（所有节点）**
```
┌──────────────────────────────────────┐
│  ●订阅中   [停止订阅]                │  ← 正在订阅
└──────────────────────────────────────┘

┌──────────────────────────────────────┐
│  ○未订阅   [启动订阅]                │  ← 未订阅
└──────────────────────────────────────┘
```

### 交互流程

#### 设置主节点

```
用户点击 👑 图标
  ↓
弹出确认对话框
  ↓
用户确认
  ↓
调用 POST /api/mqtt/node/set-master
  ↓
保存到 SQLite node_config 表
  ↓
返回成功消息
  ↓
刷新界面显示
  ↓
角色变为 "主节点"（紫色）
显示 "MQTT Broker 控制" 区域
```

#### 启动 MQTT Broker（仅主节点）

```
主节点点击 [启动Broker]
  ↓
调用 POST /api/sync/mqtt/start
  ↓
启动 rumqttd broker (端口 1883)
  ↓
保存到 SYNC_CONTROL_CENTER 状态
  ↓
返回成功消息
  ↓
Broker 状态变为 "运行中"（紫色脉冲）
```

## 技术实现

### 后端实现

#### 1. 节点配置存储

**数据库表**：`deployment_sites.sqlite::node_config`

```sql
CREATE TABLE IF NOT EXISTS node_config (
    location TEXT PRIMARY KEY,
    is_master BOOLEAN NOT NULL DEFAULT 0,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

**示例数据**：
```sql
INSERT INTO node_config VALUES ('bj', 1, '2025-11-20 10:00:00');
INSERT INTO node_config VALUES ('sjz', 0, '2025-11-20 10:00:00');
INSERT INTO node_config VALUES ('tj', 0, '2025-11-20 10:00:00');
```

#### 2. 角色检查函数

**文件**：`src/web_server/sync_control_handlers.rs`

```rust
fn check_is_master_node(location: &str) -> bool {
    let db_path = /* 读取配置 */;
    let conn = rusqlite::Connection::open(&db_path)?;

    // 创建表（如果不存在）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS node_config (
            location TEXT PRIMARY KEY,
            is_master BOOLEAN NOT NULL DEFAULT 0,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )", []
    )?;

    // 查询主节点标记
    conn.query_row(
        "SELECT is_master FROM node_config WHERE location = ?1",
        [location],
        |row| row.get::<_, bool>(0),
    ).unwrap_or(false)
}
```

#### 3. 角色切换 API

```rust
pub async fn set_as_master_node(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db_option = get_db_option();
    let location = db_option.location.clone();

    match save_master_node_flag(&location, true) {
        Ok(_) => Ok(Json(json!({
            "status": "success",
            "message": format!("节点 {} 已设置为主节点", location)
        }))),
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("设置主节点失败: {}", e)
        }))),
    }
}
```

#### 4. 增强的状态查询

```rust
pub async fn get_mqtt_subscription_status(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let guard = remote_runtime::REMOTE_RUNTIME.read().await;
    let is_subscription_running = guard.is_some();

    let db_option = get_db_option();
    let location = db_option.location.clone();

    // 检查是否为主节点
    let is_master_node = check_is_master_node(&location);

    // 检查 MQTT server 状态
    let center = SYNC_CONTROL_CENTER.read().await;
    let is_server_running = center.mqtt_server.is_some();

    Ok(Json(json!({
        "status": "success",
        "is_subscription_running": is_subscription_running,
        "is_server_running": is_server_running,
        "location": location,
        "is_master_node": is_master_node,
        "node_role": if is_master_node { "master" } else { "client" }
    })))
}
```

### 前端实现

#### 1. 状态管理

```vue
<script setup>
const mqttStatus = ref({
  is_subscription_running: false,  // 订阅是否运行
  is_server_running: false,        // Broker 是否运行
  location: '',                     // 节点位置
  is_master_node: false,            // 是否为主节点
  node_role: 'client'               // 节点角色
});

const roleLoading = ref(false);
const mqttLoading = ref(false);
</script>
```

#### 2. 界面控件

**角色切换**：
```vue
<div class="flex items-center gap-2">
  <span class="text-xs text-slate-600">角色:</span>
  <span class="text-xs font-bold"
        :class="mqttStatus.is_master_node ? 'text-purple-600' : 'text-blue-600'">
    {{ mqttStatus.is_master_node ? '主节点' : '从节点' }}
  </span>
  <!-- 主节点 → 从节点 -->
  <button v-if="mqttStatus.is_master_node"
          @click="setAsClientNode"
          class="btn btn-xs btn-ghost">
    <i class="fas fa-users text-blue-500"></i>
  </button>
  <!-- 从节点 → 主节点 -->
  <button v-else
          @click="setAsMasterNode"
          class="btn btn-xs btn-ghost">
    <i class="fas fa-crown text-yellow-500"></i>
  </button>
</div>
```

**Broker 控制（条件渲染）**：
```vue
<div v-if="mqttStatus.is_master_node">
  <button v-if="!mqttStatus.is_server_running"
          @click="startMqttServer">
    <i class="fas fa-server"></i> 启动Broker
  </button>
  <button v-else @click="stopMqttServer">
    <i class="fas fa-stop"></i> 停止Broker
  </button>
</div>
```

#### 3. 角色切换函数

```javascript
async function setAsMasterNode() {
  if (!confirm('确定要将当前节点设为主节点吗？主节点可以启动 MQTT Broker。')) {
    return;
  }

  roleLoading.value = true;
  try {
    const res = await fetch('/api/mqtt/node/set-master', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    });
    const data = await res.json();

    if (data.status === 'success') {
      alert('✅ ' + data.message);
      await loadData();  // 刷新状态
    }
  } catch (error) {
    alert('❌ 设置失败: ' + error.message);
  } finally {
    roleLoading.value = false;
  }
}
```

## 使用指南

### 初始设置

#### 1. 设置主节点

1. 在中心机房节点上访问 MQTT 节点监控页面
2. 查看当前角色（默认为 "从节点"）
3. 点击 👑 图标
4. 确认对话框点击 "确定"
5. 等待成功提示：`✅ 节点 bj 已设置为主节点`
6. 界面刷新，显示 "MQTT Broker 控制" 区域

#### 2. 启动 MQTT Broker（主节点）

1. 确认节点角色为 "主节点"
2. 点击 **[启动Broker]** 按钮
3. 等待成功提示：`✅ MQTT服务器已启动在端口 1883`
4. Broker 状态变为 "运行中"（紫色脉冲）

#### 3. 配置从节点连接

在从节点的 `DbOption.toml` 中配置主节点地址：

```toml
mqtt_host = "192.168.1.100"  # 主节点 IP
mqtt_port = 1883
location = "sjz"
```

#### 4. 启动从节点订阅

1. 从节点访问 MQTT 节点监控页面
2. 确认角色为 "从节点"
3. 点击 **[启动订阅]** 按钮
4. 等待成功提示：`✅ MQTT 订阅已启动，当前节点已成为主节点`
5. 订阅状态变为 "订阅中"（绿色脉冲）

### 运行场景

#### 场景 1：主节点发送增量

```
主节点 (BJ) 检测到增量更新
  ↓
生成 CBA 压缩包
  ↓
发布到 MQTT Broker (本地 127.0.0.1:1883)
主题: Sync/E3d
  ↓
MQTT Broker 转发给所有订阅者
  ↓
从节点 (SJZ, TJ) 接收消息
  ↓
应用增量更新
```

#### 场景 2：从节点发送增量

```
从节点 (SJZ) 检测到增量更新
  ↓
生成 CBA 压缩包
  ↓
发布到 MQTT Broker (主节点 192.168.1.100:1883)
主题: Sync/E3d
  ↓
MQTT Broker 转发给所有订阅者
  ↓
主节点 (BJ) 和其他从节点 (TJ) 接收消息
  ↓
应用增量更新
```

## 角色切换注意事项

### 从节点 → 主节点

**影响**：
- 获得启动 MQTT Broker 的权限
- 可以为其他节点提供中转服务
- 需要更稳定的网络环境和更高的性能

**建议**：
- 只在中心机房或核心节点设置
- 确保节点可以被其他节点访问（防火墙、网络配置）
- 监控 Broker 性能和连接数

### 主节点 → 从节点

**前置条件**：
- ✅ 必须先停止 MQTT Broker
- ✅ 确保其他从节点已配置新的主节点地址

**影响**：
- 失去 MQTT Broker 启动权限
- 如果 Broker 正在运行，其他节点将断开连接
- 需要重新配置连接到新的主节点

**操作流程**：
1. 停止 MQTT Broker
2. 通知其他从节点更新配置
3. 切换为从节点
4. 更新本地 `DbOption.toml` 的 `mqtt_host`

## 故障排查

### 问题 1：无法设置为主节点

**可能原因**：
- SQLite 数据库不可写
- 权限不足

**解决方法**：
1. 检查 `deployment_sites.sqlite` 文件权限
2. 查看后端日志确认错误信息
3. 手动修改数据库：
   ```sql
   UPDATE node_config SET is_master = 1 WHERE location = 'bj';
   ```

### 问题 2：主节点无法启动 Broker

**可能原因**：
- 端口 1883 被占用
- rumqttd 未正确安装

**解决方法**：
1. 检查端口占用：
   ```bash
   netstat -an | findstr 1883  # Windows
   lsof -i :1883               # Linux
   ```
2. 更改端口配置
3. 确认 MQTT Server 功能可用

### 问题 3：从节点无法连接主节点

**可能原因**：
- 主节点 Broker 未启动
- 网络不通
- 防火墙阻止

**解决方法**：
1. 确认主节点 Broker 状态为 "运行中"
2. 测试网络连接：
   ```bash
   telnet 192.168.1.100 1883
   ```
3. 检查防火墙规则：
   ```bash
   # Windows
   netsh advfirewall firewall add rule name="MQTT" protocol=TCP dir=in localport=1883 action=allow

   # Linux
   sudo ufw allow 1883/tcp
   ```

### 问题 4：角色切换后界面未更新

**解决方法**：
1. 点击 [刷新] 按钮手动刷新
2. 清空浏览器缓存
3. 重新登录 Web 界面

## 安全建议

1. **访问控制**：
   - 为 MQTT Broker 添加用户名/密码认证
   - 限制 Broker 只监听内网地址

2. **角色权限**：
   - 添加 API 身份验证
   - 限制角色切换操作的权限

3. **审计日志**：
   - 记录角色切换操作
   - 记录 Broker 启动/停止事件

4. **网络隔离**：
   - 使用 VPN 或专网连接远程节点
   - 避免 Broker 暴露在公网

## 性能考虑

### 主节点选择

**优选标准**：
- ✅ 网络稳定、带宽充足
- ✅ 硬件性能较高
- ✅ 24小时运行
- ✅ 位置适中（延迟最小化）

### Broker 性能

- **连接数**：根据从节点数量规划（建议 < 100）
- **消息吞吐**：根据增量更新频率调整
- **内存占用**：监控 Broker 进程内存使用

### 扩展性

**单主节点模式**：
- 适用于 < 10 个从节点
- 简单、易维护

**多主节点模式**（未来扩展）：
- 适用于 > 10 个从节点
- 需要 Broker 集群和负载均衡

## 相关文档

- [MQTT_SUBSCRIPTION_CONTROL.md](MQTT_SUBSCRIPTION_CONTROL.md) - MQTT 订阅控制
- [MQTT_NODE_MONITORING.md](frontend/MQTT_NODE_MONITORING.md) - 节点监控详细文档
- [MQTT_MONITORING_SUMMARY.md](MQTT_MONITORING_SUMMARY.md) - 总体实现总结
- [REMOTE_SYNC_DEVELOPMENT_GUIDE.md](docs/REMOTE_SYNC_DEVELOPMENT_GUIDE.md) - 异地同步开发指南

## 更新日志

### 2025-11-20
- 初始版本发布
- 实现主从节点角色管理
- 添加角色切换 API
- 实现主节点 MQTT Broker 启动控制
- 前端界面集成完成
- 编译测试通过

---

**创建日期**：2025-11-20
**版本**：v1.0
**状态**：已完成开发，待用户测试
