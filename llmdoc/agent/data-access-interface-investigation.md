# 监控仪表盘数据访问接口设计调查报告

## 调查摘要

本报告调查了aios-database项目中增量更新监控仪表盘所需的数据访问接口,重点关注增量更新历史记录、实时状态查询、失败任务统计和数据聚合需求。调查发现系统已具备完整的数据基础设施,但部分接口需要新增或优化以支持仪表盘的实时监控需求。

---

## Code Sections (证据列表)

### 1. 增量更新历史数据库

- `src/web_server/remote_sync_handlers.rs:142-204` (open_sqlite): 创建SQLite数据库连接,包含remote_sync_logs表的schema定义,支持增量更新历史记录存储
- `src/web_server/remote_sync_handlers.rs:184-204` (remote_sync_logs表结构): 包含id、task_id、env_id、source_env、target_site、site_id、direction、file_path、file_size、record_count、status、error_message、notes、started_at、completed_at、created_at、updated_at等字段
- `src/web_server/remote_sync_handlers.rs:206-212` (索引定义): 为env_id和status字段创建索引,优化查询性能

### 2. 日志查询接口

- `src/web_server/remote_sync_handlers.rs:853-949` (list_logs): 支持多维度查询参数(env_id、site_id、target_site、status、direction、start、end、keyword),返回分页结果和总数
- `src/web_server/remote_sync_handlers.rs:842-851` (LogQueryParams): 定义查询参数结构,包含limit、offset和多个过滤条件
- `src/web_server/remote_sync_handlers.rs:54-72` (RemoteSyncLogRecord): 日志记录数据结构,包含完整的任务元数据和状态信息

### 3. 失败任务队列系统

- `src/data_interface/failed_task_queue.rs:212-236` (FailedTaskQueue): 失败任务队列管理器,使用Arc<RwLock<Vec<FailedTask>>>内存存储,支持JSON持久化到assets/failed_tasks.json
- `src/data_interface/failed_task_queue.rs:285-300` (get_stats): 返回TaskQueueStats统计信息,包含total、pending、waiting、exhausted四个计数
- `src/data_interface/failed_task_queue.rs:269-277` (get_pending_tasks): 获取所有should_retry()为true的待重试任务列表
- `src/data_interface/failed_task_queue.rs:335-339` (get_exhausted_tasks): 获取已达到最大重试次数的失败任务列表
- `src/data_interface/failed_task_queue.rs:279-283` (get_all_tasks): 获取完整任务列表用于监控

### 4. 失败任务数据结构

- `src/data_interface/failed_task_queue.rs:100-139` (FailedTask): 包含id、task_type、error、retry_count、max_retries、first_failed_at、last_retry_at、next_retry_at、priority、metadata等字段
- `src/data_interface/failed_task_queue.rs:37-75` (FailedTaskType): 四种失败类型枚举 - DatabaseQuery、Compression、IncrementUpdate、MqttPublish
- `src/data_interface/failed_task_queue.rs:398-409` (TaskQueueStats): 统计结构,包含total、pending、waiting、exhausted计数

### 5. 实时同步状态

- `src/web_server/sync_control_center.rs:69-102` (SyncControlState): 同步控制状态结构,包含is_running、is_paused、current_env、mqtt_connected、watcher_active、total_synced、total_failed、pending_count、queue_size、sync_rate_mbps、avg_sync_time_ms、last_sync_time、started_at、uptime_seconds等字段
- `src/web_server/sync_control_center.rs:217-236` (SyncControlCenter): 全局同步控制中心,包含state、config、task_queue、running_tasks、history、mqtt_server、worker_handle、logs等字段
- `src/web_server/sync_control_center.rs:128-149` (SyncTask): 同步任务结构,包含id、file_path、file_size、file_name、file_hash、record_count、env_id、source_env、target_site、direction、notes、status、priority、retry_count、created_at、started_at、completed_at、error_message等字段

### 6. 日志系统

- `src/web_server/sync_control_center.rs:209-215` (LogEntry): 日志条目结构,包含timestamp、level、message
- `src/web_server/sync_control_center.rs:263-277` (add_log): 添加日志条目到内存缓冲区,自动维护最近500条日志
- `src/web_server/sync_control_center.rs:279-287` (get_logs): 获取最近N条日志
- `src/web_server/incremental_update_handlers.rs:442-466` (get_increment_logs): HTTP接口,从SYNC_CONTROL_CENTER读取日志并返回JSON格式

### 7. SSE事件推送

- `src/web_server/sync_control_center.rs:28-32` (SYNC_EVENT_TX): 全局broadcast通道,支持最多1000个事件缓冲
- `src/web_server/sse_handlers.rs` (SyncEvent): 事件枚举类型,包含Started、Stopped、SyncCompleted、SyncFailed、ProgressUpdate、ConnectionChanged、Alert等

### 8. 进度广播中心

- `src/web_server/sync_control_center.rs:235` (progress_hub): 可选的ProgressHub实例,用于WebSocket实时推送
- `src/shared/progress_hub.rs` (ProgressHub): 进度广播中心实现(文件未在本次调查中读取,但从注释可知存在)

---

## Report (调查结果)

### result

#### 1. 增量更新历史记录

**现有能力:**
- SQLite数据库`deployment_sites.sqlite`中的`remote_sync_logs`表存储完整历史记录
- 表结构包含16个字段,覆盖任务元数据、时间戳、状态、错误信息等
- 已建立env_id和status索引,支持高效查询

**查询接口:**
- `GET /api/remote-sync/logs` - 支持多维度过滤(env_id、site_id、target_site、status、direction、时间范围、关键词)
- 支持分页查询(limit、offset参数)
- 返回总数和记录列表

**数据字段:**
```rust
{
  id: String,              // 日志ID
  task_id: String,         // 任务ID
  env_id: String,          // 环境ID
  source_env: String,      // 源环境
  target_site: String,     // 目标站点
  site_id: String,         // 站点ID
  direction: String,       // 方向(UPLOAD/DOWNLOAD)
  file_path: String,       // 文件路径
  file_size: u64,          // 文件大小(字节)
  record_count: u64,       // 记录数
  status: String,          // 状态(pending/running/completed/failed/cancelled)
  error_message: String,   // 错误信息
  notes: String,           // 备注
  started_at: String,      // 开始时间(RFC3339)
  completed_at: String,    // 完成时间(RFC3339)
  created_at: String,      // 创建时间(RFC3339)
  updated_at: String,      // 更新时间(RFC3339)
}
```

#### 2. 实时状态查询

**当前运行状态:**
- `SyncControlState`结构包含所有核心运行指标
- 存储在全局单例`SYNC_CONTROL_CENTER`中,使用RwLock保证并发安全

**实时数据字段:**
```rust
{
  is_running: bool,               // 服务是否运行
  is_paused: bool,                // 是否暂停
  current_env: Option<String>,    // 当前环境ID
  env_name: Option<String>,       // 环境名称
  mqtt_connected: bool,           // MQTT连接状态
  watcher_active: bool,           // 文件监听器状态
  total_synced: u64,              // 总成功数
  total_failed: u64,              // 总失败数
  pending_count: u32,             // 待处理任务数
  queue_size: u32,                // 队列大小
  sync_rate_mbps: f64,            // 同步速率(Mbps)
  avg_sync_time_ms: u64,          // 平均同步时间(ms)
  last_sync_time: Option<SystemTime>, // 最后同步时间
  started_at: Option<SystemTime>, // 启动时间
  uptime_seconds: u64,            // 运行时长(秒)
}
```

**进度信息:**
- 增量更新进度信息**当前不在SyncControlState中**,需要新增或通过其他途径获取
- `ProgressHub`提供WebSocket推送机制,但与SyncControlState未完全集成

**增删改统计:**
- SQLite日志表中的`record_count`字段存储记录数
- **缺失**增量更新中新增、修改、删除元素的细分统计
- 建议在`remote_sync_logs`表或内存状态中新增字段:`added_count`、`modified_count`、`deleted_count`

#### 3. 失败任务统计

**FailedTaskQueue API:**

1. **get_stats() → TaskQueueStats**
   - `total: usize` - 队列中总任务数
   - `pending: usize` - 待重试任务数(已到重试时间)
   - `waiting: usize` - 等待中任务数(未到重试时间)
   - `exhausted: usize` - 已耗尽任务数(达到最大重试次数)

2. **get_pending_tasks() → Vec<FailedTask>**
   - 返回所有should_retry()为true的任务
   - 适用于显示即将重试的任务列表

3. **get_exhausted_tasks() → Vec<FailedTask>**
   - 返回retry_count >= max_retries的任务
   - 适用于告警和人工介入场景

4. **get_all_tasks() → Vec<FailedTask>**
   - 返回完整任务列表
   - 适用于仪表盘完整视图

**FailedTask详细信息:**
```rust
{
  id: String,                        // UUID
  task_type: FailedTaskType,         // DatabaseQuery/Compression/IncrementUpdate/MqttPublish
  error: String,                     // 错误信息
  error_trace: Option<String>,       // 错误堆栈
  retry_count: u32,                  // 当前重试次数
  max_retries: u32,                  // 最大重试次数(默认5)
  first_failed_at: SystemTime,       // 首次失败时间
  last_retry_at: Option<SystemTime>, // 最后重试时间
  next_retry_at: SystemTime,         // 下次重试时间
  priority: u8,                      // 优先级(1-10)
  metadata: Option<serde_json::Value> // 额外元数据
}
```

**过滤能力:**
- 当前接口**不支持**按类型、时间、优先级过滤
- `get_all_tasks()`返回完整列表,需在应用层进行过滤
- 建议新增:`get_tasks_by_type(task_type)`, `get_tasks_since(timestamp)`, `get_tasks_by_priority(min_priority)`

#### 4. 数据聚合需求

**现有聚合统计(来自SyncControlState):**
- 总成功次数: `total_synced`
- 总失败次数: `total_failed`
- 成功率计算: `total_synced / (total_synced + total_failed)`
- 平均耗时: `avg_sync_time_ms`

**缺失的聚合统计:**
1. **时间窗口统计**
   - 最近1小时/24小时/7天的同步次数
   - 时间段内的成功率趋势
   - 高峰期统计

2. **分类统计**
   - 按环境(env_id)分组的统计
   - 按站点(site_id)分组的统计
   - 按文件类型(file_path扩展名)分组的统计

3. **性能指标**
   - P50/P90/P99延迟分位数
   - 吞吐量(MB/s)
   - 并发任务数峰值

4. **增量更新详细统计**
   - 总增量次数
   - 平均每次增量的元素数量
   - 增量大小分布(字节数直方图)

**建议新增接口:**
```rust
// GET /api/remote-sync/stats/summary
pub async fn get_sync_summary(
    Query(params): Query<StatsSummaryQuery>
) -> Json<StatsSummary> {
    // params: time_window, env_id, site_id
    // returns: total_syncs, success_rate, avg_duration, total_bytes
}

// GET /api/remote-sync/stats/timeline
pub async fn get_sync_timeline(
    Query(params): Query<TimelineQuery>
) -> Json<Vec<TimelineDataPoint>> {
    // params: start, end, interval (hour/day)
    // returns: array of { timestamp, count, success_count, fail_count }
}

// GET /api/remote-sync/stats/performance
pub async fn get_performance_metrics(
    Query(params): Query<PerfQuery>
) -> Json<PerformanceMetrics> {
    // returns: { p50_ms, p90_ms, p99_ms, throughput_mbps, active_workers }
}
```

---

## conclusions

1. **SQLite日志表提供完整历史记录**:
   `remote_sync_logs`表包含16个字段,覆盖任务生命周期的所有关键信息,已建立索引支持高效查询。

2. **实时状态存储在内存单例中**:
   `SyncControlCenter`通过全局RwLock单例维护实时状态,包含运行状态、连接状态、统计指标、性能指标等15个字段。

3. **失败任务队列提供4个核心查询接口**:
   `get_stats()`、`get_pending_tasks()`、`get_exhausted_tasks()`、`get_all_tasks()`满足基本统计需求,但缺乏过滤能力。

4. **增量更新进度信息不完整**:
   当前系统未在`SyncControlState`中记录增量更新的完成百分比、当前步骤、增删改细分统计等实时进度数据。

5. **数据聚合接口需要新增**:
   现有接口仅支持原始数据查询和简单计数,缺少时间窗口统计、分组聚合、性能分位数等高级分析能力。

6. **SSE事件推送机制已就绪**:
   全局`SYNC_EVENT_TX`广播通道支持实时事件推送,但事件类型需扩展以支持进度更新和细粒度状态变化。

---

## relations

### 数据流关系

1. **增量更新流程 → 日志记录**:
   `src/data_interface/increment_manager.rs` 执行增量更新 → 插入记录到 `remote_sync_logs` 表 → `list_logs()` 接口查询返回

2. **任务失败 → 失败队列**:
   增量更新或同步失败 → 创建`FailedTask` → `FailedTaskQueue.push()` → 持久化到`assets/failed_tasks.json` → `get_stats()`查询统计

3. **实时状态更新 → SSE推送**:
   `SyncControlCenter` 状态变更 → `SYNC_EVENT_TX.send(SyncEvent)` → 订阅客户端通过`/api/remote-sync/events`接收

4. **日志缓冲 → 内存查询**:
   `SyncControlCenter.add_log()` → 写入内存`logs: Vec<LogEntry>` → `get_logs()` → `get_increment_logs()` HTTP接口返回

### 组件依赖关系

- `SyncControlCenter` 依赖 `SyncControlState`(状态存储) + `SyncTask`(任务模型) + `LogEntry`(日志模型)
- `remote_sync_handlers.rs` 依赖 `rusqlite`(SQLite访问) + `RemoteSyncLogRecord`(ORM映射)
- `FailedTaskQueue` 依赖 `tokio::fs`(异步文件IO) + `serde_json`(JSON序列化) + `RwLock`(并发控制)
- `incremental_update_handlers.rs` 依赖 `SYNC_CONTROL_CENTER`全局单例 + `chrono`(时间转换)

### 查询路径

**查询历史记录**:
HTTP请求 → `list_logs()` → `rusqlite::Connection` → `SELECT * FROM remote_sync_logs WHERE ...` → 返回`Vec<RemoteSyncLogRecord>`

**查询实时状态**:
HTTP请求 → 读取`SYNC_CONTROL_CENTER.read().await` → 克隆`state`字段 → 返回`SyncControlState`的JSON

**查询失败任务**:
内部调用 → `AiosDBManager.failed_queue.get_stats().await` → 读取`Arc<RwLock<Vec<FailedTask>>>` → 计算统计 → 返回`TaskQueueStats`

---

## 性能优化建议

### 数据库索引优化

**现有索引:**
- `idx_remote_sync_logs_env`: 覆盖`env_id`
- `idx_remote_sync_logs_status`: 覆盖`status`

**建议新增索引:**
```sql
-- 时间范围查询优化
CREATE INDEX idx_remote_sync_logs_created_desc
ON remote_sync_logs(created_at DESC);

-- 复合查询优化
CREATE INDEX idx_remote_sync_logs_env_status_time
ON remote_sync_logs(env_id, status, created_at DESC);

-- 站点统计优化
CREATE INDEX idx_remote_sync_logs_site_time
ON remote_sync_logs(site_id, created_at DESC);
```

### 缓存策略

1. **统计数据缓存**:
   使用`tokio::sync::RwLock<Option<(SystemTime, StatsSummary)>>`缓存聚合统计,TTL=30秒

2. **失败任务统计缓存**:
   `FailedTaskQueue`内部维护缓存的`TaskQueueStats`,仅在队列变更时重新计算

3. **时间线数据预计算**:
   后台任务每分钟预聚合最近24小时的时间线数据,存储在内存HashMap中

### 查询优化

1. **分页查询最大限制**:
   当前`limit.min(500)`,建议降低到100,避免单次查询返回过多数据

2. **COUNT(*)优化**:
   对于大表,使用`SELECT COUNT(*) FROM ... LIMIT 10001`判断是否超过10000条,超过则返回">10000"而非精确值

3. **批量查询**:
   提供批量查询接口`/api/remote-sync/logs/batch`,支持一次请求获取多个env_id的统计数据

### 实时性优化

1. **WebSocket替代轮询**:
   仪表盘使用WebSocket连接`ProgressHub`,订阅实时更新事件,避免频繁轮询HTTP接口

2. **SSE事件细化**:
   新增`SyncEvent::TaskQueued`、`SyncEvent::TaskStarted`、`SyncEvent::TaskProgress`事件,提供更细粒度的实时反馈

3. **增量推送**:
   仅推送变化的数据(delta),而非完整状态快照,减少带宽消耗

---

## 新增接口设计建议

### 1. 聚合统计接口

**路由**: `GET /api/remote-sync/stats/summary`

**查询参数**:
```rust
struct StatsSummaryQuery {
    time_window: Option<String>, // "1h" | "24h" | "7d" | "30d"
    env_id: Option<String>,
    site_id: Option<String>,
}
```

**响应示例**:
```json
{
  "time_window": "24h",
  "total_syncs": 1523,
  "success_count": 1498,
  "fail_count": 25,
  "success_rate": 0.984,
  "avg_duration_ms": 342,
  "total_bytes": 15234567890,
  "avg_bytes_per_sync": 10002345
}
```

### 2. 时间线统计接口

**路由**: `GET /api/remote-sync/stats/timeline`

**查询参数**:
```rust
struct TimelineQuery {
    start: String,     // RFC3339 timestamp
    end: String,       // RFC3339 timestamp
    interval: String,  // "hour" | "day"
    env_id: Option<String>,
}
```

**响应示例**:
```json
{
  "data_points": [
    {
      "timestamp": "2025-11-20T10:00:00Z",
      "total_count": 45,
      "success_count": 43,
      "fail_count": 2,
      "total_bytes": 456789012
    },
    // ...
  ]
}
```

### 3. 失败任务过滤接口

**路由**: `GET /api/failed-tasks/query`

**查询参数**:
```rust
struct FailedTaskQueryParams {
    task_type: Option<String>,  // "DatabaseQuery" | "Compression" | ...
    since: Option<String>,       // RFC3339 timestamp
    min_priority: Option<u8>,
    status: Option<String>,      // "pending" | "waiting" | "exhausted"
    limit: Option<usize>,
    offset: Option<usize>,
}
```

**实现建议**:
在`FailedTaskQueue`中新增方法:
```rust
pub async fn query_tasks(
    &self,
    filter: FailedTaskFilter
) -> Vec<FailedTask> {
    let tasks = self.tasks.read().await;
    tasks.iter()
        .filter(|t| filter.matches(t))
        .cloned()
        .collect()
}
```

### 4. 增量更新进度接口

**路由**: `GET /api/incremental/progress/{task_id}`

**响应示例**:
```json
{
  "task_id": "abc-123",
  "status": "running",
  "current_step": "update_database",
  "total_steps": 5,
  "progress_percent": 60,
  "elements_processed": 12345,
  "total_elements": 20000,
  "added_count": 500,
  "modified_count": 11000,
  "deleted_count": 845,
  "elapsed_ms": 5432,
  "estimated_remaining_ms": 3600
}
```

**实现建议**:
在`SyncControlCenter`中新增字段:
```rust
pub struct SyncControlCenter {
    // ...
    pub active_increment_progress: HashMap<String, IncrementProgress>,
}

pub struct IncrementProgress {
    pub task_id: String,
    pub current_step: String,
    pub total_steps: u32,
    pub progress_percent: f32,
    pub elements_processed: u64,
    pub total_elements: u64,
    pub added_count: u64,
    pub modified_count: u64,
    pub deleted_count: u64,
    pub started_at: SystemTime,
}
```

---

**文档版本**: 1.0
**调查日期**: 2025-11-20
**调查范围**: 增量更新监控仪表盘数据接口
**代码分支**: only-csg
**代码版本**: commit 749ec4c
