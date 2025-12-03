# MQTT 订阅操作指南

## 🎯 目标

启动 MQTT Server 后，订阅到 MQTT Server 以接收增量更新消息。

---

## 📋 完整操作步骤

### 方法 1: 通过 Web 界面（推荐）

这是最简单直观的方法！

#### 步骤 1: 启动 Web 服务器

```bash
cd d:\work\plant\web-server
cargo run --bin web_server --features web_server
```

等待编译完成，看到：
```
✅ Web server started on http://0.0.0.0:8080
```

#### 步骤 2: 打开 Web 界面

在浏览器中访问:
```
http://localhost:8080
```

#### 步骤 3: 导航到 MQTT 节点监控页面

在左侧菜单或导航栏找到：
```
MQTT 节点监控 / MQTT Node Monitor
```

#### 步骤 4: 设置主节点（如果需要）

如果当前显示 "从节点"，需要先设为主节点：

1. 查看角色区域：
   ```
   ┌────────────────────────┐
   │ 角色: 从节点  👥        │
   └────────────────────────┘
   ```

2. 点击 **👑 图标**（设为主节点）

3. 确认对话框：
   ```
   确定要将当前节点设为主节点吗？
   主节点可以启动 MQTT Broker。
   [确定] [取消]
   ```

4. 点击 **[确定]**

5. 等待成功提示：
   ```
   ✅ 节点 bj 已设置为主节点
   ```

6. 界面刷新，显示：
   ```
   ┌────────────────────────┐
   │ 角色: 主节点  👑        │
   └────────────────────────┘
   ```

#### 步骤 5: 启动 MQTT Broker

在 "MQTT Server 状态" 区域：

1. 当前状态：
   ```
   ┌──────────────────────────────────────┐
   │  ○Broker未启动   [启动Broker]        │
   └──────────────────────────────────────┘
   ```

2. 点击 **[启动Broker]** 按钮

3. 等待成功提示：
   ```
   ✅ MQTT服务器已启动在端口 1883
   ```

4. 状态变为：
   ```
   ┌──────────────────────────────────────┐
   │  ●Broker运行中   [停止Broker]        │
   └──────────────────────────────────────┘
   ```

#### 步骤 6: 启动 MQTT 订阅 ✅

在 "MQTT 订阅状态" 区域：

1. 当前状态：
   ```
   ┌──────────────────────────────────────┐
   │  ○未订阅   [启动订阅]                │
   └──────────────────────────────────────┘
   ```

2. 点击 **[启动订阅]** 按钮

3. 等待成功提示：
   ```
   ✅ MQTT 订阅已启动，当前节点已成为主节点
   ```

4. 状态变为：
   ```
   ┌──────────────────────────────────────┐
   │  ●订阅中   [停止订阅]                │
   └──────────────────────────────────────┘
   ```

#### 步骤 7: 验证订阅成功

**界面显示**:
- ✅ Broker 运行中（紫色脉冲 ●）
- ✅ 订阅中（绿色脉冲 ●）
- ✅ 在线节点数量显示（至少有当前节点）

**后台日志**（控制台输出）:
```
✅ MQTT 服务器已启动在端口 1883
🔌 正在连接 MQTT broker: 127.0.0.1:1883
✅ MQTT 订阅已启动
📡 已订阅主题: Sync/E3d
```

**系统验证**:
```bash
# 检查 MQTT Broker 监听
netstat -an | findstr "1883"
# 应该看到:
# TCP    0.0.0.0:1883           0.0.0.0:0              LISTENING
# TCP    127.0.0.1:1883         127.0.0.1:xxxxx        ESTABLISHED

# 检查 rumqttd 进程
tasklist | findstr rumqttd
# 应该看到:
# rumqttd.exe      12345 Console     1     25,000 K
```

---

### 方法 2: 通过 API 调用

如果你想通过脚本或命令行操作：

#### 启动 MQTT Broker

```bash
curl -X POST http://localhost:8080/api/sync/mqtt/start \
  -H "Content-Type: application/json" \
  -d '{"port": 1883}'
```

**响应**:
```json
{
  "status": "success",
  "message": "MQTT服务器已启动在端口 1883",
  "port": 1883
}
```

#### 启动 MQTT 订阅

```bash
curl -X POST http://localhost:8080/api/mqtt/subscription/start \
  -H "Content-Type: application/json"
```

**响应**:
```json
{
  "status": "success",
  "message": "MQTT 订阅已启动，当前节点已成为主节点"
}
```

#### 查询订阅状态

```bash
curl http://localhost:8080/api/mqtt/subscription/status
```

**响应**:
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

---

## 🔍 订阅功能详解

### 后端实现

**API 端点**: `POST /api/mqtt/subscription/start`

**Handler 位置**: [src/web_server/sync_control_handlers.rs:468](src/web_server/sync_control_handlers.rs#L468)

**核心逻辑**:

```rust
pub async fn start_mqtt_subscription_api() {
    // 1. 检查是否已经订阅
    if REMOTE_RUNTIME.read().await.is_some() {
        return error("已经在运行中");
    }

    // 2. 启动运行时（包括 MQTT 订阅和文件监听）
    start_runtime("default".to_string()).await?;

    // 3. 注册节点到监控系统
    mqtt_monitor_handlers::update_node_heartbeat(
        location, node_name, vec!["Sync/E3d"]
    ).await;

    // 4. 返回成功
    Ok("MQTT 订阅已启动")
}
```

**启动流程** ([src/web_server/remote_runtime.rs:31](src/web_server/remote_runtime.rs#L31)):

```rust
pub async fn start_runtime(env_id: String) -> anyhow::Result<()> {
    // 1. 初始化数据库管理器
    let mgr = Arc::new(AiosDBManager::init_form_config().await?);
    mgr.init_watcher().await.ok();

    // 2. 启动文件监听任务（监控 PDMS 文件变化）
    let watcher_handle = tokio::spawn(async move {
        mgr_clone.async_watch().await;
    });

    // 3. 启动 MQTT 订阅任务（接收远程增量消息）
    let mqtt_handle = tokio::spawn(async move {
        AiosDBManager::poll_sync_e3d_mqtt_events_with_backoff(
            watcher_arc, init_ms, max_ms
        ).await;
    });

    // 4. 保存运行时状态
    *REMOTE_RUNTIME.write().await = Some(RuntimeState {
        env_id,
        mgr,
        watcher_handle: Some(watcher_handle),
        mqtt_handle: Some(mqtt_handle),
    });

    Ok(())
}
```

### 订阅的内容

当你启动订阅后，系统会：

1. **连接到 MQTT Broker**:
   - 主站点: 连接到 `127.0.0.1:1883`
   - 从站点: 连接到主站点外网地址（如 `192.168.31.58:1883`）

2. **订阅主题**:
   ```
   Sync/E3d
   ```

3. **接收消息**:
   - 其他站点发送的增量更新
   - 自己站点发送的增量更新（通过 Broker 中转）

4. **处理消息**:
   - 解析消息内容（文件名、hash、会话范围等）
   - 下载 CBA 压缩包
   - 应用增量更新到本地数据库
   - 更新监控状态

### 订阅配置来源

MQTT 连接配置来自 `DbOption.toml`:

```toml
# MQTT Broker 地址
mqtt_host = "127.0.0.1"  # 主站点连接本地
# mqtt_host = "192.168.31.58"  # 从站点连接主站点

# MQTT Broker 端口
mqtt_port = 1883

# 站点标识
location = "bj"

# 项目代码（用于生成唯一的客户端 ID）
project_code = "1112"
```

**客户端 ID 生成**:
```rust
let client_id = format!("{}-{}", location, project_code);
// 例如: "bj-1112"
```

---

## ✅ 验证订阅成功

### 1. Web 界面检查

**主站点界面**:
```
┌─────────────────────────────────────────────────────┐
│ MQTT 节点实时监控                                    │
├─────────────────────────────────────────────────────┤
│ 角色: 主节点 👑                                       │
│                                                      │
│ MQTT Server 状态:                                    │
│  ● Broker运行中 (0.0.0.0:1883)  [停止Broker]       │
│                                                      │
│ MQTT 订阅状态:                                       │
│  ● 订阅中 (连接到 127.0.0.1:1883) [停止订阅]       │
│                                                      │
│ 在线节点: 1  离线节点: 0                             │
│  • BJ (本地) - 0 条消息                              │
└─────────────────────────────────────────────────────┘
```

**关键指标**:
- ✅ Broker 运行中（紫色 ●）
- ✅ 订阅中（绿色 ●）
- ✅ 在线节点至少为 1（当前节点）

### 2. 网络连接检查

```bash
# 检查 MQTT Broker 监听
netstat -an | findstr "1883"
```

**期望输出**:
```
TCP    0.0.0.0:1883           0.0.0.0:0              LISTENING
TCP    127.0.0.1:1883         127.0.0.1:xxxxx        ESTABLISHED
```

**说明**:
- `LISTENING`: Broker 在监听端口 1883
- `ESTABLISHED`: 本地客户端已连接（127.0.0.1 → 127.0.0.1）

### 3. 进程检查

```bash
# 检查 rumqttd 进程
tasklist | findstr rumqttd

# 检查 web_server 进程
tasklist | findstr web_server
```

**期望输出**:
```
rumqttd.exe      12345 Console     1     25,000 K
web_server.exe   67890 Console     1    150,000 K
```

### 4. 日志检查

**Web 服务器控制台输出**:
```
[INFO] ✅ MQTT 服务器已启动在端口 1883
[INFO] 🔌 正在连接 MQTT broker: 127.0.0.1:1883
[INFO] ✅ MQTT 连接成功
[INFO] 📡 已订阅主题: Sync/E3d
[INFO] ✅ MQTT 订阅已启动
[INFO] 🎯 节点 bj-1112 已注册到监控系统
```

### 5. 测试消息接收

#### 方法 A: 使用 mosquitto 发送测试消息

```bash
# 发布测试消息
mosquitto_pub -h 127.0.0.1 -p 1883 -t "Sync/E3d" -m '{"test": "message"}'
```

**验证**:
- 查看 Web 服务器日志，应该显示接收到消息
- 查看 Web 界面 "消息投递状态"，应该有新消息

#### 方法 B: 触发真实的增量更新

1. 修改 PDMS 数据文件
2. 等待文件监听检测到变化
3. 系统自动生成 CBA 并发布到 MQTT
4. 订阅者（包括自己）接收并处理

---

## 🐛 常见问题

### 问题 1: 点击"启动订阅"没有反应

**可能原因**:
- MQTT Broker 未启动
- 配置文件错误

**解决方法**:

1. 先启动 MQTT Broker:
   ```
   点击 [启动Broker] 按钮
   ```

2. 检查配置文件 `DbOption.toml`:
   ```toml
   mqtt_host = "127.0.0.1"  # 确认地址正确
   mqtt_port = 1883
   ```

3. 查看浏览器控制台错误信息（F12）

### 问题 2: 订阅后立即断开

**可能原因**:
- MQTT Broker 未运行
- 网络不通

**解决方法**:

```bash
# 检查 Broker 是否运行
netstat -an | findstr "1883"

# 如果没有监听，手动启动
cd remote-test-dir\test-real
rumqttd.exe -c rumqttd.toml -vv
```

### 问题 3: "MQTT 订阅已经在运行中"

**原因**: 之前启动的订阅还在运行

**解决方法**:

1. 先停止现有订阅:
   ```
   点击 [停止订阅] 按钮
   ```

2. 等待 1-2 秒

3. 再次点击 [启动订阅]

### 问题 4: 连接到主站点失败（从站点）

**可能原因**:
- 主站点 IP 地址错误
- 防火墙阻止
- 主站点 Broker 未启动

**解决方法**:

1. **验证网络连通性**:
   ```bash
   ping 192.168.31.58
   telnet 192.168.31.58 1883
   ```

2. **检查主站点防火墙**:
   ```bash
   # Windows
   netsh advfirewall firewall add rule name="MQTT" protocol=TCP dir=in localport=1883 action=allow

   # Linux
   sudo ufw allow 1883/tcp
   ```

3. **确认主站点 Broker 监听**:
   ```bash
   # 在主站点执行
   netstat -an | findstr "1883"
   # 应该看到: 0.0.0.0:1883 LISTENING
   ```

### 问题 5: 订阅成功但收不到消息

**可能原因**:
- 订阅的主题不匹配
- 消息格式不正确

**解决方法**:

1. **确认订阅主题**:
   ```
   主题应该是: Sync/E3d
   ```

2. **测试消息接收**:
   ```bash
   mosquitto_pub -h 127.0.0.1 -p 1883 -t "Sync/E3d" -m "test"
   ```

3. **查看日志输出**，确认消息被接收

---

## 📊 订阅状态图

```
未启动
  ↓
点击 [启动订阅]
  ↓
连接中 (connecting)
  ↓
├─ 成功 → 订阅中 (subscribed) ✅
│         ↓
│    接收消息
│    处理更新
│         ↓
│    点击 [停止订阅]
│         ↓
│    已停止 (stopped)
│
└─ 失败 → 错误状态 (error) ❌
         ↓
    查看错误日志
    解决问题
         ↓
    重新启动
```

---

## 🎯 快速参考

### Web 界面操作

| 操作 | 位置 | 按钮 | 结果 |
|------|------|------|------|
| 启动 Broker | MQTT Server 状态 | [启动Broker] | Broker 运行中 ●
| 启动订阅 | MQTT 订阅状态 | [启动订阅] | 订阅中 ● |
| 停止订阅 | MQTT 订阅状态 | [停止订阅] | 未订阅 ○ |
| 停止 Broker | MQTT Server 状态 | [停止Broker] | Broker未启动 ○ |

### API 端点

| 操作 | 方法 | 端点 | 请求体 |
|------|------|------|--------|
| 启动订阅 | POST | `/api/mqtt/subscription/start` | - |
| 停止订阅 | POST | `/api/mqtt/subscription/stop` | - |
| 查询状态 | GET | `/api/mqtt/subscription/status` | - |
| 启动 Broker | POST | `/api/sync/mqtt/start` | `{"port": 1883}` |
| 停止 Broker | POST | `/api/sync/mqtt/stop` | - |

### 配置文件

```toml
# DbOption.toml

# 主站点
mqtt_host = "127.0.0.1"

# 从站点
mqtt_host = "192.168.31.58"  # 改为主站点 IP

# 通用配置
mqtt_port = 1883
location = "bj"
project_code = "1112"
```

---

## 📚 相关文档

- [MQTT_ARCHITECTURE_CLARIFICATION.md](MQTT_ARCHITECTURE_CLARIFICATION.md) - 架构说明
- [MQTT_INTEGRATION_SUMMARY.md](MQTT_INTEGRATION_SUMMARY.md) - 集成总结
- [MQTT_WEB_CONTROL.md](MQTT_WEB_CONTROL.md) - Web 控制详解
- [MQTT_QUICK_START.md](MQTT_QUICK_START.md) - 快速开始

---

**创建日期**: 2025-11-20
**版本**: v1.0
**状态**: 订阅操作完整指南
