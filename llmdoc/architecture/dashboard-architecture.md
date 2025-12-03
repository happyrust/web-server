# 增量更新实时监控仪表盘架构文档

## 1. Identity

- **What it is**: 基于Web的增量更新实时监控仪表盘，提供可视化的任务状态追踪、失败任务管理和性能分析功能
- **Purpose**: 为运维人员提供直观的增量更新系统监控界面，实现故障快速发现、数据可视化分析和失败任务管理

## 2. 架构概览

### 2.1 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend (浏览器)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Alpine.js    │  │ Chart.js     │  │ WebSocket       │  │
│  │ (状态管理)   │  │ (图表渲染)   │  │ (实时推送)      │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬────────┘  │
│         │                  │                   │            │
└─────────┼──────────────────┼───────────────────┼────────────┘
          │                  │                   │
          │ REST API         │ 数据查询          │ WS连接
          ▼                  ▼                   ▼
┌─────────────────────────────────────────────────────────────┐
│                     Backend (Rust/Axum)                     │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           dashboard_handlers.rs (7个API)             │  │
│  │  - get_dashboard_summary()    - get_active_tasks()   │  │
│  │  - get_failed_tasks()         - get_task_details()   │  │
│  │  - get_timeline_stats()       - retry_failed_task()  │  │
│  │  - cleanup_exhausted_tasks()                         │  │
│  └─────────┬────────────────────────────┬────────────────┘  │
│            │                            │                    │
│  ┌─────────▼────────┐       ┌──────────▼──────────┐        │
│  │ SYNC_CONTROL_    │       │ FailedTaskQueue     │        │
│  │ CENTER           │       │ (Arc<RwLock>)       │        │
│  │ (全局状态)       │       │ assets/             │        │
│  │                  │       │ failed_tasks.json   │        │
│  └──────────────────┘       └─────────────────────┘        │
│            │                                                 │
│  ┌─────────▼────────────────────────────────────────────┐  │
│  │         deployment_sites.sqlite                      │  │
│  │         remote_sync_logs表 (历史记录)                │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 技术栈

**后端:**
- Rust 1.75+ (nightly)
- Axum 0.6 - Web框架
- Tokio - 异步运行时
- Serde - JSON序列化
- Rusqlite - SQLite数据库访问
- Chrono - 时间处理

**前端:**
- Alpine.js 3.x - 轻量级响应式框架
- Chart.js 4.4 - 数据可视化
- Tailwind CSS - 工具类样式
- WebSocket API - 实时通信
- Fetch API - HTTP请求

---

## 3. Core Components

### 3.1 后端模块

#### dashboard_handlers.rs

**职责**: 处理所有Dashboard相关的HTTP请求

**核心函数:**

1. `get_dashboard_summary()` - 获取仪表盘概览
   - 位置: `src/web_server/dashboard_handlers.rs:255`
   - 返回: `DashboardSummary` (统计卡片数据)
   - 数据来源: SYNC_CONTROL_CENTER + FailedTaskQueue

2. `get_active_tasks()` - 获取活跃任务列表
   - 位置: `src/web_server/dashboard_handlers.rs:296`
   - 返回: `Vec<ActiveTaskInfo>`
   - 数据来源: SYNC_CONTROL_CENTER.running_tasks

3. `get_failed_tasks()` - 获取失败任务（支持过滤）
   - 位置: `src/web_server/dashboard_handlers.rs:326`
   - 查询参数: `FailedTaskQueryParams`
   - 过滤逻辑: 类型、优先级、时间范围、状态

4. `get_timeline_stats()` - 时间线统计数据
   - 位置: `src/web_server/dashboard_handlers.rs:388`
   - 查询参数: `TimelineQueryParams` (window: 1h/24h/7d/30d)
   - 返回: `Vec<TimelineDataPoint>`

5. `query_timeline_data()` - SQLite聚合查询
   - 位置: `src/web_server/dashboard_handlers.rs:535`
   - 实现: 动态时间分桶查询
   - 性能优化: GROUP BY time_bucket 避免全表扫描

**数据结构:**

```rust
pub struct DashboardSummary {
    pub task_stats: TaskStatistics,           // 任务统计
    pub failed_task_stats: FailedTaskStatistics, // 失败任务统计
    pub performance_metrics: PerformanceMetrics, // 性能指标
    pub recent_events: Vec<RecentEvent>,       // 最近事件
}

pub struct TaskStatistics {
    pub in_progress: usize,      // 进行中
    pub completed_today: usize,  // 今日完成
    pub failed_today: usize,     // 今日失败
    pub pending: usize,          // 待处理
}

pub struct TimelineDataPoint {
    pub timestamp: DateTime<Utc>,  // 时间戳
    pub success_count: usize,      // 成功数
    pub failure_count: usize,      // 失败数
    pub avg_duration_secs: f64,    // 平均耗时
    pub total_bytes: u64,          // 总字节数
}
```

#### dashboard_template.rs

**职责**: 生成Dashboard HTML页面

**位置**: `src/web_server/dashboard_template.rs`

**关键代码段:**

1. 统计卡片 (行8-75)
   - 4个卡片：进行中、今日完成、今日失败、成功率
   - Alpine.js数据绑定: `x-text="summary.task_stats?.in_progress"`

2. 失败任务队列 (行77-101)
   - 4个指标：待重试、等待中、已耗尽、总计
   - 操作按钮: 查看详情、清理已耗尽

3. 活跃任务列表 (行103-136)
   - 循环渲染: `x-for="task in activeTasks"`
   - 进度条: 动态宽度 `:style="width: ${task.progress}%"`

4. Chart.js图表 (行138-177)
   - 任务趋势图 (面积图)
   - 成功率趋势图 (折线图)
   - 平均耗时图 (柱状图)

5. 失败任务模态框 (行200-264)
   - 过滤器: 类型、状态
   - 任务列表: 展开元数据、重试按钮

### 3.2 前端模块

#### dashboard.js

**职责**: Alpine.js应用逻辑和Chart.js集成

**位置**: `src/web_server/static/dashboard.js`

**核心方法:**

1. `init()` - 初始化
   ```javascript
   async init() {
       await this.loadSummary();       // 加载统计
       await this.loadActiveTasks();   // 加载任务
       await this.initCharts();        // 初始化图表
       await this.loadTimelineData();  // 加载时间线
       this.startPolling();            // 开始轮询
       this.connectWebSocket();        // 连接WS
   }
   ```

2. `initCharts()` - 初始化Chart.js图表 (行142-246)
   ```javascript
   this.charts.taskTrend = new Chart(ctx, {
       type: 'line',
       data: { labels: [], datasets: [...] },
       options: {
           responsive: true,
           maintainAspectRatio: false,
           ...
       }
   });
   ```

3. `updateCharts(timelineData)` - 更新图表数据 (行262-303)
   - 数据转换: 时间戳 → 本地化标签
   - 成功率计算: `success / (success + failure) * 100`
   - Chart.update(): 触发重绘

4. `connectWebSocket()` - WebSocket连接 (行69-92)
   ```javascript
   const ws = new WebSocket(`ws://${window.location.host}/ws/tasks`);
   ws.onmessage = (event) => {
       const data = JSON.parse(event.data);
       this.handleWebSocketMessage(data);
   };
   ```

5. `handleWebSocketMessage(data)` - 处理WS消息 (行95-109)
   - TaskStatusChange: 更新任务状态
   - StatsUpdate: 更新统计数据
   - NewFailedTask: 刷新失败任务

**状态管理:**

```javascript
// Alpine.js响应式数据
{
    summary: {},              // 概览统计
    activeTasks: [],          // 活跃任务
    failedTasks: [],          // 失败任务
    selectedTimeWindow: '24h', // 时间窗口
    showFailedModal: false,   // 模态框显示
    charts: {                 // Chart.js实例
        taskTrend: null,
        successRate: null,
        avgDuration: null
    }
}
```

### 3.3 数据持久化

#### SQLite数据库

**文件**: `deployment_sites.sqlite`

**表结构**: `remote_sync_logs`

```sql
CREATE TABLE remote_sync_logs (
    id TEXT PRIMARY KEY,
    task_id TEXT,
    env_id TEXT,
    source_env TEXT,
    target_site TEXT,
    site_id TEXT,
    direction TEXT,        -- UPLOAD/DOWNLOAD
    file_path TEXT,
    file_size INTEGER,
    record_count INTEGER,
    status TEXT,           -- pending/running/completed/failed/cancelled
    error_message TEXT,
    notes TEXT,
    started_at INTEGER,    -- Unix纳秒时间戳
    completed_at INTEGER,
    created_at INTEGER,
    updated_at INTEGER
);

CREATE INDEX idx_env_id ON remote_sync_logs(env_id);
CREATE INDEX idx_status ON remote_sync_logs(status);
```

**查询优化:**

- 时间分桶聚合: `GROUP BY datetime((started_at / interval) * interval)`
- 索引利用: env_id和status复合条件查询
- 时间范围过滤: `WHERE started_at >= ...` 利用时间戳比较

#### FailedTaskQueue持久化

**文件**: `assets/failed_tasks.json`

**格式**: JSON数组

```json
[
  {
    "id": "uuid",
    "task_type": { "type": "DatabaseQuery", ... },
    "error": "错误信息",
    "retry_count": 2,
    "max_retries": 5,
    "first_failed_at": 1732089600,
    "next_retry_at": 1732089840,
    "priority": 5,
    "metadata": { ... }
  }
]
```

**持久化机制:**

- 原子写入: 临时文件 + rename
- 触发时机: push/update/remove/cleanup
- 恢复机制: 系统启动时自动加载

---

## 4. Execution Flow (LLM Retrieval Map)

### 4.1 页面加载流程

```
1. 用户访问 /dashboard
   ↓
2. dashboard_page() handler触发
   (src/web_server/handlers.rs:5362)
   ↓
3. dashboard_template::render_dashboard_page()
   (src/web_server/dashboard_template.rs:4)
   ↓
4. 生成HTML（包含Alpine.js和Chart.js）
   ↓
5. 浏览器加载静态资源
   - /static/dashboard.js
   - /static/dashboard.css
   - Chart.js CDN
   ↓
6. Alpine.js初始化
   - dashboardApp().init()
   ↓
7. 并行执行:
   - loadSummary() → GET /api/dashboard/summary
   - loadActiveTasks() → GET /api/dashboard/active-tasks
   - initCharts() → 创建3个Chart.js实例
   - loadTimelineData() → GET /api/dashboard/stats/timeline
   - connectWebSocket() → WS /ws/tasks
   ↓
8. 启动定时轮询 (30秒间隔)
```

### 4.2 实时更新流程

#### WebSocket推送路径

```
1. 增量更新任务状态变更
   (src/data_interface/increment_manager.rs)
   ↓
2. 发布到ProgressHub
   progress_hub.publish(task_id, message)
   ↓
3. WebSocket广播
   (src/web_server/ws/progress.rs)
   ↓
4. 客户端接收
   ws.onmessage → handleWebSocketMessage()
   ↓
5. 更新Alpine.js状态
   - 更新activeTasks数组
   - 触发视图重绘
```

#### 定时轮询路径

```
1. setInterval触发 (每30秒)
   ↓
2. 并行请求:
   - /api/dashboard/summary
   - /api/dashboard/active-tasks
   ↓
3. 更新Alpine.js状态
   - summary对象
   - activeTasks数组
   ↓
4. 视图自动刷新 (Alpine.js响应式)
```

### 4.3 图表更新流程

```
1. 用户选择时间窗口
   x-model="selectedTimeWindow"
   @change="loadTimelineData()"
   ↓
2. loadTimelineData()
   GET /api/dashboard/stats/timeline?window=24h
   ↓
3. 后端查询SQLite
   query_timeline_data()
   - 动态计算时间分桶
   - GROUP BY聚合
   ↓
4. 返回TimelineDataPoint数组
   ↓
5. updateCharts(timelineData)
   - 转换时间戳为标签
   - 计算成功率
   - 更新Chart.js数据
   ↓
6. chart.update() 触发重绘
```

### 4.4 失败任务管理流程

```
1. 用户点击"查看详情"
   @click="showFailedTasks()"
   ↓
2. 打开模态框
   showFailedModal = true
   ↓
3. loadFailedTasks()
   GET /api/dashboard/failed-tasks?type=X&status=Y
   ↓
4. 后端过滤FailedTaskQueue
   - 类型匹配
   - 状态判断 (should_retry/is_exhausted)
   ↓
5. 返回过滤后的任务列表
   ↓
6. 渲染任务卡片
   x-for="task in failedTasks"
   ↓
7. 用户点击"重试"按钮
   @click="retryTask(task.id)"
   ↓
8. POST /api/dashboard/retry-task/:id
   ↓
9. 后端执行重试逻辑
   (TODO: 当前仅加入队列)
   ↓
10. 刷新任务列表和统计
```

---

## 5. Design Rationale

### 5.1 为什么选择Alpine.js而非React/Vue？

**优势:**
- ✅ 轻量级 (15KB vs 100KB+)
- ✅ 无需构建步骤，直接在HTML中使用
- ✅ 与项目现有技术栈一致 (remote_sync_template.rs已使用)
- ✅ 学习曲线低，维护成本小

**权衡:**
- ❌ 复杂状态管理能力较弱
- ❌ 生态系统不如React/Vue丰富
- ✅ 对仪表盘场景足够（状态复杂度低）

### 5.2 为什么使用WebSocket + 定时轮询混合模式？

**WebSocket优势:**
- 实时性：延迟<1秒
- 减少带宽：仅推送变更

**定时轮询优势:**
- 可靠性：防止WebSocket断开导致数据不同步
- 简单性：统计数据聚合适合轮询

**混合策略:**
- WebSocket：任务状态变更（频繁、小数据）
- 轮询：统计数据（低频、大数据）
- 30秒间隔：平衡实时性和服务器负担

### 5.3 为什么Chart.js使用CDN而非本地？

**CDN优势:**
- ✅ 减少编译后二进制大小
- ✅ 浏览器缓存共享
- ✅ 快速原型开发

**未来优化:**
- [ ] 生产环境可切换到本地文件
- [ ] 添加CDN失败时的本地fallback

### 5.4 为什么SQLite聚合查询而非SurrealDB？

**SQLite优势:**
- ✅ 已有remote_sync_logs表结构
- ✅ 时间分桶查询性能优秀
- ✅ 简单部署（单文件数据库）

**SurrealDB限制:**
- ❌ 时间聚合查询语法复杂
- ❌ 历史数据主要在SQLite

**未来扩展:**
- [ ] 考虑迁移到统一数据源（SurrealDB）
- [ ] 实现数据双写保持一致性

---

## 6. Performance Considerations

### 6.1 后端性能

**查询优化:**
- 时间范围索引: `WHERE started_at >= ...`
- 复合索引: `(env_id, status)`
- GROUP BY优化: 时间分桶避免全表扫描

**内存管理:**
- FailedTaskQueue: Arc<RwLock> 线程安全
- 读写锁: 读多写少场景优化
- JSON持久化: 异步执行不阻塞

**并发控制:**
- Axum异步处理: 高并发请求
- Tokio运行时: 高效任务调度

### 6.2 前端性能

**图表渲染:**
- Chart.js配置: `maintainAspectRatio: false`
- 数据点限制: 最多显示30天数据
- 懒加载: 仅初始化可见图表

**网络优化:**
- 并行请求: Promise.all同时加载多个API
- 数据缓存: Alpine.js响应式缓存
- 分页加载: 失败任务模态框可扩展分页

**渲染优化:**
- Alpine.js虚拟DOM: 高效DOM更新
- CSS动画: GPU加速过渡效果
- 防抖: 避免频繁刷新

### 6.3 WebSocket性能

**连接管理:**
- 自动重连: 断开后5秒重试
- 心跳检测: 保持连接活跃（TODO）
- 连接复用: 单个WebSocket处理所有消息

**消息优化:**
- JSON序列化: Serde高性能
- 广播容量: 1000消息缓冲
- 过滤推送: 仅推送订阅的任务

---

## 7. Security Considerations

### 7.1 认证授权

**当前状态:**
- ❌ 未实现用户认证
- ❌ 所有API公开访问
- ⚠️ 仅适用于内网部署

**未来改进:**
- [ ] 添加JWT认证
- [ ] API密钥验证
- [ ] 基于角色的访问控制

### 7.2 输入验证

**已实现:**
- ✅ 查询参数验证: Serde反序列化
- ✅ 路径参数验证: Axum提取器
- ✅ SQL注入防护: 参数化查询

**注意事项:**
- `format!` 拼接SQL: 需确保输入已验证
- 时间窗口: 仅允许预定义值（1h/24h/7d/30d）

### 7.3 数据保护

**敏感数据:**
- 文件路径: 可能暴露系统结构
- 错误信息: 可能包含内部细节

**防护措施:**
- 错误消息过滤: 避免暴露堆栈跟踪
- 访问控制: 限制可见的任务范围（TODO）

---

## 8. Testing Strategy

### 8.1 单元测试

**后端测试:**
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_timeline_data_aggregation() {
        // 测试时间分桶计算
        // 测试成功率计算
        // 测试边界条件
    }
}
```

**前端测试:**
```javascript
// 使用Vitest或Jest
describe('dashboardApp', () => {
    test('updates chart on time window change', () => {
        // 测试图表更新逻辑
    });
});
```

### 8.2 集成测试

**API测试:**
```bash
# 测试概览API
curl http://localhost:8080/api/dashboard/summary

# 测试时间线API
curl "http://localhost:8080/api/dashboard/stats/timeline?window=24h"

# 测试重试API
curl -X POST http://localhost:8080/api/dashboard/retry-task/uuid
```

**WebSocket测试:**
```javascript
const ws = new WebSocket('ws://localhost:8080/ws/tasks');
ws.onmessage = (e) => console.log(JSON.parse(e.data));
```

### 8.3 端到端测试

**手动测试清单:**
- [ ] 页面加载完整性
- [ ] 统计卡片数据准确性
- [ ] 图表渲染正确性
- [ ] 时间窗口切换功能
- [ ] 失败任务模态框交互
- [ ] WebSocket实时更新
- [ ] 定时轮询刷新
- [ ] 手动重试功能
- [ ] 清理已耗尽任务

---

## 9. Known Limitations

### 9.1 功能限制

1. **任务详情**
   - ❌ `get_task_details_from_db()` 返回占位符
   - 需要实现: 从remote_sync_logs查询完整任务信息

2. **重试逻辑**
   - ⚠️ `retry_failed_task()` 仅加入队列
   - 需要实现: 调用实际的重试处理函数

3. **统计数据**
   - ❌ `get_completed_tasks_today()` 返回0
   - ❌ `get_tasks_last_hour()` 返回0
   - 需要实现: 从SQLite查询今日数据

### 9.2 性能限制

1. **大数据集**
   - 30天时间窗口可能返回大量数据点
   - 建议: 实现数据点采样或聚合

2. **并发限制**
   - FailedTaskQueue全局锁可能成为瓶颈
   - 建议: 考虑分片或无锁数据结构

3. **内存占用**
   - Chart.js实例常驻内存
   - 建议: 实现图表销毁和懒加载

### 9.3 浏览器兼容性

**已测试:**
- ✅ Chrome 90+
- ✅ Edge 90+
- ✅ Firefox 88+

**未测试:**
- ❓ Safari (应该支持)
- ❌ IE11 (不支持)

---

## 10. Future Enhancements

### 10.1 短期改进 (1-2周)

- [ ] 实现完整的任务详情查询
- [ ] 实现IncrementUpdate和MqttPublish重试逻辑
- [ ] 添加今日统计数据查询
- [ ] 实现失败任务分页加载

### 10.2 中期改进 (1-2月)

- [ ] 添加用户认证和授权
- [ ] 实现数据导出功能 (CSV/JSON)
- [ ] 添加浏览器通知API集成
- [ ] 实现图表数据缓存

### 10.3 长期改进 (3-6月)

- [ ] 迁移到统一数据源 (SurrealDB)
- [ ] 实现Prometheus指标导出
- [ ] 添加告警规则引擎
- [ ] 支持多租户部署

---

**文档版本**: 1.0
**创建时间**: 2025-11-21
**维护者**: AIOS开发团队
**相关文档**:
- [Dashboard使用指南](../guides/dashboard-usage-guide.md)
- [失败任务队列架构](./increment-error-recovery.md)
