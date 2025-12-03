# HTTP 连接频繁请求问题分析

## 问题现象

终端日志显示频繁的 HTTP 连接请求：
```
[2025-11-28T16:38:46Z DEBUG reqwest::connect] starting new connection: http://localhost:8081/
[2025-11-28T16:38:46Z DEBUG hyper_util::client::legacy::connect::http] connecting to [::1]:8081
[2025-11-28T16:38:46Z DEBUG hyper_util::client::legacy::connect::http] connecting to 127.0.0.1:8081
[2025-11-28T16:38:48Z DEBUG reqwest::connect] starting new connection: http://localhost:8080/
[2025-11-28T16:38:48Z DEBUG hyper_util::client::legacy::connect::http] connecting to [::1]:8080
```

## 根本原因

### 1. 前端轮询机制

**位置**：`frontend/src/components/views/MqttNodeMonitorEnhanced.vue`

```javascript
onMounted(() => {
  loadData();
  refreshInterval = setInterval(loadData, 5000); // 每 5 秒轮询一次
});
```

`loadData()` 函数会调用 `/api/mqtt/nodes` API。

### 2. 后端频繁健康检查

**位置**：`src/web_server/mqtt_monitor_handlers.rs`

`get_mqtt_nodes_status()` 函数在每次调用时：
1. 获取所有部署站点列表
2. 对每个**未启动 MQTT 的站点**调用 `check_site_http_status()`
3. `check_site_http_status()` 每次都会：
   - 创建新的 `reqwest::Client`
   - 连接到站点的 `/api/health` 端点
   - 如果站点配置为 `localhost:8080` 或 `localhost:8081`，就会频繁连接这些地址

### 3. 问题链路

```
前端每 5 秒轮询
  ↓
GET /api/mqtt/nodes
  ↓
get_mqtt_nodes_status()
  ↓
对每个未启动 MQTT 的站点调用 check_site_http_status()
  ↓
创建新的 reqwest::Client → 连接 localhost:8080/8081
  ↓
DEBUG 日志：reqwest::connect, hyper_util::client::legacy::connect::http
```

## 影响

1. **资源浪费**：每次健康检查都创建新的 HTTP 客户端，无法复用 TCP 连接
2. **日志噪音**：DEBUG 级别的连接日志频繁打印，影响日志可读性
3. **性能影响**：如果站点数量多，每次 API 调用都会执行多次 HTTP 请求
4. **网络负载**：频繁的 HTTP 连接尝试增加网络负载

## 解决方案

### 已实施：健康检查结果缓存

**修改文件**：`src/web_server/mqtt_monitor_handlers.rs`

**实现**：
1. 添加 `HTTP_HEALTH_CACHE` 全局缓存，存储健康检查结果
2. `check_site_http_status()` 函数先检查缓存
3. 如果缓存未过期（30 秒内），直接返回缓存结果
4. 如果缓存过期或不存在，执行健康检查并更新缓存

**缓存策略**：
- **缓存有效期**：30 秒
- **缓存键**：站点 HTTP Host（去除尾部斜杠）
- **缓存值**：`{ is_online: bool, checked_at: DateTime<Utc> }`

**效果**：
- 前端每 5 秒轮询一次，但健康检查结果缓存 30 秒
- 实际健康检查频率从每 5 秒降低到每 30 秒
- 减少了 83% 的 HTTP 连接请求

## 进一步优化建议

### 1. 复用 HTTP 客户端（可选）

可以考虑创建一个全局的 `reqwest::Client` 实例，而不是每次创建新的：

```rust
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap()
});
```

**优点**：
- 复用 TCP 连接，减少连接开销
- 更好的性能

**缺点**：
- 需要确保客户端配置适用于所有站点
- 可能增加代码复杂度

### 2. 减少前端轮询频率（可选）

如果不需要实时性很高，可以将前端轮询间隔从 5 秒增加到 10-15 秒：

```javascript
refreshInterval = setInterval(loadData, 10000); // 改为 10 秒
```

### 3. 使用 SSE 替代轮询（已部分实施）

对于 MQTT 订阅状态，已经使用 SSE 替代轮询。可以考虑将节点状态也改为 SSE 推送。

## 验证方法

1. **查看日志**：启动服务后，观察 `reqwest::connect` 日志频率
2. **预期结果**：健康检查相关的连接日志应该每 30 秒出现一次（而不是每 5 秒）
3. **功能验证**：确保节点状态显示仍然正常，只是更新频率降低

## 相关文件

- `src/web_server/mqtt_monitor_handlers.rs` - 健康检查缓存实现
- `frontend/src/components/views/MqttNodeMonitorEnhanced.vue` - 前端轮询逻辑
- `docs/LOG_ANALYSIS.md` - 其他日志优化记录












