# MQTT 节点实时监控功能

## 功能概述

新增了 MQTT 节点实时监控功能，用于追踪和管理异地协同环境中的 MQTT 节点订阅状态、消息投递情况，实现全链路可观测性。

## 核心功能

### 1. 节点状态监控
- **在线/离线检测**：实时显示各节点的在线状态（30秒心跳超时）
- **订阅主题追踪**：显示每个节点订阅的 MQTT 主题列表
- **消息统计**：记录每个节点接收的消息总数
- **时间追踪**：显示最后心跳时间、最后接收消息时间、连接时间

### 2. 消息投递追踪
- **发送记录**：追踪每条 MQTT 消息的发送方和发送时间
- **接收状态**：显示哪些节点已接收、哪些未接收
- **处理状态**：追踪消息处理进度（Pending → Received → Processing → Completed/Failed）
- **可视化展示**：用颜色区分不同的投递状态

### 3. 实时更新
- **自动刷新**：每 5 秒自动刷新节点和消息状态
- **心跳监控**：自动检测超过 30 秒无心跳的节点并标记为离线
- **消息队列管理**：保留最近 1000 条消息记录，自动清理旧数据

## 使用指南

### 访问方式
1. 启动 Web 界面
2. 点击左侧导航栏的 **"MQTT 节点"** 按钮
3. 查看节点列表和消息投递状态

### 界面布局

#### 左侧：节点列表
- 显示所有订阅了 MQTT 的节点
- 绿色圆点 + 脉冲效果 = 在线节点
- 红色圆点 = 离线节点
- 显示节点位置（location）、消息接收数量、最后心跳时间
- 点击节点可查看详细信息

#### 右侧：消息投递状态
- 显示与选中节点相关的消息投递记录
- 发送方信息、会话范围、文件数量
- 接收者列表及其接收状态：
  - ⏱️ **灰色**：等待接收（Pending）
  - 📧 **蓝色**：已接收（Received）
  - ⚙️ **蓝色 + 旋转**：处理中（Processing）
  - ✅ **绿色**：已完成（Completed）
  - ❌ **红色**：失败（Failed）

## 技术架构

### 后端实现

#### 数据结构
```rust
// MQTT 节点状态
pub struct MqttNodeStatus {
    pub location: String,           // 位置标识（如 "bj", "sjz"）
    pub node_name: String,           // 节点名称
    pub is_online: bool,             // 是否在线
    pub last_heartbeat: DateTime<Utc>, // 最后心跳时间
    pub subscribed_topics: Vec<String>, // 订阅主题
    pub messages_received: u64,      // 接收消息总数
    pub last_message_time: Option<DateTime<Utc>>, // 最后接收消息时间
    pub connected_at: DateTime<Utc>, // 连接时间
}

// 消息投递状态
pub struct MessageDeliveryStatus {
    pub message_id: String,          // 消息 ID
    pub sender_location: String,     // 发送者位置
    pub sent_at: DateTime<Utc>,      // 发送时间
    pub session_range: Option<String>, // 会话范围
    pub receivers: Vec<ReceiverStatus>, // 接收者列表
    pub file_count: usize,           // 文件数量
}

// 接收者状态
pub struct ReceiverStatus {
    pub location: String,            // 接收者位置
    pub received: bool,              // 是否已接收
    pub received_at: Option<DateTime<Utc>>, // 接收时间
    pub status: ReceiverProcessStatus, // 处理状态
}
```

#### API 端点
| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/mqtt/nodes` | GET | 获取所有 MQTT 节点状态 |
| `/api/mqtt/messages` | GET | 获取消息投递状态列表（支持 limit 参数） |
| `/api/mqtt/messages/:id` | GET | 获取特定消息的投递详情 |

#### 核心模块
- **mqtt_monitor_handlers.rs**：API 处理器和状态管理
- **increment_manager.rs**：MQTT 消息发送时记录状态
- **db_model.rs**：MQTT 消息接收时记录状态

### 前端实现

#### 组件
- **MqttNodeMonitor.vue**：主监控组件
  - 位置：`frontend/src/components/views/MqttNodeMonitor.vue`
  - 功能：节点列表、消息投递追踪、实时刷新

#### 数据流
```
用户访问 MQTT 节点页面
  ↓
组件挂载，调用 loadData()
  ↓
并行请求 /api/mqtt/nodes 和 /api/mqtt/messages
  ↓
渲染节点列表和消息状态
  ↓
每 5 秒自动刷新
```

## 集成说明

### 后端集成

1. **注册路由**（已完成）
   - 在 `src/web_server/mod.rs` 中添加 MQTT 监控路由

2. **发送消息时记录**
   - 在 `src/data_interface/increment_manager.rs::publish_sync_payload_with_retry()` 中
   - 成功发送后调用 `record_message_sent()`

3. **接收消息时记录**
   - 在 `src/data_interface/db_model.rs::poll_sync_e3d_mqtt_events()` 中
   - 接收到消息后调用 `record_message_received()`

### 前端集成

1. **导航菜单**（已完成）
   - 在 App.vue 侧边栏添加 "MQTT 节点" 按钮

2. **视图路由**（已完成）
   - 添加 `mqtt-nodes` 视图分支

3. **组件导入**（已完成）
   - 导入 `MqttNodeMonitor.vue` 组件

## 待完善功能

### 1. 预期接收者列表配置
目前 `record_message_sent()` 的 `expected_receivers` 参数为空数组。需要：
- 从 `deployment_sites.sqlite` 或远程环境配置读取
- 根据当前环境的拓扑关系确定接收者
- 建议添加到 `DbOption.toml` 配置或数据库

示例配置：
```toml
[[remote_nodes]]
location = "bj"
mqtt_host = "192.168.1.10"
mqtt_port = 1883

[[remote_nodes]]
location = "sjz"
mqtt_host = "192.168.2.20"
mqtt_port = 1883
```

### 2. 心跳机制实现
当前节点状态是被动更新，需要主动心跳：
- 在 MQTT 客户端连接后定期发送心跳消息
- 调用 `update_node_heartbeat()` 更新状态
- 建议每 10 秒发送一次心跳

实现位置：`src/data_interface/db_model.rs::poll_sync_e3d_mqtt_events_with_backoff()`

### 3. WebSocket/SSE 实时推送
当前使用轮询（每 5 秒），可优化为：
- 使用 WebSocket 推送节点状态变化
- 使用 SSE 推送消息投递事件
- 减少网络请求，提高实时性

### 4. 消息重试和告警
- 检测长时间未接收的消息
- 自动重发机制
- 告警通知（邮件/webhook）

## 监控指标

系统自动统计以下指标：

- **节点总数**：所有注册的 MQTT 节点
- **在线节点数**：当前在线的节点
- **离线节点数**：超过 30 秒无心跳的节点
- **消息总数**：历史消息记录总数
- **已完成投递数**：所有接收者都已接收的消息数
- **待投递数**：还有节点未接收的消息数

## 故障排查

### 节点显示离线
1. 检查节点的 MQTT 客户端是否正常运行
2. 检查网络连接和 MQTT broker 状态
3. 查看节点日志确认是否有连接错误
4. 验证心跳机制是否正常工作

### 消息未被接收
1. 检查接收节点是否在线
2. 验证接收节点是否订阅了正确的主题（`Sync/E3d`）
3. 检查 MQTT QoS 设置（应为 ExactlyOnce）
4. 查看消息投递详情确认发送状态

### 数据不更新
1. 检查前端是否正常轮询（5 秒间隔）
2. 验证后端 API 返回正常
3. 检查浏览器控制台是否有错误
4. 清空缓存并刷新页面

## 性能优化

- **内存管理**：自动清理超过 1000 条的旧消息记录
- **查询优化**：使用内存缓存而非数据库查询
- **并发控制**：使用 RwLock 保护共享状态
- **批量更新**：心跳更新使用批量写入

## 安全考虑

- API 端点应添加身份验证（TODO）
- 敏感信息（如 MQTT 密码）不应在前端显示
- 限制历史记录查询数量防止 DOS

## 相关文件

### 后端
- [src/web_server/mqtt_monitor_handlers.rs](../src/web_server/mqtt_monitor_handlers.rs) - 主要 API 处理器
- [src/web_server/mod.rs](../src/web_server/mod.rs) - 路由注册
- [src/data_interface/increment_manager.rs](../src/data_interface/increment_manager.rs) - 发送端集成
- [src/data_interface/db_model.rs](../src/data_interface/db_model.rs) - 接收端集成

### 前端
- [frontend/src/components/views/MqttNodeMonitor.vue](src/components/views/MqttNodeMonitor.vue) - 主监控组件
- [frontend/src/App.vue](src/App.vue) - 路由集成

## 更新日志

### 2025-11-20
- 初始版本发布
- 实现节点状态监控
- 实现消息投递追踪
- 集成到主界面
- 添加自动刷新功能
