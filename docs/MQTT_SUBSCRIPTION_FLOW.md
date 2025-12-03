# MQTT 订阅启动流程详解

## 概述

"启动订阅"功能用于启动 MQTT 客户端，连接到 MQTT Broker 并订阅消息。根据节点角色（主节点/从节点），行为有所不同。

## 主节点启动订阅

### 流程步骤

1. **检查状态**
   - 检查 `REMOTE_RUNTIME` 是否已有运行实例
   - 如果已运行，返回错误

2. **判断节点角色**
   - 从 `DbOption.toml` 读取 `location`
   - 查询 `node_config` 表判断是否为主节点
   - 主节点不需要选择其他主节点

3. **启动运行时** (`remote_runtime::start_runtime`)
   - **初始化数据库管理器** (`AiosDBManager`)
     - 从 `DbOption.toml` 读取配置
     - 初始化数据库连接
   
   - **启动文件监听器 (Watcher)**
     - 监听 PDMS 数据库文件变化（`.cdf` 文件）
     - 检测增量更新（session number 变化）
     - 生成增量更新文件（`.cba`）
   
   - **启动 MQTT 客户端**
     - **使用本地 MQTT 配置**（`DbOption.toml` 中的 `mqtt_host` 和 `mqtt_port`）
     - 连接到**本地 MQTT Broker**（如果已启动）
     - 订阅主题：`Sync/E3d`
     - 接收其他从节点发送的同步消息
     - 使用指数退避重连机制（初始 1 秒，最大 30 秒）

4. **注册到监控系统**
   - 发送心跳到 MQTT 监控系统
   - 节点名称：`{location}-{project_code}`
   - 订阅主题列表：`["Sync/E3d"]`

5. **发送状态事件**
   - 通过 SSE 广播 `MqttSubscriptionStatusChanged` 事件
   - 通知前端状态已更新

### 主节点的作用

- **作为 MQTT Broker**：提供 MQTT 服务器（需要单独启动）
- **接收同步消息**：接收从节点发送的增量更新通知
- **文件监控**：监控本地 PDMS 数据库变化
- **生成增量更新**：检测到变化时生成 `.cba` 文件

## 从节点启动订阅

### 流程步骤

1. **检查状态**
   - 检查 `REMOTE_RUNTIME` 是否已有运行实例
   - 如果已运行，返回错误

2. **判断节点角色**
   - 从 `DbOption.toml` 读取 `location`
   - 查询 `node_config` 表判断为从节点
   - **必须选择要订阅的主节点**

3. **保存主节点配置**
   - 从请求中获取主节点信息（`master_location` 或 `master_mqtt_host/master_mqtt_port`）
   - 更新 `remote_sync_sites` 表中的 `master_mqtt_host` 和 `master_mqtt_port`
   - 如果使用 `master_location`，从 `remote_sync_sites` 表查询主节点的 MQTT 配置

4. **启动运行时** (`remote_runtime::start_runtime`)
   - **初始化数据库管理器** (`AiosDBManager`)
     - 从 `DbOption.toml` 读取配置
     - 初始化数据库连接
   
   - **启动文件监听器 (Watcher)**
     - 监听 PDMS 数据库文件变化（`.cdf` 文件）
     - 检测增量更新（session number 变化）
     - 生成增量更新文件（`.cba`）
   
   - **启动 MQTT 客户端**
     - **使用主节点 MQTT 配置**（从 `remote_sync_sites` 表读取）
     - 连接到**主节点的 MQTT Broker**
     - 订阅主题：`Sync/E3d`
     - 接收主节点和其他从节点发送的同步消息
     - 使用指数退避重连机制（初始 1 秒，最大 30 秒）

5. **注册到监控系统**
   - 发送心跳到 MQTT 监控系统
   - 节点名称：`{location}-{project_code}`
   - 订阅主题列表：`["Sync/E3d"]`

6. **发送状态事件**
   - 通过 SSE 广播 `MqttSubscriptionStatusChanged` 事件
   - 通知前端状态已更新

### 从节点的作用

- **作为 MQTT 客户端**：连接到主节点的 MQTT Broker
- **接收同步消息**：接收主节点和其他从节点发送的增量更新通知
- **文件监控**：监控本地 PDMS 数据库变化
- **生成增量更新**：检测到变化时生成 `.cba` 文件
- **发送同步消息**：检测到变化时通过 MQTT 通知其他节点

## 关键差异对比

| 特性 | 主节点 | 从节点 |
|------|--------|--------|
| **MQTT 连接** | 连接到本地 Broker | 连接到主节点 Broker |
| **MQTT 配置来源** | `DbOption.toml` | `remote_sync_sites` 表 |
| **是否需要选择主节点** | ❌ 不需要 | ✅ 必须选择 |
| **启动参数** | 空对象 `{}` | 包含主节点信息 |
| **主要作用** | 提供 MQTT Broker | 订阅主节点消息 |

## 共同功能

无论主节点还是从节点，启动订阅后都会：

1. **文件监控 (Watcher)**
   - 监控 PDMS 数据库文件变化
   - 检测增量更新
   - 生成 `.cba` 增量更新文件

2. **MQTT 订阅**
   - 订阅 `Sync/E3d` 主题
   - 接收同步消息
   - 处理增量更新通知

3. **监控注册**
   - 发送心跳到监控系统
   - 更新节点在线状态

## 注意事项

1. **主节点需要先启动 MQTT Broker**
   - 主节点启动订阅时，MQTT Broker 应该已经运行
   - 可以通过 `/api/sync/mqtt/start` 启动 Broker

2. **从节点需要主节点在线**
   - 从节点启动订阅前，确保主节点的 MQTT Broker 已启动
   - 否则连接会失败，但会使用指数退避重连

3. **环境配置**
   - 使用 `env_id` 参数指定环境（默认 "default"）
   - 环境配置影响重连策略（`reconnect_initial_ms`, `reconnect_max_ms`）

4. **状态同步**
   - 状态变化通过 SSE 实时推送到前端
   - 前端无需轮询即可获得最新状态













