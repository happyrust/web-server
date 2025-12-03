# 拓扑可视化完整指南

## 概述

本文档详细说明了 MQTT 拓扑可视化功能的数据流、显示逻辑和故障排查方法。

## 数据源架构

拓扑可视化从三个数据源获取信息：

### 1. MQTT 节点状态 (`/api/mqtt/nodes`)

**数据来源**: 内存缓存 `MQTT_NODES`（由 MQTT 客户端实时更新）

**包含信息**:
- `location`: 节点位置标识
- `node_name`: 节点名称
- `is_online`: 在线状态（基于心跳超时）
- `is_master_node`: 是否为主节点
- `messages_received`: 接收消息计数
- `subscribed_topics`: 订阅的主题列表
- `broker_connected_pub`: 发布客户端连接状态
- `broker_connected_sub`: 订阅客户端连接状态
- `last_heartbeat`: 最后心跳时间

**特点**:
- ✅ 实时性高（5秒刷新）
- ✅ 包含运行时状态
- ❌ 不包含离线或未启动的节点

### 2. 拓扑配置 (`/api/remote-sync/topology`)

**数据来源**: SQLite 数据库 `deployment_sites.sqlite`

**包含信息**:
```json
{
  "environments": [
    {
      "id": "env-1",
      "name": "主节点-北京",
      "location": "bj",
      "mqtt_host": "localhost",
      "mqtt_port": 1883
    }
  ],
  "sites": [
    {
      "id": "site-1",
      "name": "站点-上海",
      "location": "sh",
      "env_id": "env-1"
    }
  ],
  "connections": [
    {
      "env_id": "env-1",
      "site_id": "site-1"
    }
  ]
}
```

**特点**:
- ✅ 包含所有配置的节点（包括离线节点）
- ✅ 定义主从关系（`env_id` 关联）
- ❌ 不包含实时状态

### 3. 消息投递记录 (`/api/mqtt/messages`)

**数据来源**: 内存缓存 `MQTT_MESSAGE_DELIVERY`

**包含信息**:
- `message_id`: 消息ID
- `sender_location`: 发送者位置
- `receivers`: 接收者列表
  - `location`: 接收者位置
  - `status`: 投递状态（pending/received/processing/completed/failed）
  - `received`: 是否已接收
  - `received_at`: 接收时间

**特点**:
- ✅ 用于显示活跃消息流动画
- ✅ 反映实时通信状态
- ❌ 仅保留最近的消息

## 节点显示逻辑

### 数据合并策略

```javascript
// 1. 优先使用 MQTT 实时状态
nodes.value.forEach(n => {
  nodeMap.set(n.location, { ...n });
});

// 2. 从拓扑配置补充环境节点（主节点）
topology.value.environments.forEach(env => {
  if (!nodeMap.has(env.location)) {
    // 添加离线主节点
    nodeMap.set(env.location, {
      location: env.location,
      node_name: env.name,
      is_online: false,
      is_master_node: true
    });
  } else {
    // 标记为主节点
    nodeMap.get(env.location).is_master_node = true;
  }
});

// 3. 从拓扑配置补充站点节点（从节点）
topology.value.sites.forEach(site => {
  // 类似逻辑
});
```

### 节点布局算法

**主节点布局**:
- 单个主节点: 放在圆心 `(centerX, centerY)`
- 多个主节点: 小圆环绕，半径 = `radius * 0.3`

**从节点布局**:
- 大圆环绕，半径 = `radius`
- 均匀分布，角度 = `(index * 2π) / clientNodes.length`

### 节点视觉编码

| 属性 | 视觉表现 | 含义 |
|------|---------|------|
| 主节点 | 紫色正方形 (#a855f7) + 圆角 | 环境节点，位于中心 |
| 从节点在线 | 绿色圆圈 (#10b981) + 脉冲动画 | 站点节点在线，环绕外圈 |
| 从节点离线 | 红色圆圈 (#ef4444) | 站点节点离线 |
| 发布客户端 | 左上角 P 徽章 | 绿色=已连接，红色=未连接 |
| 订阅客户端 | 右下角 S 徽章 | 绿色=已连接，红色=未连接 |
| 消息计数 | 右上角数字徽章 | 接收消息数量 |

## 连接关系显示逻辑

### 连接来源优先级

1. **拓扑配置连接** (最高优先级)
   - 基于 `connections` 数组
   - 反映配置的主从关系

2. **MQTT 动态连接**
   - 基于 `is_master_node` 标记
   - 主节点 → 所有从节点

3. **消息投递连接**
   - 基于 `messages` 数组
   - 用于更新活跃状态

### 连接状态判断

```javascript
const isSubscribed = siteNode.is_online && 
  (siteNode.broker_connected_sub === true || siteNode.has_mqtt_subscription);

const status = isSubscribed ? 'subscribed' : 
               (siteNode.is_online ? 'online' : 'offline');
```

### 连接视觉编码

| 状态 | 颜色 | 线型 | 箭头 | 标签 |
|------|------|------|------|------|
| 已订阅 | 绿色 (#10b981) | 实线 | 绿色箭头 | ✓ |
| 在线未订阅 | 蓝色 (#3b82f6) | 实线 | 蓝色箭头 | ○ |
| 离线 | 灰色 (#94a3b8) | 虚线 | 灰色箭头 | ✗ |
| 活跃消息流 | 黄色 (#fbbf24) | 流动虚线 | - | - |

## 故障排查

### 问题 1: 只显示一个节点

**可能原因**:
1. 拓扑配置为空
2. API 路由不匹配
3. 数据合并逻辑错误

**排查步骤**:
```bash
# 1. 检查拓扑配置
curl http://localhost:8080/api/remote-sync/topology | jq '.'

# 2. 检查 MQTT 节点
curl http://localhost:8080/api/mqtt/nodes | jq '.nodes | length'

# 3. 查看浏览器控制台日志
# 应该看到:
# 📡 加载 MQTT 节点: X 个节点
# 🗺️ 加载拓扑配置: { environments: A, sites: B }
# 📊 最终节点列表: N 个节点
```

**解决方案**:
- 确保 `deployment_sites.sqlite` 中有配置的环境和站点
- 确保前端使用正确的 API 路径 `/api/remote-sync/topology`
- 检查控制台日志确认数据加载成功

### 问题 2: 连接线不显示

**可能原因**:
1. 拓扑配置中缺少 `connections`
2. 节点位置不匹配
3. 连接计算逻辑错误

**排查步骤**:
```javascript
// 浏览器控制台
// 应该看到:
// 🔗 开始计算连接关系...
// 🌐 环境 bj 有 2 个站点
//   ├─ bj → sh: 已订阅
//   ├─ bj → gz: 离线
// ✅ 总共 2 条连接
```

**解决方案**:
- 确保站点的 `env_id` 正确关联到环境
- 确保节点的 `location` 字段一致
- 检查 `is_master_node` 标记是否正确

### 问题 3: 订阅状态不正确

**可能原因**:
1. MQTT 客户端未正确更新状态
2. 心跳超时设置不合理
3. `broker_connected_sub` 字段缺失

**排查步骤**:
```bash
# 检查节点详细信息
curl http://localhost:8080/api/mqtt/nodes | jq '.nodes[] | {location, is_online, broker_connected_sub, has_mqtt_subscription}'
```

**解决方案**:
- 确保 MQTT 客户端调用 `update_node_heartbeat()`
- 确保订阅成功后调用 `update_subscription_status(location, node_name, true)`
- 检查心跳超时设置（默认 30 秒）

## 开发调试技巧

### 1. 启用详细日志

前端已添加详细的控制台日志，打开浏览器控制台即可查看。

### 2. 模拟节点数据

```javascript
// 浏览器控制台
// 手动添加测试节点
fetch('/api/mqtt/nodes').then(r => r.json()).then(data => {
  console.log('当前节点:', data.nodes);
});
```

### 3. 检查数据库

```bash
# 查看拓扑配置
sqlite3 deployment_sites.sqlite "SELECT * FROM remote_sync_environments;"
sqlite3 deployment_sites.sqlite "SELECT * FROM remote_sync_sites;"
```

---

**文档版本**: v1.1  
**最后更新**: 2025-01-XX  
**相关文档**: 
- `MQTT_TOPOLOGY_VISUALIZATION.md` - 功能文档
- `TOPOLOGY_VISUALIZATION_FIX.md` - 修复说明

