# MQTT 架构澄清：主站点的双重角色

## 🎯 核心问题

**问题**: 主站点运行 MQTT Server，它自己还需要订阅吗？

**答案**: ✅ **是的！主站点也必须订阅自己的 MQTT Broker！**

---

## 📐 正确的架构模式

### 完整拓扑图

```
┌────────────────────────────────────────────────────────┐
│              主站点 (北京 BJ)                           │
│                                                         │
│  ┌──────────────────────────────────────────────────┐ │
│  │  Layer 1: MQTT Broker (rumqttd)                  │ │
│  │  • 监听端口: 0.0.0.0:1883                        │ │
│  │  • 角色: 消息中转中心                             │ │
│  │  • 接收所有站点发来的消息                         │ │
│  │  • 转发给所有订阅者（包括自己）                   │ │
│  └──────────────────┬───────────────────────────────┘ │
│                     │                                   │
│                     │ 内部回环 (127.0.0.1:1883)         │
│                     ↓                                   │
│  ┌──────────────────────────────────────────────────┐ │
│  │  Layer 2: MQTT Client (订阅者)                   │ │
│  │  • 连接到: 127.0.0.1:1883 (本地 Broker)          │ │
│  │  • 订阅主题: Sync/E3d                             │ │
│  │  • 接收: 所有站点的增量消息（包括自己发的）       │ │
│  │  • 发布: 本地增量更新到 Broker                    │ │
│  └──────────────────────────────────────────────────┘ │
│                                                         │
└─────────────────────────────────────────────────────────┘
           ▲                                  ▲
           │ 外部网络连接                     │
           │ (192.168.31.58:1883)             │
           │                                  │
┌──────────┴────────────┐      ┌─────────────┴───────────┐
│   从站点 (石家庄 SJZ)   │      │   从站点 (天津 TJ)       │
│                        │      │                         │
│  ┌──────────────────┐ │      │  ┌──────────────────┐  │
│  │  MQTT Client     │ │      │  │  MQTT Client     │  │
│  │  • 连接到 BJ     │ │      │  │  • 连接到 BJ     │  │
│  │  • 订阅 Sync/E3d │ │      │  │  • 订阅 Sync/E3d │  │
│  │  • 发布增量消息  │ │      │  │  • 发布增量消息  │  │
│  └──────────────────┘ │      │  └──────────────────┘  │
└───────────────────────┘      └─────────────────────────┘
```

---

## 🔑 关键概念

### 1. 主站点的双重角色

主站点（运行 MQTT Server 的节点）同时扮演两个角色：

#### 角色 A: MQTT Broker (服务器)
- **进程**: `rumqttd.exe`
- **监听**: `0.0.0.0:1883`（对外）
- **功能**:
  - 接收所有站点的连接
  - 接收所有发布的消息
  - 转发消息给所有订阅者

#### 角色 B: MQTT Client (订阅者)
- **进程**: web_server 内的 MQTT 客户端
- **连接**: `127.0.0.1:1883`（本地回环）
- **功能**:
  - 订阅 `Sync/E3d` 主题
  - 接收其他站点发来的消息
  - **也接收自己发布的消息**（通过 Broker 中转）
  - 发布本地增量更新

### 2. 为什么主站点也要订阅？

#### ✅ 原因 1: 消息一致性

所有站点（包括主站点）都应该收到相同的消息序列：

```
时间线:
t1: BJ 检测到增量 → 发布到 Broker → Broker 转发给所有订阅者（BJ, SJZ, TJ）
t2: SJZ 检测到增量 → 发布到 Broker → Broker 转发给所有订阅者（BJ, SJZ, TJ）
t3: TJ 检测到增量 → 发布到 Broker → Broker 转发给所有订阅者（BJ, SJZ, TJ）

结果: 所有站点都收到了 3 条消息，保持数据一致
```

#### ✅ 原因 2: 避免特殊处理

如果主站点不订阅，代码需要区分：
- ❌ 主站点: 直接处理本地更新
- ❌ 从站点: 通过 MQTT 接收更新

这样会导致代码逻辑复杂，增加维护成本。

#### ✅ 原因 3: 统一的消息处理流程

```rust
// 所有站点使用相同的代码路径
接收 MQTT 消息
  ↓
解析消息内容
  ↓
下载 CBA 文件
  ↓
应用增量更新
  ↓
更新数据库
```

---

## ⚙️ 配置说明

### 主站点配置 (北京 BJ)

**DbOption.toml**:
```toml
location = "bj"

# MQTT 客户端连接配置
# 主站点连接到本地 Broker
mqtt_host = "127.0.0.1"    # ✅ 连接本地 Broker
mqtt_port = 1883

# 文件服务器配置（用于其他站点访问）
file_server_host = "http://192.168.31.58:8080"  # 外部访问地址
```

**节点角色**:
```sql
-- deployment_sites.sqlite::node_config
INSERT INTO node_config VALUES ('bj', 1, '2025-11-20 10:00:00');
-- is_master = 1 表示主节点
```

### 从站点配置 (石家庄 SJZ)

**DbOption.toml**:
```toml
location = "sjz"

# MQTT 客户端连接配置
# 从站点连接到主站点 Broker
mqtt_host = "192.168.31.58"  # ✅ 主站点外网地址
mqtt_port = 1883

# 文件服务器配置（本地服务）
file_server_host = "http://localhost:8080"
```

**节点角色**:
```sql
-- deployment_sites.sqlite::node_config
INSERT INTO node_config VALUES ('sjz', 0, '2025-11-20 10:00:00');
-- is_master = 0 表示从节点
```

---

## 📊 消息流转示例

### 场景 1: 主站点 (BJ) 发送增量

```
1. BJ 检测到增量更新
   ↓
2. BJ MQTT Client 发布消息到 127.0.0.1:1883
   主题: Sync/E3d
   ↓
3. BJ Broker 接收消息
   ↓
4. BJ Broker 转发给所有订阅者:
   • BJ MQTT Client (127.0.0.1:1883) ✅
   • SJZ MQTT Client (192.168.31.58:1883) ✅
   • TJ MQTT Client (192.168.31.58:1883) ✅
   ↓
5. 所有站点（包括 BJ）都收到消息并应用更新
```

**关键**: BJ 也收到了自己发布的消息，通过 Broker 中转！

### 场景 2: 从站点 (SJZ) 发送增量

```
1. SJZ 检测到增量更新
   ↓
2. SJZ MQTT Client 发布消息到 192.168.31.58:1883
   主题: Sync/E3d
   ↓
3. BJ Broker 接收消息
   ↓
4. BJ Broker 转发给所有订阅者:
   • BJ MQTT Client (127.0.0.1:1883) ✅
   • SJZ MQTT Client (192.168.31.58:1883) ✅
   • TJ MQTT Client (192.168.31.58:1883) ✅
   ↓
5. 所有站点（包括 BJ 和 SJZ 自己）都收到消息
```

**关键**: SJZ 也收到了自己发布的消息！

---

## 🔍 Web 界面监控

### 主站点界面显示

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
│ 在线节点: 3  离线节点: 0                             │
│  • BJ (本地) - 128 条消息                            │
│  • SJZ - 64 条消息                                   │
│  • TJ - 42 条消息                                    │
└─────────────────────────────────────────────────────┘
```

**关键观察**:
- ✅ Broker 监听 `0.0.0.0:1883`（对外服务）
- ✅ 订阅连接到 `127.0.0.1:1883`（本地回环）
- ✅ 能看到自己的节点也在订阅列表中

### 从站点界面显示

```
┌─────────────────────────────────────────────────────┐
│ MQTT 节点实时监控                                    │
├─────────────────────────────────────────────────────┤
│ 角色: 从节点 👥                                       │
│                                                      │
│ MQTT 订阅状态:                                       │
│  ● 订阅中 (连接到 192.168.31.58:1883) [停止订阅]   │
│                                                      │
│ 在线节点: 3  离线节点: 0                             │
│  • BJ - 128 条消息                                   │
│  • SJZ (本地) - 64 条消息                            │
│  • TJ - 42 条消息                                    │
└─────────────────────────────────────────────────────┘
```

**关键观察**:
- ✅ 没有 "MQTT Server 状态" 区域（从节点不能启动 Broker）
- ✅ 订阅连接到主站点 `192.168.31.58:1883`
- ✅ 能看到自己的节点也在订阅列表中

---

## 🛠️ 实现细节

### 代码位置

#### MQTT Client 创建

**文件**: [src/mqtt_service/mod.rs:117](src/mqtt_service/mod.rs#L117)

```rust
pub fn new_mqtt_instance_from_global_setting(id: &str) -> MqttInstance {
    let db_option = get_db_option();

    // 创建 MQTT 连接
    // 主站点: mqtt_host = "127.0.0.1"
    // 从站点: mqtt_host = "192.168.31.58"
    let mut mqttoptions = MqttOptions::new(
        id,
        db_option.mqtt_host.as_str(),  // ✅ 从配置读取
        db_option.mqtt_port
    );

    mqttoptions.set_clean_session(false);
    mqttoptions.set_keep_alive(Duration::from_secs(500));

    let (client, el) = AsyncClient::new(mqttoptions, 5000);

    MqttInstance { client, el }
}
```

#### MQTT Broker 启动

**文件**: [src/web_server/sync_control_center.rs:1237](src/web_server/sync_control_center.rs#L1237)

```rust
pub async fn start_mqtt_server(port: u16) -> anyhow::Result<()> {
    // 启动 rumqttd 进程
    let mut child = Command::new("rumqttd.exe")
        .arg("-c")
        .arg(&config_path)  // remote-test-dir/test-real/rumqttd.toml
        .arg("-vv")
        .spawn()?;

    // 配置文件中: listen = "0.0.0.0:1883"
    // 这样其他站点可以通过外网 IP 连接

    // 保存进程句柄
    *MQTT_SERVER_PROCESS.write().await = Some(child);

    log::info!("✅ MQTT 服务器已启动在端口 {}", port);
    Ok(())
}
```

#### rumqttd 配置

**文件**: [remote-test-dir/test-real/rumqttd.toml](remote-test-dir/test-real/rumqttd.toml)

```toml
[v4.1]
name = "v4-1"
listen = "0.0.0.0:1883"  # ✅ 监听所有网络接口
next_connection_delay_ms = 1

    [v4.1.connections]
    connection_timeout_ms = 60000
    max_payload_size = 20480
    max_inflight_count = 100
    dynamic_filters = true
```

**关键**: `listen = "0.0.0.0:1883"` 确保：
- 本地可以通过 `127.0.0.1:1883` 连接
- 外部可以通过 `192.168.31.58:1883` 连接

---

## ✅ 配置检查清单

### 主站点 (BJ) 配置检查

- [ ] **DbOption.toml**:
  - [ ] `location = "bj"`
  - [ ] `mqtt_host = "127.0.0.1"` ✅ 连接本地
  - [ ] `mqtt_port = 1883`
  - [ ] `file_server_host` 配置为外部可访问地址

- [ ] **节点角色**:
  - [ ] 在 Web 界面设置为 "主节点"
  - [ ] 或手动修改数据库: `UPDATE node_config SET is_master = 1 WHERE location = 'bj';`

- [ ] **rumqttd 配置**:
  - [ ] `remote-test-dir/test-real/rumqttd.toml` 存在
  - [ ] `listen = "0.0.0.0:1883"`

- [ ] **防火墙**:
  - [ ] 开放端口 1883（对内网）
  - [ ] `netsh advfirewall firewall add rule name="MQTT" protocol=TCP dir=in localport=1883 action=allow`

### 从站点 (SJZ) 配置检查

- [ ] **DbOption.toml**:
  - [ ] `location = "sjz"`
  - [ ] `mqtt_host = "192.168.31.58"` ✅ 主站点外网 IP
  - [ ] `mqtt_port = 1883`

- [ ] **节点角色**:
  - [ ] 确认为 "从节点" (默认)

- [ ] **网络连通性**:
  - [ ] 能 ping 通主站点: `ping 192.168.31.58`
  - [ ] 能连接 MQTT 端口: `telnet 192.168.31.58 1883`

---

## 🧪 测试验证

### 步骤 1: 启动主站点

1. **启动 Web 服务器**:
   ```bash
   cd d:\work\plant\web-server
   cargo run --bin web_server --features web_server
   ```

2. **打开浏览器**: `http://localhost:8080`

3. **设置主节点**:
   - 导航到 "MQTT 节点监控"
   - 点击 👑 按钮设为主节点

4. **启动 MQTT Broker**:
   - 点击 "启动 Broker" 按钮
   - 验证状态: "Broker运行中"

5. **启动订阅**:
   - 点击 "启动订阅" 按钮
   - 验证状态: "订阅中"

6. **检查连接**:
   ```bash
   # 检查 Broker 监听
   netstat -an | findstr "1883"
   # 应该看到 LISTENING

   # 检查本地订阅连接
   netstat -an | findstr "1883" | findstr "ESTABLISHED"
   # 应该看到至少 1 个连接（主站点自己）
   ```

### 步骤 2: 连接从站点

在从站点机器上：

1. **配置 DbOption.toml**:
   ```toml
   mqtt_host = "192.168.31.58"  # 主站点 IP
   mqtt_port = 1883
   location = "sjz"
   ```

2. **启动订阅**:
   ```bash
   cargo run --bin web_server --features web_server
   ```

   在 Web 界面点击 "启动订阅"

3. **验证连接**:
   - 从站点: 状态显示 "订阅中"
   - 主站点: 在线节点数量 +1

### 步骤 3: 测试消息流转

1. **在主站点触发增量更新**:
   - 修改 PDMS 数据
   - 观察 Web 界面 "消息投递状态"

2. **验证所有站点都收到消息**:
   - BJ (主站点): 能看到自己发的消息 ✅
   - SJZ (从站点): 收到 BJ 的消息 ✅

3. **在从站点触发增量更新**:
   - 在 SJZ 修改数据
   - 观察所有站点是否都收到

4. **验证主站点收到从站点消息**:
   - BJ: 能看到 SJZ 发的消息 ✅
   - SJZ: 也能看到自己发的消息 ✅

---

## 📖 常见误解

### ❌ 误解 1: 主站点不需要订阅

**错误理解**:
> "主站点运行了 Broker，它应该直接处理消息，不需要订阅"

**正确理解**:
> Broker 只负责转发消息，不处理业务逻辑。主站点的应用程序需要作为客户端订阅，才能接收和处理消息。

### ❌ 误解 2: 主站点订阅会收到重复消息

**错误理解**:
> "主站点自己发的消息，又订阅自己的 Broker，会不会收到重复？"

**正确理解**:
> 这不是重复，这是设计如此。所有站点（包括发送者）都应该收到相同的消息序列，确保数据一致性。

### ❌ 误解 3: 主站点应该连接外网 IP

**错误理解**:
> 主站点的 `mqtt_host` 应该配置为 `192.168.31.58`（外网地址）

**正确理解**:
> 主站点应该连接 `127.0.0.1`（本地回环），避免不必要的网络开销，提高性能。

---

## 🎯 总结

### 核心原则

1. **✅ 主站点 = Broker + Client**
   - Broker 对外服务（`0.0.0.0:1883`）
   - Client 订阅本地（`127.0.0.1:1883`）

2. **✅ 所有站点都订阅**
   - 主站点订阅本地 Broker
   - 从站点订阅主站点 Broker
   - 确保消息一致性

3. **✅ 统一的消息处理**
   - 所有站点使用相同的代码路径
   - 简化开发和维护
   - 降低出错风险

### 配置要点

| 站点类型 | mqtt_host | Broker 状态 | 订阅状态 |
|---------|-----------|------------|---------|
| 主站点 | `127.0.0.1` | ✅ 运行中 | ✅ 订阅中 |
| 从站点 | `192.168.31.58` (主站点 IP) | ❌ 无 | ✅ 订阅中 |

---

**创建日期**: 2025-11-20
**版本**: v1.0
**状态**: 架构澄清文档
