# Web UI结构和实时通讯机制深度调查报告

**调查日期**: 2025-11-20
**调查目标**: 深入分析现有Web UI架构、SSE/WebSocket实时通讯机制、监控页面实现模式

---

## 代码章节 (Code Sections - 证据清单)

### 1. 核心架构文件

- `src/web_server/mod.rs` (`AppState`, `start_web_server_with_config`) - Web服务器主入口，包含全局状态定义、路由注册、中间件配置
- `src/web_server/layout.rs` (`render_layout_with_sidebar`) - 统一页面布局系统，提供侧边栏导航和顶部导航的标准模板
- `src/web_server/handlers.rs` - 核心API处理器集合，包含任务管理、配置管理、数据库操作等handlers

### 2. 实时通讯基础设施

#### SSE (Server-Sent Events)
- `src/web_server/sse_handlers.rs` (`sync_events_handler`, `SyncEvent`) - SSE事件流处理器，使用`tokio_stream::wrappers::BroadcastStream`实现多播推送
- `src/web_server/sync_control_center.rs` (`SYNC_EVENT_TX`) - 全局广播通道`Lazy<broadcast::Sender<SyncEvent>>`，容量1000

#### WebSocket
- `src/web_server/ws/progress.rs` (`ws_progress_handler`, `handle_progress_socket`) - 单任务进度订阅WebSocket端点
- `src/web_server/ws/progress.rs` (`ws_tasks_handler`, `handle_tasks_socket`) - 多任务订阅WebSocket端点，支持动态订阅/取消订阅
- `src/web_server/ws/mod.rs` - WebSocket模块导出层

#### 进度管理核心
- `src/shared/progress_hub.rs` (`ProgressHub`) - 统一进度广播中心，基于`DashMap`和`broadcast channel`实现线程安全的多路广播
- `docs/PROGRESS_HUB_IMPLEMENTATION.md` - ProgressHub架构文档，详细说明了设计原则、API、使用场景

### 3. 监控页面实现

#### 远程同步监控
- `src/web_server/remote_sync_template.rs` (`render_remote_sync_page_with_sidebar`) - 异地增量环境配置页面，使用Alpine.js实现响应式UI
- `src/web_server/remote_sync_handlers.rs` - 远程同步相关API handlers (list_envs, create_env, list_sites等)
- `src/web_server/sync_control_handlers.rs` - 同步控制API handlers (start/stop/pause/resume/get_status等)
- `src/web_server/sync_control_center.rs` (`SyncControlCenter`, `SyncControlState`) - 同步控制中心核心状态管理

#### 数据库状态监控
- `src/web_server/db_status_template.rs` (`db_status_page`) - 数据库状态管理页面，完全自包含HTML+CSS+JS
- `src/web_server/db_status_handlers.rs` - 数据库状态相关API handlers (get_db_status_list, execute_incremental_update等)
- `src/web_server/database_status_handlers.rs` - 增强的数据库状态管理API (get_all_database_status, reparse_database等)

#### 增量更新监控
- `src/web_server/incremental_update_handlers.rs` - 增量更新检测相关API (get_all_incremental_status, start_incremental_detection等)
- `src/web_server/static/incremental_update.js` (197KB) - 增量更新页面前端逻辑
- `src/web_server/static/incremental_update.css` (123KB) - 增量更新页面样式

### 4. 前端资源结构

#### JavaScript库
- `src/web_server/static/alpine.min.js` (44KB) - Alpine.js响应式框架
- `src/web_server/static/chart.umd.min.js` (205KB) - Chart.js图表库
- `src/web_server/static/xeokit-sdk.es.js` (6.4MB) - 3D模型可视化SDK

#### 页面级JS文件
- `src/web_server/static/sync-control.js` (13KB) - 同步控制面板逻辑
- `src/web_server/static/database_status.js` (30KB) - 数据库状态页面逻辑
- `src/web_server/static/deployment-sites.js` (12KB) - 部署站点管理逻辑
- `src/web_server/static/projects.js` (65KB) - 项目管理页面逻辑

#### CSS样式
- `src/web_server/static/ui.css` (12KB) - 统一UI样式
- `src/web_server/static/simple-tailwind.css` (16KB) - 简化版Tailwind CSS
- `src/web_server/static/simple-icons.css` (4KB) - 图标样式
- `src/web_server/static/font-awesome.min.css` (89KB) - FontAwesome图标库

### 5. 路由注册模式

#### 核心路由分组 (`src/web_server/mod.rs` 第254-856行)
- **API路由**: `/api/*` - RESTful API端点
- **页面路由**: `/dashboard`, `/tasks`, `/incremental`, `/remote-sync` 等
- **WebSocket路由**: `/ws/progress/:task_id`, `/ws/tasks`
- **SSE路由**: `/api/sync/events/stream`, `/api/sync/events/test`
- **静态文件**: `/static/*`, `/files/output/*`, `/assets/archives/*`

---

## 报告 (调查结果)

### 1. 前端架构总览

#### 1.1 目录结构
```
src/web_server/
├── static/               # 静态资源目录
│   ├── *.js             # 页面级JavaScript (Alpine.js为主)
│   ├── *.css            # 样式表
│   └── xeokit-sdk.es.js # 3D可视化SDK
├── mod.rs               # 路由注册和全局状态
├── layout.rs            # 统一布局模板
├── handlers.rs          # 核心API handlers
├── *_template.rs        # 各页面HTML模板
├── *_handlers.rs        # 各功能模块API handlers
└── ws/                  # WebSocket模块
    ├── mod.rs
    └── progress.rs      # 进度推送实现
```

#### 1.2 前端框架选型
- **Alpine.js**: 主要响应式框架，轻量级（44KB），类似Vue.js的声明式语法
- **Chart.js**: 图表可视化，用于统计数据展示
- **XeoKit SDK**: 3D模型查看器，处理GLTF/GLB/XKT格式
- **FontAwesome**: 图标系统
- **自定义Tailwind CSS**: 简化版工具类CSS框架

#### 1.3 页面模板系统
**实现方式**: Rust字符串模板 (不使用外部模板引擎)
- 优点: 零运行时依赖，编译时检查
- 缺点: 手动字符串拼接，缺乏语法高亮

**标准模式** (以`remote_sync_template.rs`为例):
```rust
pub fn render_remote_sync_page_with_sidebar() -> String {
    let content = r#"<div x-data="remoteSyncApp()" ...>...</div>"#;
    layout::render_layout_with_sidebar(
        "异地增量环境配置",
        Some("remote-sync"),
        content,
        extra_head,
        extra_scripts
    )
}
```

#### 1.4 现有Dashboard页面
**位置**: 路由`/dashboard`已注册 (mod.rs 第810行)
**实现**: 未找到对应template文件，可能使用通用handler或需要新建

---

### 2. SSE/WebSocket实时通讯机制

#### 2.1 SSE实现详解

**核心组件**:
1. **全局广播通道** (`sync_control_center.rs` 第29-32行):
```rust
pub static SYNC_EVENT_TX: Lazy<broadcast::Sender<SyncEvent>> = Lazy::new(|| {
    let (tx, _) = broadcast::channel(1000); // 缓冲区容量1000
    tx
});
```

2. **事件处理器** (`sse_handlers.rs` 第115-146行):
```rust
pub async fn sync_events_handler() -> impl IntoResponse {
    let rx = SYNC_EVENT_TX.subscribe();
    let stream = BroadcastStream::new(rx);
    let event_stream = stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                match serde_json::to_string(&event) {
                    Ok(json) => Some(Ok(Event::default().data(json).event("message"))),
                    Err(_) => None
                }
            }
            Err(_) => None
        }
    });
    Sse::new(event_stream).keep_alive(KeepAlive::default())
}
```

3. **事件类型** (`sse_handlers.rs` 第18-97行):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SyncEvent {
    Started { env_id: String, timestamp: String },
    Stopped { env_id: String, timestamp: String },
    SyncCompleted { task_id: String, file_path: String, duration_ms: u64, ... },
    SyncFailed { task_id: String, error: String, ... },
    MqttConnected { env_id: String, ... },
    ProgressUpdate { total: u64, completed: u64, failed: u64, ... },
    Alert { level: String, message: String, ... },
    // ... 其他事件
}
```

**推送流程**:
```
事件发生 -> SYNC_EVENT_TX.send(event)
         -> 所有订阅者收到
         -> 通过SSE推送到前端
```

**路由**:
- `/api/sync/events/stream` - 主要SSE端点
- `/api/sync/events/test` - 测试端点

#### 2.2 WebSocket实现详解

**单任务进度订阅** (`ws/progress.rs` 第53-175行):

**路由**: `/ws/progress/:task_id`

**握手流程**:
1. 客户端连接 -> 检查任务是否存在
2. 发送握手消息 (包含当前状态)
3. 订阅进度广播通道 `hub.subscribe(&task_id)`
4. 持续推送更新

**核心代码**:
```rust
pub async fn ws_progress_handler(
    ws: WebSocketUpgrade,
    Path(task_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let hub = state.progress_hub.clone();
    ws.on_upgrade(move |socket| handle_progress_socket(socket, task_id, hub))
}

async fn handle_progress_socket(socket: WebSocket, task_id: String, hub: Arc<ProgressHub>) {
    let (mut sender, mut receiver) = socket.split();

    // 发送握手消息
    let handshake_msg = WsHandshakeMessage { ... };
    sender.send(Message::Text(json)).await;

    // 订阅进度
    let mut progress_rx = hub.subscribe(&task_id);

    // 双向通信循环
    loop {
        tokio::select! {
            msg = receiver.next() => { /* 处理客户端消息 */ }
            progress = progress_rx.recv() => { /* 推送进度更新 */ }
        }
    }
}
```

**多任务订阅** (`ws/progress.rs` 第187-298行):

**路由**: `/ws/tasks`

**客户端命令**:
```json
{ "action": "subscribe", "task_id": "task-123" }
{ "action": "unsubscribe", "task_id": "task-123" }
{ "action": "list" }
```

**特性**:
- 动态订阅/取消订阅
- 同时监听多个任务
- 支持列出所有活跃任务

#### 2.3 ProgressHub统一进度管理

**设计目标** (来自`PROGRESS_HUB_IMPLEMENTATION.md`):
- 单一数据源
- 多路广播 (gRPC + WebSocket + 其他)
- 自动清理
- 线程安全

**核心API** (文档第86-114行):
```rust
pub struct ProgressHub {
    channels: Arc<DashMap<String, broadcast::Sender<ProgressMessage>>>,
    task_states: Arc<DashMap<String, ProgressMessage>>,
    buffer_size: usize, // 默认64
}

impl ProgressHub {
    pub fn register(&self, task_id: String) -> broadcast::Receiver<ProgressMessage>;
    pub fn subscribe(&self, task_id: &str) -> broadcast::Receiver<ProgressMessage>;
    pub fn publish(&self, message: ProgressMessage) -> Result<usize, String>;
    pub fn get_task_state(&self, task_id: &str) -> Option<ProgressMessage>;
    pub fn active_tasks(&self) -> Vec<String>;
}
```

**消息结构** (文档第118-135行):
```rust
pub struct ProgressMessage {
    pub task_id: String,
    pub status: TaskStatus, // Pending | Running | Completed | Failed | Cancelled
    pub percentage: f32,
    pub current_step: String,
    pub current_step_number: u32,
    pub total_steps: u32,
    pub processed_items: u64,
    pub total_items: u64,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub details: Option<serde_json::Value>,
}
```

**使用示例** (文档第200-227行):
```rust
// WebSocket推送场景
async fn handle_websocket(ws: WebSocket, task_id: String, hub: Arc<ProgressHub>) {
    let mut rx = hub.subscribe(&task_id);

    // 发送当前状态
    if let Some(state) = hub.get_task_state(&task_id) {
        let json = serde_json::to_string(&state).unwrap();
        ws.send(Message::Text(json)).await.ok();
    }

    // 持续推送
    while let Ok(msg) = rx.recv().await {
        let json = serde_json::to_string(&msg).unwrap();
        if ws.send(Message::Text(json)).await.is_err() { break; }
    }
}
```

---

### 3. 现有监控页面实现模式

#### 3.1 远程同步监控页面

**文件**: `src/web_server/remote_sync_template.rs`

**技术栈**:
- Alpine.js响应式数据绑定
- 内联CSS (scope到`.wrapped-page`)
- AJAX轮询 + SSE推送混合模式

**核心功能**:
1. 环境配置管理 (CRUD)
2. 站点配置管理
3. 运行时状态监控
4. MQTT连接状态显示
5. 实时日志推送

**Alpine.js数据模型**:
```javascript
function remoteSyncApp() {
    return {
        envs: [],           // 环境列表
        selectedEnv: null,  // 当前选中环境
        sites: [],          // 站点列表
        runtimeStatus: {},  // 运行时状态
        toast: { show: false, text: '', type: '' },

        async init() {
            await this.loadEnvs();
            await this.refreshRuntime();
        },

        async loadEnvs() {
            const resp = await fetch('/api/remote-sync/envs');
            this.envs = await resp.json();
        },

        // ... 其他方法
    }
}
```

**状态更新方式**:
- 轮询: 定时调用`refreshRuntime()`
- SSE推送: 未在此页面直接使用，但可集成

**UI组件**:
- 环境列表 (左侧栏)
- 环境详情表单 (右侧主区域)
- 站点表格
- Toast通知
- 模态框 (环境/站点编辑)

#### 3.2 数据库状态监控页面

**文件**: `src/web_server/db_status_template.rs`

**特点**:
- 完全自包含HTML (无外部依赖)
- 原生JavaScript (无框架)
- 内联CSS
- 卡片式布局

**核心功能**:
1. 数据库列表展示
2. 版本检测
3. 增量更新触发
4. 自动更新配置
5. 批量操作

**状态更新**:
- 手动刷新按钮
- 定时轮询 (可选)

**样式风格**:
```css
body {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.db-card {
    background: white;
    border-radius: 10px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
}
```

#### 3.3 增量更新监控页面

**文件**: `src/web_server/static/incremental_update.js` (197KB)

**特性**:
- 大型SPA应用
- 复杂状态管理
- Chart.js图表集成
- 实时进度条

**核心模块**:
- 站点状态监控
- 增量检测触发
- 同步任务管理
- 日志查看
- 统计图表

**API集成** (来自`incremental_update_handlers.rs`):
- GET `/api/incremental/status` - 获取所有站点状态
- GET `/api/incremental/site/:site_id` - 站点详情
- POST `/api/incremental/detect/:site_id` - 启动检测
- POST `/api/incremental/sync/:site_id` - 启动同步
- GET `/api/incremental/logs` - 日志查询
- GET `/api/incremental/stats` - 统计数据

---

### 4. 路由和Handler模式

#### 4.1 路由注册标准流程

**位置**: `src/web_server/mod.rs` 第254行开始

**模式1: 简单GET页面**
```rust
.route("/dashboard", get(dashboard_page))
```

**模式2: RESTful API**
```rust
.route("/api/tasks", get(get_tasks).post(create_task))
.route("/api/tasks/{id}",
    get(get_task)
    .put(update_task)
    .delete(delete_task)
)
```

**模式3: SSE端点**
```rust
.route("/api/sync/events/stream", get(sse_handlers::sync_events_handler))
```

**模式4: WebSocket端点**
```rust
.route("/ws/progress/{task_id}", get(ws::ws_progress_handler))
```

**模式5: 嵌套路由**
```rust
let room_routes = room_api::create_room_api_routes().with_state(room_api_state);
app.merge(room_routes)
```

#### 4.2 Handler函数标准模式

**基础模式** (无状态):
```rust
pub async fn get_config() -> Result<Json<DatabaseConfig>, StatusCode> {
    let config = /* 获取配置 */;
    Ok(Json(config))
}
```

**带状态模式**:
```rust
pub async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<TaskInfo>, StatusCode> {
    let mut manager = state.task_manager.lock().await;
    let task = manager.create_task(req);
    Ok(Json(task))
}
```

**带路径参数**:
```rust
pub async fn get_task(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<TaskInfo>, StatusCode> {
    let manager = state.task_manager.lock().await;
    match manager.get_task(&id) {
        Some(task) => Ok(Json(task)),
        None => Err(StatusCode::NOT_FOUND),
    }
}
```

**带查询参数**:
```rust
pub async fn get_tasks(
    Query(params): Query<TaskQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<TaskInfo>>, StatusCode> {
    let manager = state.task_manager.lock().await;
    let tasks = manager.filter_tasks(params);
    Ok(Json(tasks))
}
```

**HTML页面模式**:
```rust
pub async fn dashboard_page() -> Html<String> {
    let content = render_dashboard_content();
    Html(layout::render_layout_with_sidebar(
        "仪表板",
        Some("dashboard"),
        &content,
        None,
        None
    ))
}
```

**SSE处理器模式** (参见`sse_handlers.rs`):
```rust
pub async fn sync_events_handler() -> impl IntoResponse {
    let rx = SYNC_EVENT_TX.subscribe();
    let stream = BroadcastStream::new(rx);
    let event_stream = stream.filter_map(|result| async move {
        // 转换为SSE Event
    });
    Sse::new(event_stream).keep_alive(KeepAlive::default())
}
```

**WebSocket升级模式** (参见`ws/progress.rs`):
```rust
pub async fn ws_progress_handler(
    ws: WebSocketUpgrade,
    Path(task_id): Path<String>,
    State(state): State<AppState>,
) -> Response {
    let hub = state.progress_hub.clone();
    ws.on_upgrade(move |socket| handle_progress_socket(socket, task_id, hub))
}
```

#### 4.3 错误处理模式

**简单错误响应**:
```rust
Err(StatusCode::NOT_FOUND)
Err(StatusCode::INTERNAL_SERVER_ERROR)
```

**JSON错误响应**:
```rust
Ok(Json(json!({
    "success": false,
    "error": "操作失败"
})))
```

**自定义错误类型**:
```rust
impl IntoResponse for MyError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(json!({ "error": self.message }))).into_response()
    }
}
```

---

### 5. AppState结构和共享数据

**定义位置**: `src/web_server/mod.rs` 第56-64行

```rust
#[derive(Clone)]
pub struct AppState {
    pub task_manager: Arc<Mutex<TaskManager>>,
    pub config_manager: Arc<RwLock<ConfigManager>>,
    pub progress_hub: Arc<crate::shared::ProgressHub>,
}
```

**组件说明**:
1. **task_manager**: 任务管理器，使用`Mutex`保护写操作
2. **config_manager**: 配置管理器，使用`RwLock`支持多读单写
3. **progress_hub**: 进度广播中心，所有任务进度推送的核心

**初始化** (第84-140行):
```rust
impl AppState {
    pub fn new() -> Self {
        let mut config_manager = ConfigManager::default();
        config_manager.add_template("default", DatabaseConfig { ... });

        let mut task_manager = TaskManager::default();
        let restored_tasks = wizard_handlers::restore_tasks_from_sqlite();
        for task in restored_tasks {
            task_manager.active_tasks.insert(task.id.clone(), task);
        }

        Self {
            task_manager: Arc::new(Mutex::new(task_manager)),
            config_manager: Arc::new(RwLock::new(config_manager)),
            progress_hub: Arc::new(crate::shared::ProgressHub::default()),
        }
    }
}
```

**全局状态** (除AppState外):
- `SYNC_CONTROL_CENTER` - 同步控制中心 (`sync_control_center.rs`)
- `SYNC_EVENT_TX` - SSE广播通道 (`sync_control_center.rs`)
- `MQTT_SERVER_PROCESS` - MQTT服务器进程句柄
- `REMOTE_RUNTIME` - 远程运行时状态 (`remote_runtime.rs`)

---

## 结论 (Conclusions)

### 核心发现

1. **统一布局系统已完善**: `layout.rs`提供了标准的侧边栏导航模板，所有新页面都应使用`render_layout_with_sidebar`函数

2. **实时通讯双轨制**:
   - SSE用于单向推送 (服务器 -> 客户端)
   - WebSocket用于双向通信和高频进度更新
   - 两者可共存，SSE更简单，WebSocket更灵活

3. **ProgressHub是进度推送的核心**: 所有需要实时进度反馈的任务都应该集成ProgressHub，而不是自己实现广播机制

4. **Alpine.js是主流前端框架**: 现有大部分监控页面都使用Alpine.js，建议新页面保持一致

5. **Dashboard页面需要新建**: 虽然路由已注册，但未找到对应的实现文件

6. **静态资源已经很丰富**: Chart.js、FontAwesome、XeoKit等库都已就绪，可直接使用

### 技术债务

1. **模板系统原始**: 手动字符串拼接容易出错，缺少IDE支持
2. **前端代码重复**: 每个页面都自己写AJAX调用，缺少统一的API客户端
3. **状态管理分散**: AppState、全局静态变量、各种manager混合使用
4. **文档不完整**: 大部分handler函数缺少文档注释

### 最佳实践建议

1. **新增Dashboard页面应该**:
   - 使用`render_layout_with_sidebar`布局
   - 集成ProgressHub订阅任务进度
   - 使用SSE接收实时事件 (`/api/sync/events/stream`)
   - 使用Alpine.js管理状态
   - 复用Chart.js展示统计图表

2. **API设计应该**:
   - 遵循RESTful约定
   - 统一错误响应格式
   - 提供完整的CRUD操作
   - 添加分页、过滤、排序支持

3. **进度推送应该**:
   - 使用ProgressHub而不是自建通道
   - 通过WebSocket推送高频更新 (>1次/秒)
   - 通过SSE推送事件通知
   - 提供当前状态查询API (供页面刷新时恢复)

---

## 关系 (Relations)

### 模块依赖关系

```
mod.rs (路由注册)
  ├─> layout.rs (页面布局)
  ├─> *_template.rs (HTML模板)
  ├─> *_handlers.rs (API处理)
  ├─> sse_handlers.rs (SSE推送)
  ├─> ws/progress.rs (WebSocket推送)
  └─> sync_control_center.rs (全局状态)
       └─> shared/progress_hub.rs (进度管理)
```

### 数据流关系

**SSE推送链路**:
```
事件源 -> SYNC_EVENT_TX.send()
      -> BroadcastStream
      -> sse_handlers::sync_events_handler()
      -> 前端EventSource
```

**WebSocket推送链路**:
```
任务执行 -> ProgressHub.publish()
        -> DashMap + broadcast channel
        -> ws::handle_progress_socket()
        -> 前端WebSocket
```

**页面渲染链路**:
```
用户访问 -> Axum路由
        -> handler函数
        -> *_template.rs
        -> layout::render_layout_with_sidebar()
        -> HTML响应
```

### 现有页面可复用组件

1. **remote_sync_template.rs** 提供:
   - Alpine.js环境/站点管理模式
   - Toast通知组件
   - 模态框组件
   - 表格+表单混合布局

2. **db_status_template.rs** 提供:
   - 卡片式布局
   - 筛选器组件
   - 批量操作模式

3. **incremental_update.js** 提供:
   - Chart.js图表集成
   - 复杂状态管理
   - 实时进度条

---

## 推荐实现路径

### Dashboard页面开发步骤

1. **创建模板文件**: `src/web_server/dashboard_template.rs`
2. **实现handler**: 在`handlers.rs`中添加`dashboard_page()`
3. **注册路由**: 已完成 (mod.rs 第810行)
4. **集成SSE**: 连接`/api/sync/events/stream`接收实时事件
5. **集成Chart.js**: 展示任务统计、同步速率、失败率等
6. **集成ProgressHub**: 通过WebSocket订阅`/ws/tasks`监听所有任务

### 可复用代码示例

**模板结构**:
```rust
// src/web_server/dashboard_template.rs
pub fn render_dashboard_page() -> String {
    let content = r#"
    <div x-data="dashboardApp()" x-init="init()">
      <!-- 卡片布局 -->
      <!-- Chart.js图表 -->
      <!-- 实时任务列表 -->
    </div>

    <script>
    function dashboardApp() {
        return {
            stats: {},
            tasks: [],
            evtSource: null,

            async init() {
                this.connectSSE();
                this.connectWebSocket();
                await this.loadStats();
            },

            connectSSE() {
                this.evtSource = new EventSource('/api/sync/events/stream');
                this.evtSource.onmessage = (e) => {
                    const event = JSON.parse(e.data);
                    this.handleEvent(event);
                };
            },

            // ... 其他方法
        }
    }
    </script>
    "#;

    crate::web_server::layout::render_layout_with_sidebar(
        "仪表板",
        Some("dashboard"),
        content,
        Some("<script src='/static/chart.umd.min.js'></script>"),
        None
    )
}
```

**Handler实现**:
```rust
// src/web_server/handlers.rs
pub async fn dashboard_page() -> Html<String> {
    Html(dashboard_template::render_dashboard_page())
}
```

---

**报告完成时间**: 2025-11-20
**总代码文件数**: 20+
**总代码行数**: ~5000行 (排除静态库)
**调查深度**: P0 (架构级) + P1 (实现级)
