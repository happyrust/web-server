# 异地更新系统开发文档

## 目录

1. [系统概述](#系统概述)
2. [架构设计](#架构设计)
3. [核心组件](#核心组件)
4. [运行流程](#运行流程)
5. [数据模型](#数据模型)
6. [API 接口](#api-接口)
7. [配置说明](#配置说明)
8. [部署指南](#部署指南)
9. [测试验证](#测试验证)
10. [故障排查](#故障排查)
11. [性能优化](#性能优化)
12. [开发规范](#开发规范)

---

## 系统概述

### 1.1 功能定位

异地更新系统（Remote Sync System）是一个用于在不同地理位置的站点之间自动同步 PDMS 数据库增量变更的分布式系统。

**核心功能**：
- 实时监听本地 PDMS 数据文件变化
- 自动检测增量更新并生成压缩包
- 通过 MQTT 发布变更通知
- 支持多目标站点的并发同步
- 提供本地文件复制和 HTTP 远程传输两种方式
- 实时监控、日志记录和告警机制

### 1.2 技术栈

**后端核心**：
- Rust (异步运行时: Tokio)
- SurrealDB (数据存储)
- SQLite (配置管理)
- MQTT (消息通知)
- Axum (HTTP 服务)
- notify (文件监听)

**前端**：
- Next.js 14
- React
- TypeScript
- Tailwind CSS
- SSE (Server-Sent Events)

### 1.3 系统特性

- ✅ **实时监控**: 基于 notify 的文件系统监听
- ✅ **增量同步**: 只传输变更的数据元素
- ✅ **断线重连**: MQTT 自动重连机制
- ✅ **任务队列**: 优先级队列和并发控制
- ✅ **容错重试**: 自动重试失败任务
- ✅ **多目标**: 支持一对多同步
- ✅ **双模式**: 本地文件和 HTTP 传输
- ✅ **实时反馈**: SSE 推送状态更新

---

## 架构设计

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                         异地更新系统架构                           │
└─────────────────────────────────────────────────────────────────┘

┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│   源端环境    │         │  同步控制中心  │         │   目标站点    │
│              │         │              │         │              │
│ ┌──────────┐ │         │ ┌──────────┐ │         │ ┌──────────┐ │
│ │File      │ │  MQTT   │ │Task      │ │  HTTP   │ │File      │ │
│ │Watcher   │─┼────────▶│ │Queue     │─┼────────▶│ │Receiver  │ │
│ └──────────┘ │         │ └──────────┘ │         │ └──────────┘ │
│              │         │              │         │              │
│ ┌──────────┐ │         │ ┌──────────┐ │         │ ┌──────────┐ │
│ │Increment │ │         │ │Worker    │ │         │ │Metadata  │ │
│ │Detector  │ │         │ │Pool      │ │         │ │Manager   │ │
│ └──────────┘ │         │ └──────────┘ │         │ └──────────┘ │
│              │         │              │         │              │
│ ┌──────────┐ │         │ ┌──────────┐ │         │              │
│ │CBA       │ │         │ │SQLite    │ │         │              │
│ │Compressor│ │         │ │Logger    │ │         │              │
│ └──────────┘ │         │ └──────────┘ │         │              │
└──────────────┘         └──────────────┘         └──────────────┘
       │                        │                        │
       └────────────────────────┴────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │   Web UI         │
                    │   - 监控面板      │
                    │   - 配置管理      │
                    │   - 日志查询      │
                    └──────────────────┘
```

### 2.2 核心模块划分

#### 2.2.1 源端模块 (Source Side)

| 模块 | 职责 | 实现位置 |
|------|------|---------|
| **File Watcher** | 监听 PDMS 数据文件变化 | `increment_manager.rs::async_watch()` |
| **Increment Detector** | 检测增量更新（会话号对比） | `increment_manager.rs::execute_incr_update()` |
| **CBA Compressor** | 生成压缩归档文件 | `pdms_io::compress::execute_compress()` |
| **MQTT Publisher** | 发布变更通知 | `increment_manager.rs::mqtt_client.publish()` |

#### 2.2.2 控制中心模块 (Control Center)

| 模块 | 职责 | 实现位置 |
|------|------|---------|
| **SyncControlCenter** | 全局状态和任务队列管理 | `sync_control_center.rs` |
| **Task Queue** | 优先级队列 | `SyncControlCenter::task_queue` |
| **Worker Pool** | 后台任务执行器 | `SyncControlCenter::spawn_worker()` |
| **SQLite Logger** | 持久化日志记录 | `sync_control_center.rs::persist_task_*()` |
| **Config Manager** | 环境和站点配置 | `remote_sync_handlers.rs` |

#### 2.2.3 目标端模块 (Target Side)

| 模块 | 职责 | 实现位置 |
|------|------|---------|
| **File Receiver** | 接收 HTTP PUT 请求 | `remote_sync_smoke_test.rs::handle_put()` |
| **Metadata Manager** | 更新 metadata.json | `site_metadata.rs` |
| **Storage Handler** | 文件存储处理 | `process_sync_task()` |

#### 2.2.4 运行时管理 (Runtime)

| 模块 | 职责 | 实现位置 |
|------|------|---------|
| **RemoteRuntime** | 管理 Watcher 和 MQTT 生命周期 | `remote_runtime.rs` |
| **Monitoring** | 连接状态监控和告警 | `sync_control_center.rs::start_monitoring()` |
| **SSE Broadcaster** | 实时事件推送 | `sse_handlers.rs` |

### 2.3 数据流图

```
┌─────────────┐
│ PDMS File   │ (*.db 文件变化)
└─────┬───────┘
      │ notify::Event
      ▼
┌─────────────────────────────┐
│ File Watcher                │
│ - 扫描文件头                 │
│ - 读取 sesno (会话号)        │
└─────┬───────────────────────┘
      │ DbPageBasicInfo
      ▼
┌─────────────────────────────┐
│ Increment Detector          │
│ - 查询数据库中的 sesno       │
│ - 对比是否有增量             │
└─────┬───────────────────────┘
      │ [有增量] RangeInclusive<sesno>
      ▼
┌─────────────────────────────┐
│ Increment Processor         │
│ - collect_increment_eles()  │
│ - update_to_database()      │
│ - execute_compress()        │
└─────┬───────────────────────┘
      │ .cba file + hash
      ├──────────────┬─────────────┐
      │              │             │
      ▼              ▼             ▼
┌─────────┐   ┌──────────┐   ┌─────────────┐
│ MQTT    │   │SurrealDB │   │Task Queue   │
│ Publish │   │UPDATE    │   │enqueue_task │
└─────────┘   └──────────┘   └─────┬───────┘
                                    │
                                    ▼
                        ┌───────────────────────┐
                        │ SQLite Logs           │
                        │ INSERT remote_sync_   │
                        │ logs (status=pending) │
                        └───────┬───────────────┘
                                │
                                ▼
                        ┌───────────────────┐
                        │ Worker Thread     │
                        │ get_next_task()   │
                        └───────┬───────────┘
                                │
                ┌───────────────┴───────────────┐
                │                               │
                ▼                               ▼
    ┌───────────────────┐         ┌────────────────────┐
    │ Local File Copy   │         │ HTTP PUT Upload    │
    │ fs::copy()        │         │ reqwest::put()     │
    └───────┬───────────┘         └────────┬───────────┘
            │                              │
            └──────────┬───────────────────┘
                       │
                       ▼
            ┌──────────────────────┐
            │ Update Metadata      │
            │ metadata.json        │
            └──────────┬───────────┘
                       │
                       ▼
            ┌──────────────────────┐
            │ Complete Task        │
            │ UPDATE logs          │
            │ (status=completed)   │
            └──────────┬───────────┘
                       │
                       ▼
            ┌──────────────────────┐
            │ SSE Event Push       │
            │ SyncCompleted        │
            └──────────────────────┘
```

---

## 核心组件

### 3.1 SyncControlCenter (同步控制中心)

**文件位置**: `src/web_server/sync_control_center.rs`

#### 3.1.1 数据结构

```rust
pub struct SyncControlCenter {
    /// 当前状态
    pub state: SyncControlState,
    /// 同步配置
    pub config: SyncConfig,
    /// 任务队列（按优先级排序）
    pub task_queue: Vec<SyncTask>,
    /// 运行中的任务
    pub running_tasks: HashMap<String, SyncTask>,
    /// 历史记录（最近100条）
    pub history: Vec<SyncTask>,
    /// MQTT 服务器状态
    pub mqtt_server: Option<MqttServerState>,
    /// 后台处理任务句柄
    pub worker_handle: Option<JoinHandle<()>>,
}
```

#### 3.1.2 核心方法

```rust
impl SyncControlCenter {
    /// 启动同步服务
    pub async fn start(&mut self, env_id: String) -> anyhow::Result<()>
    
    /// 停止同步服务
    pub async fn stop(&mut self) -> anyhow::Result<()>
    
    /// 暂停/恢复同步
    pub fn pause(&mut self) -> anyhow::Result<()>
    pub fn resume(&mut self) -> anyhow::Result<()>
    
    /// 添加同步任务
    pub fn add_task(&mut self, params: NewSyncTaskParams) -> String
    
    /// 获取下一个待处理任务（考虑并发限制）
    pub fn get_next_task(&mut self) -> Option<SyncTask>
    
    /// 完成任务（成功或失败）
    pub fn complete_task(&mut self, task_id: &str, success: bool, error: Option<String>)
    
    /// 取消待处理任务
    pub fn cancel_pending_task(&mut self, task_id: &str, reason: &str) -> bool
    
    /// 清空队列
    pub fn clear_queue(&mut self, reason: &str) -> usize
    
    /// 更新统计信息
    pub fn update_statistics(&mut self)
    
    /// 启动后台 Worker
    fn spawn_worker(&mut self)
}
```

#### 3.1.3 任务状态机

```
┌─────────┐
│ Pending │ ◀─┐ (重试)
└────┬────┘   │
     │        │
     ▼        │
┌─────────┐   │
│ Running │   │
└────┬────┘   │
     │        │
     ├────────┼─────────┐
     │        │         │
     ▼        ▼         ▼
┌──────────┐ ┌─────┐ ┌──────────┐
│Completed │ │Failed│ │Cancelled │
└──────────┘ └─────┘ └──────────┘
```

### 3.2 RemoteRuntime (运行时管理)

**文件位置**: `src/web_server/remote_runtime.rs`

#### 3.2.1 数据结构

```rust
pub struct RuntimeState {
    /// 当前环境ID
    pub env_id: String,
    /// 数据库管理器
    pub mgr: Arc<AiosDBManager>,
    /// Watcher 任务句柄
    pub watcher_handle: Option<tokio::task::JoinHandle<()>>,
    /// MQTT 任务句柄
    pub mqtt_handle: Option<tokio::task::JoinHandle<()>>,
}

pub static REMOTE_RUNTIME: Lazy<RwLock<Option<RuntimeState>>> = 
    Lazy::new(|| RwLock::new(None));
```

#### 3.2.2 核心方法

```rust
/// 停止当前运行态
pub async fn stop_runtime()

/// 启动新运行态
pub async fn start_runtime(env_id: String) -> anyhow::Result<()> {
    // 1. 查询 MQTT 重连参数
    let (init_ms, max_ms) = query_backoff_ms(&env_id)?;
    
    // 2. 初始化数据库管理器
    let mgr = Arc::new(AiosDBManager::init_form_config().await?);
    mgr.init_watcher().await.ok();
    
    // 3. 启动文件监听
    let watcher_handle = tokio::spawn(async move {
        let _ = mgr_clone.async_watch().await;
    });
    
    // 4. 启动 MQTT 订阅
    let mqtt_handle = tokio::spawn(async move {
        AiosDBManager::poll_sync_e3d_mqtt_events_with_backoff(
            watcher_arc, init_ms, max_ms
        ).await;
    });
    
    // 5. 保存到全局状态
    *REMOTE_RUNTIME.write().await = Some(RuntimeState { ... });
}
```

### 3.3 AiosDBManager (增量管理器)

**文件位置**: `src/data_interface/increment_manager.rs`

#### 3.3.1 核心方法

```rust
impl AiosDBManager {
    /// 初始化文件监听器
    pub async fn init_watcher(&self) -> anyhow::Result<()>
    
    /// 异步监听文件变化（主循环）
    pub async fn async_watch(&self) -> anyhow::Result<()>
    
    /// 执行增量更新
    pub async fn execute_incr_update(
        &self,
        increment_ranges_map: IndexMap<PathBuf, (DbPageBasicInfo, RangeInclusive<i32>)>
    ) -> anyhow::Result<bool>
    
    /// MQTT 事件订阅（带重连）
    pub async fn poll_sync_e3d_mqtt_events_with_backoff(
        watcher: Arc<RwLock<PdmsWatcher>>,
        initial_interval_ms: u64,
        max_interval_ms: u64,
    )
}
```

#### 3.3.2 文件监听流程

```rust
// 伪代码展示核心逻辑
async fn async_watch(&self) -> anyhow::Result<()> {
    loop {
        select! {
            event = rx.recv() => {
                // 1. 扫描文件头获取 sesno
                let basic_info = parse_db_basic_info(&path)?;
                
                // 2. 查询数据库中的 sesno
                let db_sesno = query_latest_sesno_by_file_name(&file_name).await?;
                
                // 3. 判断是否有增量
                if basic_info.sesno > db_sesno {
                    // 4. 收集增量元素
                    let increment_eles = io.collect_increment_eles(range)?;
                    
                    // 5. 更新到 SurrealDB
                    io.update_elements_to_database(&increment_eles).await?;
                    
                    // 6. 生成压缩包
                    let cba_path = execute_compress(&CompressOptions { ... })?;
                    
                    // 7. 发布 MQTT 消息
                    mqtt_client.publish("Sync/E3d", payload).await?;
                    
                    // 8. 入队同步任务
                    enqueue_generated_sync_tasks(artifacts).await;
                }
            }
        }
    }
}
```

### 3.4 process_sync_task (任务执行器)

**文件位置**: `src/web_server/sync_control_center.rs:584`

#### 3.4.1 执行流程

```rust
pub(crate) async fn process_sync_task(task: &SyncTask) -> anyhow::Result<()> {
    // 1. 验证文件存在
    let metadata = fs::metadata(&task.file_path).await?;
    
    // 2. 解析目标位置
    let destination = resolve_sync_destination(task.clone()).await?;
    
    // 3. 根据目标类型执行同步
    match &destination.target {
        ResolvedTarget::Local { final_path } => {
            // 本地文件复制
            fs::create_dir_all(parent).await?;
            fs::copy(&task.file_path, &final_path).await?;
            
            // 更新站点元数据
            update_site_metadata(base, &destination, &task, &final_path, file_size).await?;
        }
        ResolvedTarget::Http { url } => {
            // HTTP 上传
            let data = fs::read(&task.file_path).await?;
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?;
            let response = client.put(&url).body(data).send().await?;
            
            if !response.status().is_success() {
                return Err(anyhow!("HTTP upload failed: {}", response.status()));
            }
            
            // 刷新远程元数据
            refresh_remote_site_metadata(&destination).await?;
        }
    }
    
    Ok(())
}
```

#### 3.4.2 目标解析逻辑

```rust
async fn resolve_sync_destination(task: SyncTask) -> anyhow::Result<SyncDestination> {
    // 1. 查询环境和站点配置（SQLite）
    let destination_ctx = spawn_blocking(|| {
        let conn = remote_sync_handlers::open_sqlite()?;
        
        // 查询环境信息
        let (env_name, env_file_host) = conn.query_row(
            "SELECT name, file_server_host FROM remote_sync_envs WHERE id = ?1",
            [env_id]
        )?;
        
        // 查询站点信息
        let (site_id, site_name, site_http_host) = conn.query_row(
            "SELECT id, name, http_host FROM remote_sync_sites WHERE id = ?1 OR name = ?1",
            [site_identifier]
        )?;
        
        Ok(DestinationContext { ... })
    }).await??;
    
    // 2. 构建路径段
    let mut path_segments = vec![];
    if let Some(env) = destination_ctx.env_name {
        path_segments.push(sanitize_path_segment(env));
    }
    if let Some(site) = destination_ctx.site_name {
        path_segments.push(sanitize_path_segment(site));
    }
    if let Some(direction) = task.direction {
        path_segments.push(sanitize_path_segment(direction));
    }
    
    // 3. 判断目标类型
    if let Some(http_base) = http_base {
        // HTTP URL
        let mut url = http_base.trim_end_matches('/').to_string();
        for segment in &path_segments {
            url.push('/');
            url.push_str(segment);
        }
        url.push('/');
        url.push_str(&file_name);
        
        return Ok(SyncDestination {
            target: ResolvedTarget::Http { url },
            ...
        });
    }
    
    // 本地路径
    let mut final_path = local_base.unwrap_or_else(|| PathBuf::from("output/remote_sync"));
    for segment in &path_segments {
        final_path.push(segment);
    }
    final_path.push(&file_name);
    
    Ok(SyncDestination {
        target: ResolvedTarget::Local { final_path },
        ...
    })
}
```

---

## 运行流程

### 4.1 服务启动流程

```
┌──────────────────────────────────────────────────────────────┐
│ 用户触发: POST /api/remote-sync/control/start                 │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ SyncControlCenter::start(env_id)   │
        └────────────────────────────────────┘
                            │
        ┌───────────────────┴───────────────┐
        │                                   │
        ▼                                   ▼
┌────────────────┐              ┌────────────────────┐
│ 停止旧运行时    │              │ 读取环境配置         │
│ stop_runtime() │              │ - SQLite 查询       │
└────────────────┘              │ - MQTT 参数         │
        │                       └────────────────────┘
        │                                   │
        └───────────────────┬───────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ start_runtime(env_id)              │
        └────────────────────────────────────┘
                            │
        ┌───────────────────┴───────────────┐
        │                                   │
        ▼                                   ▼
┌────────────────┐              ┌────────────────────┐
│ 初始化数据库    │              │ 初始化文件监听器     │
│ AiosDBManager  │              │ init_watcher()     │
│ ::init()       │              └────────────────────┘
└────────────────┘                          │
        │                                   ▼
        │                       ┌────────────────────┐
        │                       │ 启动文件监听任务     │
        │                       │ async_watch()      │
        │                       └────────────────────┘
        │                                   │
        └───────────────────┬───────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 启动 MQTT 订阅任务                  │
        │ poll_sync_e3d_mqtt_events()        │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 启动后台 Worker                     │
        │ spawn_worker()                     │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 启动监控任务                        │
        │ start_monitoring()                 │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 更新状态                            │
        │ - is_running = true                │
        │ - mqtt_connected = true            │
        │ - watcher_active = true            │
        │ - started_at = now()               │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 发送 SSE 事件                       │
        │ SyncEvent::Started                 │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 返回成功响应                        │
        └────────────────────────────────────┘
```

### 4.2 增量检测与同步流程

```
┌──────────────────────────────────────────────────────────────┐
│ 文件系统事件: data/CATA.db 被修改                              │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ notify::Event::Modify               │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ async_watch() 接收事件              │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 扫描文件头                          │
        │ parse_db_basic_info(&path)         │
        │ -> DbPageBasicInfo {               │
        │      sesno: 12345,                 │
        │      ...                           │
        │    }                               │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 查询数据库中的会话号                 │
        │ query_latest_sesno("CATA")         │
        │ -> db_sesno: 12340                 │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 判断是否有增量                       │
        │ file_sesno (12345) > db_sesno?     │
        └────────────────────────────────────┘
                        [是] │
                            ▼
        ┌────────────────────────────────────┐
        │ 收集增量元素                        │
        │ collect_increment_eles(            │
        │   range: 12341..=12345             │
        │ )                                  │
        │ -> Vec<IncrementInfo>              │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 更新到 SurrealDB                    │
        │ update_elements_to_database()      │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 生成压缩包                          │
        │ execute_compress()                 │
        │ -> assets/archives/CATA_xxx.cba    │
        │    file_hash: "abc123..."          │
        └────────────────────────────────────┘
                            │
        ┌───────────────────┴───────────────┐
        │                                   │
        ▼                                   ▼
┌────────────────┐              ┌────────────────────┐
│ 发布 MQTT       │              │ 插入 SurrealDB      │
│ Topic:         │              │ INSERT INTO        │
│ "Sync/E3d"     │              │ e3d_sync           │
│ Payload: {     │              └────────────────────┘
│   files: [...],│
│   hashes: [...] │
│ }              │
└────────────────┘
        │
        ▼
┌────────────────────────────────────┐
│ enqueue_generated_sync_tasks()     │
└────────────────────────────────────┘
        │
        ▼
┌────────────────────────────────────┐
│ 查询环境和站点配置                  │
│ SELECT * FROM remote_sync_envs     │
│ SELECT * FROM remote_sync_sites    │
└────────────────────────────────────┘
        │
        ▼
┌────────────────────────────────────┐
│ 为每个站点创建任务                  │
│ SyncControlCenter::add_task() {    │
│   file_path: "assets/.../xxx.cba"  │
│   env_id: "env-001"                │
│   site_id: "site-001"              │
│   direction: "UPLOAD"              │
│   priority: 5                      │
│ }                                  │
└────────────────────────────────────┘
        │
        ▼
┌────────────────────────────────────┐
│ 持久化任务日志                      │
│ INSERT INTO remote_sync_logs       │
│ SET status = 'pending'             │
└────────────────────────────────────┘
        │
        ▼
┌────────────────────────────────────┐
│ 任务入队                            │
│ task_queue.push(task)              │
│ sort_by(priority)                  │
└────────────────────────────────────┘
```

### 4.3 Worker 任务执行流程

```
┌──────────────────────────────────────────────────────────────┐
│ Worker 后台循环 (每 500ms)                                     │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 检查运行状态                        │
        │ is_running? is_paused?             │
        └────────────────────────────────────┘
                        [运行中] │
                            ▼
        ┌────────────────────────────────────┐
        │ 获取下一个任务                      │
        │ get_next_task()                    │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 检查并发限制                        │
        │ running_tasks.len() <              │
        │ max_concurrent_syncs (5)?          │
        └────────────────────────────────────┘
                        [是] │
                            ▼
        ┌────────────────────────────────────┐
        │ 从队列中取出任务                    │
        │ task_queue.remove(index)           │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 更新任务状态                        │
        │ task.status = Running              │
        │ task.started_at = now()            │
        │ running_tasks.insert(task.id, task)│
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 持久化运行状态                      │
        │ UPDATE remote_sync_logs            │
        │ SET status='running',              │
        │     started_at=now()               │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 执行同步任务                        │
        │ process_sync_task(&task)           │
        └────────────────────────────────────┘
                            │
                ┌───────────┴───────────┐
                │                       │
          [成功] ▼                       ▼ [失败]
    ┌────────────────┐        ┌────────────────┐
    │ Ok(())         │        │ Err(error)     │
    └────────────────┘        └────────────────┘
                │                       │
                ▼                       ▼
    ┌────────────────┐        ┌────────────────────┐
    │ complete_task  │        │ complete_task      │
    │ success=true   │        │ success=false      │
    └────────────────┘        │ error=Some(msg)    │
                │             └────────────────────┘
                │                       │
                │                       ▼
                │             ┌────────────────────┐
                │             │ 检查重试            │
                │             │ retry_count < 3?   │
                │             └────────────────────┘
                │                   [是] │
                │                       ▼
                │             ┌────────────────────┐
                │             │ 重新入队            │
                │             │ status=Pending     │
                │             │ retry_count++      │
                │             └────────────────────┘
                │                       │
                └───────────┬───────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 持久化最终状态                      │
        │ UPDATE remote_sync_logs            │
        │ SET status, completed_at,          │
        │     error_message                  │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 添加到历史                          │
        │ history.push(task)                 │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 发送 SSE 事件                       │
        │ SyncEvent::SyncCompleted /         │
        │ SyncEvent::SyncFailed              │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 从运行队列移除                      │
        │ running_tasks.remove(task.id)      │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 更新统计                            │
        │ total_synced++ / total_failed++    │
        │ avg_sync_time_ms                   │
        └────────────────────────────────────┘
                            │
                            ▼
        ┌────────────────────────────────────┐
        │ 继续循环                            │
        └────────────────────────────────────┘
```

---

## 数据模型

### 5.1 SQLite 数据库表结构

#### 5.1.1 remote_sync_envs (环境配置表)

```sql
CREATE TABLE remote_sync_envs (
    id TEXT PRIMARY KEY,                    -- 环境ID (UUID)
    name TEXT NOT NULL,                     -- 环境名称 (如: "bj-env")
    mqtt_host TEXT,                         -- MQTT 服务器地址
    mqtt_port INTEGER,                      -- MQTT 端口
    file_server_host TEXT,                  -- 文件服务器地址
    location TEXT,                          -- 位置标识 (如: "bj")
    location_dbs TEXT,                      -- 数据库列表 (JSON)
    reconnect_initial_ms INTEGER,           -- MQTT 初始重连间隔
    reconnect_max_ms INTEGER,               -- MQTT 最大重连间隔
    created_at TEXT NOT NULL,               -- 创建时间 (RFC3339)
    updated_at TEXT NOT NULL                -- 更新时间 (RFC3339)
);
```

**示例数据**：
```json
{
  "id": "env-001",
  "name": "bj-env",
  "mqtt_host": "127.0.0.1",
  "mqtt_port": 1883,
  "file_server_host": "http://192.168.1.100:8080/files",
  "location": "bj",
  "reconnect_initial_ms": 1000,
  "reconnect_max_ms": 30000,
  "created_at": "2025-01-15T10:00:00Z",
  "updated_at": "2025-01-15T10:00:00Z"
}
```

#### 5.1.2 remote_sync_sites (站点配置表)

```sql
CREATE TABLE remote_sync_sites (
    id TEXT PRIMARY KEY,                    -- 站点ID (UUID)
    env_id TEXT NOT NULL,                   -- 所属环境ID
    name TEXT NOT NULL,                     -- 站点名称 (如: "sjz-site")
    location TEXT,                          -- 位置标识 (如: "sjz")
    http_host TEXT,                         -- HTTP 接收地址
    dbnums TEXT,                            -- 数据库编号列表 (JSON)
    notes TEXT,                             -- 备注
    created_at TEXT NOT NULL,               -- 创建时间
    updated_at TEXT NOT NULL,               -- 更新时间
    FOREIGN KEY(env_id) REFERENCES remote_sync_envs(id) ON DELETE CASCADE
);
```

**示例数据**：
```json
{
  "id": "site-001",
  "env_id": "env-001",
  "name": "sjz-site",
  "location": "sjz",
  "http_host": "http://192.168.1.200:8080/files",
  "dbnums": "[1,2,3]",
  "notes": "石家庄站点",
  "created_at": "2025-01-15T10:00:00Z",
  "updated_at": "2025-01-15T10:00:00Z"
}
```

**http_host 字段说明**：
- **HTTP URL**: `http://192.168.1.200:8080/files` - 使用 HTTP PUT 上传
- **本地路径提示**: `local:/path/to/files` - 使用本地文件复制

#### 5.1.3 remote_sync_logs (同步日志表)

```sql
CREATE TABLE remote_sync_logs (
    id TEXT PRIMARY KEY,                    -- 日志ID (UUID)
    task_id TEXT,                           -- 任务ID
    env_id TEXT,                            -- 环境ID
    source_env TEXT,                        -- 源环境
    target_site TEXT,                       -- 目标站点
    site_id TEXT,                           -- 站点ID
    direction TEXT,                         -- 方向 (UPLOAD/DOWNLOAD)
    file_path TEXT,                         -- 文件路径
    file_size INTEGER,                      -- 文件大小 (字节)
    record_count INTEGER,                   -- 记录数
    status TEXT,                            -- 状态 (pending/running/completed/failed/cancelled)
    error_message TEXT,                     -- 错误信息
    notes TEXT,                             -- 备注
    started_at TEXT,                        -- 开始时间
    completed_at TEXT,                      -- 完成时间
    created_at TEXT NOT NULL,               -- 创建时间
    updated_at TEXT NOT NULL                -- 更新时间
);
```

**状态流转**：
```
pending → running → completed
                 → failed → pending (重试)
                 → cancelled
```

### 5.2 Rust 数据结构

#### 5.2.1 SyncTask (同步任务)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTask {
    pub id: String,                         // 任务ID (UUID)
    pub file_path: String,                  // 文件路径
    pub file_size: u64,                     // 文件大小
    pub file_name: Option<String>,          // 文件名
    pub file_hash: Option<String>,          // 文件哈希
    pub record_count: Option<u64>,          // 记录数
    pub env_id: Option<String>,             // 环境ID
    pub source_env: Option<String>,         // 源环境
    pub target_site: Option<String>,        // 目标站点
    pub direction: Option<String>,          // 方向
    pub notes: Option<String>,              // 备注
    pub status: SyncTaskStatus,             // 状态
    pub priority: u8,                       // 优先级 (1-10)
    pub retry_count: u32,                   // 重试次数
    pub created_at: SystemTime,             // 创建时间
    pub started_at: Option<SystemTime>,     // 开始时间
    pub completed_at: Option<SystemTime>,   // 完成时间
    pub error_message: Option<String>,      // 错误信息
}
```

#### 5.2.2 SyncControlState (控制状态)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncControlState {
    pub is_running: bool,                   // 服务是否运行
    pub is_paused: bool,                    // 是否暂停
    pub current_env: Option<String>,        // 当前环境ID
    pub env_name: Option<String>,           // 环境名称
    
    // 连接状态
    pub mqtt_connected: bool,               // MQTT 连接状态
    pub watcher_active: bool,               // Watcher 活跃状态
    pub last_mqtt_connect_time: Option<SystemTime>,
    pub mqtt_reconnect_count: u32,          // 重连次数
    
    // 同步统计
    pub total_synced: u64,                  // 总成功数
    pub total_failed: u64,                  // 总失败数
    pub pending_count: u32,                 // 待处理数
    pub queue_size: u32,                    // 队列大小
    
    // 性能指标
    pub sync_rate_mbps: f64,                // 同步速率 (Mbps)
    pub avg_sync_time_ms: u64,              // 平均同步时间 (ms)
    pub last_sync_time: Option<SystemTime>, // 最后同步时间
    
    // 运行时长
    pub started_at: Option<SystemTime>,     // 启动时间
    pub uptime_seconds: u64,                // 运行时长 (秒)
}
```

#### 5.2.3 SyncConfig (同步配置)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub env_id: String,                     // 环境ID
    pub auto_retry: bool,                   // 自动重试
    pub max_retries: u32,                   // 最大重试次数
    pub retry_delay_ms: u64,                // 重试延迟 (ms)
    pub max_concurrent_syncs: u32,          // 最大并发数
    pub batch_size: u32,                    // 批处理大小
    pub sync_interval_ms: u64,              // 同步间隔 (ms)
    pub auto_pause_on_error: bool,          // 错误时自动暂停
    pub alert_on_failure: bool,             // 失败时告警
}
```

**默认配置**：
```rust
impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            env_id: String::new(),
            auto_retry: true,
            max_retries: 3,
            retry_delay_ms: 5000,
            max_concurrent_syncs: 5,
            batch_size: 10,
            sync_interval_ms: 1000,
            auto_pause_on_error: false,
            alert_on_failure: true,
        }
    }
}
```

### 5.3 SSE 事件类型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SyncEvent {
    Started {
        env_id: String,
        timestamp: String,
    },
    Stopped {
        env_id: String,
        timestamp: String,
    },
    SyncCompleted {
        task_id: String,
        file_path: String,
        duration_ms: u64,
        timestamp: String,
    },
    SyncFailed {
        task_id: String,
        file_path: String,
        error: String,
        timestamp: String,
    },
    ProgressUpdate {
        total: u64,
        completed: u64,
        failed: u64,
        pending: u64,
        timestamp: String,
    },
    ConnectionChanged {
        mqtt_connected: bool,
        watcher_active: bool,
        timestamp: String,
    },
    Alert {
        level: String,              // "info" | "warning" | "error" | "critical"
        message: String,
        timestamp: String,
    },
}
```

---

## API 接口

### 6.1 同步控制接口

#### 6.1.1 启动同步服务

```http
POST /api/remote-sync/control/start
Content-Type: application/json

{
  "env_id": "env-001"
}
```

**响应**：
```json
{
  "success": true,
  "message": "同步服务已启动",
  "env_id": "env-001"
}
```

#### 6.1.2 停止同步服务

```http
POST /api/remote-sync/control/stop
```

**响应**：
```json
{
  "success": true,
  "message": "同步服务已停止"
}
```

#### 6.1.3 暂停/恢复同步

```http
POST /api/remote-sync/control/pause
POST /api/remote-sync/control/resume
```

#### 6.1.4 获取状态

```http
GET /api/remote-sync/control/state
```

**响应**：
```json
{
  "is_running": true,
  "is_paused": false,
  "current_env": "env-001",
  "env_name": "bj-env",
  "mqtt_connected": true,
  "watcher_active": true,
  "total_synced": 150,
  "total_failed": 5,
  "pending_count": 3,
  "queue_size": 3,
  "sync_rate_mbps": 12.5,
  "avg_sync_time_ms": 320,
  "uptime_seconds": 3600
}
```

### 6.2 任务管理接口

#### 6.2.1 手动添加任务

```http
POST /api/remote-sync/tasks
Content-Type: application/json

{
  "file_path": "assets/archives/CATA_20250115.cba",
  "file_size": 1024000,
  "priority": 5,
  "file_name": "CATA_20250115.cba",
  "env_id": "env-001",
  "target_site": "site-001",
  "direction": "UPLOAD"
}
```

**响应**：
```json
{
  "task_id": "task-uuid-xxx",
  "status": "pending"
}
```

#### 6.2.2 获取任务列表

```http
GET /api/remote-sync/tasks?status=pending&limit=20&offset=0
```

**响应**：
```json
{
  "tasks": [
    {
      "id": "task-001",
      "file_path": "assets/archives/CATA.cba",
      "file_size": 1024000,
      "status": "pending",
      "priority": 5,
      "created_at": "2025-01-15T10:00:00Z"
    }
  ],
  "total": 150
}
```

#### 6.2.3 取消任务

```http
DELETE /api/remote-sync/tasks/{task_id}
```

#### 6.2.4 清空队列

```http
DELETE /api/remote-sync/tasks/queue
```

### 6.3 配置管理接口

#### 6.3.1 获取环境列表

```http
GET /api/remote-sync/environments
```

#### 6.3.2 创建环境

```http
POST /api/remote-sync/environments
Content-Type: application/json

{
  "name": "bj-env",
  "mqtt_host": "127.0.0.1",
  "mqtt_port": 1883,
  "file_server_host": "http://192.168.1.100:8080/files",
  "location": "bj",
  "reconnect_initial_ms": 1000,
  "reconnect_max_ms": 30000
}
```

#### 6.3.3 获取站点列表

```http
GET /api/remote-sync/sites?env_id=env-001
```

#### 6.3.4 创建站点

```http
POST /api/remote-sync/sites
Content-Type: application/json

{
  "env_id": "env-001",
  "name": "sjz-site",
  "location": "sjz",
  "http_host": "http://192.168.1.200:8080/files"
}
```

### 6.4 日志查询接口

```http
GET /api/remote-sync/logs?env_id=env-001&status=completed&limit=100&offset=0
```

**响应**：
```json
{
  "logs": [
    {
      "id": "log-001",
      "task_id": "task-001",
      "file_path": "assets/archives/CATA.cba",
      "status": "completed",
      "started_at": "2025-01-15T10:00:00Z",
      "completed_at": "2025-01-15T10:00:05Z",
      "duration_ms": 5000
    }
  ],
  "total": 1500
}
```

### 6.5 SSE 事件流

```http
GET /api/remote-sync/events
Accept: text/event-stream
```

**事件流示例**：
```
event: sync_completed
data: {"type":"SyncCompleted","task_id":"task-001","file_path":"...","duration_ms":5000,"timestamp":"2025-01-15T10:00:05Z"}

event: progress_update
data: {"type":"ProgressUpdate","total":100,"completed":95,"failed":5,"pending":0,"timestamp":"2025-01-15T10:00:06Z"}
```

---

## 配置说明

### 7.1 DbOption.toml

```toml
# SQLite 数据库路径
deployment_sites_sqlite_path = "deployment_sites.sqlite"

# 数据文件目录
data_dir = "data"

# 输出目录
output_dir = "output"

# 归档目录
archives_dir = "assets/archives"
```

### 7.2 环境变量

```bash
# MQTT 配置
MQTT_HOST=127.0.0.1
MQTT_PORT=1883

# HTTP 服务端口
HTTP_PORT=8080

# 日志级别
RUST_LOG=info

# 最大并发数
MAX_CONCURRENT_SYNCS=5

# 重试配置
MAX_RETRIES=3
RETRY_DELAY_MS=5000
```

### 7.3 SQLite 初始化脚本

```sql
-- 创建环境表
CREATE TABLE IF NOT EXISTS remote_sync_envs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    mqtt_host TEXT,
    mqtt_port INTEGER,
    file_server_host TEXT,
    location TEXT,
    location_dbs TEXT,
    reconnect_initial_ms INTEGER,
    reconnect_max_ms INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 创建站点表
CREATE TABLE IF NOT EXISTS remote_sync_sites (
    id TEXT PRIMARY KEY,
    env_id TEXT NOT NULL,
    name TEXT NOT NULL,
    location TEXT,
    http_host TEXT,
    dbnums TEXT,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(env_id) REFERENCES remote_sync_envs(id) ON DELETE CASCADE
);

-- 创建日志表
CREATE TABLE IF NOT EXISTS remote_sync_logs (
    id TEXT PRIMARY KEY,
    task_id TEXT,
    env_id TEXT,
    source_env TEXT,
    target_site TEXT,
    site_id TEXT,
    direction TEXT,
    file_path TEXT,
    file_size INTEGER,
    record_count INTEGER,
    status TEXT,
    error_message TEXT,
    notes TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_logs_env_id ON remote_sync_logs(env_id);
CREATE INDEX IF NOT EXISTS idx_logs_status ON remote_sync_logs(status);
CREATE INDEX IF NOT EXISTS idx_logs_created_at ON remote_sync_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sites_env_id ON remote_sync_sites(env_id);
```

---

## 部署指南

### 8.1 本地开发环境

#### 8.1.1 前置要求

- Rust 1.75+
- Node.js 18+
- SQLite 3
- MQTT Broker (如 mosquitto)

#### 8.1.2 启动步骤

```bash
# 1. 克隆项目
git clone <repo-url>
cd web-server

# 2. 安装依赖
cargo build --features web_server

# 3. 初始化数据库
sqlite3 deployment_sites.sqlite < scripts/init_db.sql

# 4. 配置 DbOption.toml
cat > DbOption.toml <<EOF
deployment_sites_sqlite_path = "deployment_sites.sqlite"
EOF

# 5. 启动 MQTT Broker
mosquitto -c mosquitto.conf

# 6. 启动后端服务
cargo run --features web_server --bin aios_database

# 7. 启动前端 (另一个终端)
cd frontend/v0-aios-database-management
npm install
npm run dev
```

### 8.2 生产环境部署

#### 8.2.1 Docker 部署

```dockerfile
# Dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features web_server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/aios_database /usr/local/bin/
COPY DbOption.toml /etc/aios/
COPY deployment_sites.sqlite /var/lib/aios/

EXPOSE 8080
CMD ["aios_database"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  aios-backend:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./data:/app/data
      - ./output:/app/output
      - ./assets:/app/assets
    environment:
      - RUST_LOG=info
      - MAX_CONCURRENT_SYNCS=5
    depends_on:
      - mosquitto
      
  mosquitto:
    image: eclipse-mosquitto:2
    ports:
      - "1883:1883"
    volumes:
      - ./mosquitto.conf:/mosquitto/config/mosquitto.conf
```

#### 8.2.2 systemd 服务

```ini
# /etc/systemd/system/aios-sync.service
[Unit]
Description=AIOS Remote Sync Service
After=network.target

[Service]
Type=simple
User=aios
WorkingDirectory=/opt/aios
ExecStart=/usr/local/bin/aios_database
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
# 启用服务
sudo systemctl enable aios-sync
sudo systemctl start aios-sync
sudo systemctl status aios-sync
```

---

## 测试验证

### 9.1 烟雾测试

**测试文件**: `src/bin/remote_sync_smoke_test.rs`

```bash
# 运行烟雾测试
cargo test --features web_server --bin remote_sync_smoke_test
```

**测试覆盖**：
1. ✅ 创建临时环境和站点配置
2. ✅ 启动本地 HTTP 文件接收器
3. ✅ 生成假的 .cba 归档文件
4. ✅ 通过 SyncControlCenter 入队任务
5. ✅ 执行 process_sync_task 上传文件
6. ✅ 验证远程文件已接收
7. ✅ 验证 SQLite 日志已记录

### 9.2 集成测试

```rust
#[tokio::test]
async fn test_full_sync_workflow() {
    // 1. 启动服务
    let mut center = SyncControlCenter::new();
    center.start("test-env".to_string()).await.unwrap();
    
    // 2. 添加任务
    let task_id = center.add_task(NewSyncTaskParams {
        file_path: "test.cba".to_string(),
        file_size: 1024,
        priority: 5,
        ...
    });
    
    // 3. 等待完成
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // 4. 验证状态
    let state = center.get_state_snapshot();
    assert_eq!(state.total_synced, 1);
}
```

### 9.3 性能测试

```bash
# 100 个并发任务
for i in {1..100}; do
  curl -X POST http://localhost:8080/api/remote-sync/tasks \
    -H "Content-Type: application/json" \
    -d "{\"file_path\":\"test_$i.cba\",\"file_size\":1024,\"priority\":5}"
done

# 监控统计
watch -n 1 'curl -s http://localhost:8080/api/remote-sync/control/state | jq'
```

---

## 故障排查

### 10.1 常见问题

#### 10.1.1 MQTT 连接失败

**症状**：
```
mqtt_connected: false
mqtt_reconnect_count: 10
```

**排查步骤**：
1. 检查 MQTT Broker 是否运行: `systemctl status mosquitto`
2. 验证网络连通性: `telnet <mqtt_host> 1883`
3. 查看日志: `journalctl -u mosquitto -f`
4. 检查配置表中的 mqtt_host 和 mqtt_port

#### 10.1.2 文件监听不工作

**症状**：
```
watcher_active: false
```

**排查步骤**：
1. 检查数据目录权限: `ls -la data/`
2. 验证 notify 监听器已启动: 查看日志中的 "init_watcher" 消息
3. 检查文件系统是否支持 inotify (Linux)
4. 尝试手动触发: `touch data/CATA.db`

#### 10.1.3 任务一直 Pending

**症状**：
```
pending_count: 50
queue_size: 50
total_synced: 0
```

**排查步骤**：
1. 检查 Worker 是否运行: `worker_handle` 不为 None
2. 验证并发限制: `running_tasks.len() < max_concurrent_syncs`
3. 检查是否暂停: `is_paused: false`
4. 查看错误日志

#### 10.1.4 HTTP 上传失败

**症状**：
```
SyncEvent::SyncFailed {
  error: "HTTP upload failed: 404"
}
```

**排查步骤**：
1. 验证目标 URL: `curl -X PUT <http_host>/test.txt -d "test"`
2. 检查 remote_sync_sites 表中的 http_host 配置
3. 确认目标服务器正在运行
4. 查看网络防火墙规则

### 10.2 日志分析

#### 10.2.1 启用详细日志

```bash
RUST_LOG=debug cargo run --features web_server
```

#### 10.2.2 关键日志点

```rust
// 文件监听事件
println!("Path: {:?}, Sesno Range: {:?}", path, &sesno_range);

// 任务入队
println!("[remote-sync-smoke] enqueued task: {}", task_id);

// 任务完成
println!("[remote-sync-smoke] SUCCESS");

// 错误信息
eprintln!("更新 remote_sync_logs 失败: {err}");
```

### 10.3 性能监控

#### 10.3.1 关键指标

```json
{
  "sync_rate_mbps": 12.5,          // 同步速率
  "avg_sync_time_ms": 320,         // 平均耗时
  "queue_size": 3,                 // 队列积压
  "total_failed": 5                // 失败率
}
```

#### 10.3.2 告警规则

```rust
// 队列积压
if center.state.queue_size > 100 {
    alert!("同步队列积压严重: {}", center.state.queue_size);
}

// 失败率过高
let failure_rate = total_failed / (total_synced + total_failed);
if failure_rate > 0.3 {
    alert!("同步失败率过高: {:.1}%", failure_rate * 100.0);
}

// MQTT 断连
if !center.state.mqtt_connected && center.state.mqtt_reconnect_count > 5 {
    alert!("MQTT 连接持续失败");
}
```

---

## 性能优化

### 11.1 并发优化

```rust
// 调整最大并发数
center.config.max_concurrent_syncs = 10;

// 批量处理
center.config.batch_size = 20;
```

### 11.2 文件压缩优化

```rust
// 使用更高压缩级别
let compress_options = CompressOptions {
    compression_level: 9,  // 1-9
    ...
};
```

### 11.3 数据库优化

```sql
-- 定期清理旧日志
DELETE FROM remote_sync_logs 
WHERE created_at < datetime('now', '-30 days');

-- 重建索引
REINDEX;

-- 优化数据库
VACUUM;
```

### 11.4 网络优化

```rust
// HTTP 客户端配置
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(60))
    .pool_max_idle_per_host(10)
    .tcp_keepalive(Some(Duration::from_secs(30)))
    .build()?;
```

---

## 开发规范

### 12.1 代码规范

#### 12.1.1 命名约定

- 模块名: `snake_case`
- 结构体: `PascalCase`
- 函数: `snake_case`
- 常量: `UPPER_SNAKE_CASE`

#### 12.1.2 错误处理

```rust
// ✅ 推荐: 使用 anyhow::Result
pub async fn process_sync_task(task: &SyncTask) -> anyhow::Result<()> {
    let metadata = fs::metadata(&task.file_path)
        .await
        .with_context(|| format!("无法访问文件: {}", task.file_path))?;
    Ok(())
}

// ❌ 避免: 使用 unwrap()
let metadata = fs::metadata(&task.file_path).await.unwrap();
```

#### 12.1.3 异步编程

```rust
// ✅ 推荐: 使用 tokio::spawn 避免阻塞
let result = tokio::spawn(async move {
    // 长时间运行的任务
}).await?;

// ✅ 推荐: 使用 spawn_blocking 处理同步代码
let result = tokio::task::spawn_blocking(|| {
    // 同步 SQLite 查询
}).await?;
```

### 12.2 测试规范

#### 12.2.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_add_task() {
        let mut center = SyncControlCenter::new();
        let task_id = center.add_task(NewSyncTaskParams { ... });
        assert!(!task_id.is_empty());
        assert_eq!(center.state.queue_size, 1);
    }
}
```

#### 12.2.2 集成测试

```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_full_workflow() {
    // 设置测试环境
    // 执行完整流程
    // 验证结果
}
```

### 12.3 文档规范

```rust
/// 处理单个同步任务
///
/// # 参数
///
/// * `task` - 要执行的同步任务
///
/// # 返回值
///
/// * `Ok(())` - 同步成功
/// * `Err(error)` - 同步失败，包含错误信息
///
/// # 错误
///
/// 当文件不存在、网络错误或目标不可达时返回错误
///
/// # 示例
///
/// ```no_run
/// let task = SyncTask { ... };
/// process_sync_task(&task).await?;
/// ```
pub async fn process_sync_task(task: &SyncTask) -> anyhow::Result<()> {
    // ...
}
```

### 12.4 Git 提交规范

```
feat: 添加新功能
fix: 修复 Bug
docs: 更新文档
refactor: 重构代码
test: 添加测试
chore: 构建/工具链变更
```

**示例**：
```
feat: 添加任务优先级队列支持

- 在 SyncTask 中添加 priority 字段
- 队列按优先级排序
- 更新 API 文档
```

---

## 附录

### A. 参考文档

- [Tokio 异步运行时](https://tokio.rs/)
- [Axum Web 框架](https://docs.rs/axum/)
- [notify 文件监听](https://docs.rs/notify/)
- [rumqttc MQTT 客户端](https://docs.rs/rumqttc/)
- [SurrealDB 文档](https://surrealdb.com/docs)

### B. 相关文件清单

```
src/
├── web_server/
│   ├── sync_control_center.rs       # 同步控制中心
│   ├── remote_runtime.rs            # 运行时管理
│   ├── remote_sync_handlers.rs      # HTTP 处理器
│   ├── sse_handlers.rs              # SSE 事件推送
│   └── site_metadata.rs             # 站点元数据管理
├── data_interface/
│   ├── increment_manager.rs         # 增量管理器
│   └── tidb_manager.rs              # 数据库管理器
└── bin/
    └── remote_sync_smoke_test.rs    # 烟雾测试

frontend/v0-aios-database-management/
├── app/remote-sync/                 # 前端页面
├── hooks/use-sync-control.ts        # 同步控制钩子
└── lib/api/remote-sync.ts           # API 客户端
```

### C. 版本历史

| 版本 | 日期 | 变更说明 |
|------|------|---------|
| 1.0.0 | 2025-01-15 | 初始版本 |
| 1.1.0 | TBD | 添加断点续传支持 |
| 2.0.0 | TBD | 支持双向同步 |

---

**文档版本**: 1.0.0  
**最后更新**: 2025-01-18  
**维护者**: AIOS 开发团队
