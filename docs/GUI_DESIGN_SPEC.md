# 异地协同管理 GUI 设计规格

## 概述

基于 egui/eframe 的图形界面，用于管理和监控异地协同测试环境。

## 技术栈

- **UI 框架**: egui 0.33.0 / eframe
- **数据库**: SQLite (rusqlite)
- **进程管理**: tokio::process
- **文件系统**: std::fs / tokio::fs
- **HTTP 客户端**: reqwest
- **实时通信**: tokio::sync::watch / broadcast

## 界面架构

### 主窗口结构

```
┌─────────────────────────────────────────────────────────┐
│ 异地协同管理工具                            [_][□][X]   │
├─────────────────────────────────────────────────────────┤
│ 📊 仪表盘 | ⚙️ 环境配置 | 🌐 站点管理 | 🔄 同步控制   │
│ 📁 文件浏览 | 📝 日志 | 🧪 测试工具                     │
├─────────────────────────────────────────────────────────┤
│                                                           │
│                    [标签页内容区域]                      │
│                                                           │
│                                                           │
│                                                           │
├─────────────────────────────────────────────────────────┤
│ 状态栏: 服务运行中 | MQTT: 已连接 | 最后同步: 2分钟前  │
└─────────────────────────────────────────────────────────┘
```

## 1. 📊 仪表盘（Dashboard）

### 功能组件

#### 1.1 服务状态卡片（Service Status Cards）

显示所有服务的实时状态：

```rust
struct ServiceCard {
    name: String,          // "MQTT", "SurrealDB-1112", etc.
    status: ServiceStatus, // Running, Stopped, Error
    port: u16,
    uptime: Duration,
    cpu_usage: f32,
    memory_mb: u64,
    health: HealthStatus,  // Healthy, Degraded, Unhealthy
}
```

**交互**:
- 点击卡片 → 显示详细信息
- 右键菜单 → 重启服务、查看日志、停止服务

#### 1.2 同步统计面板（Sync Statistics Panel）

```rust
struct SyncStats {
    total_synced: u64,
    total_failed: u64,
    sync_rate_mbps: f64,
    avg_sync_time_ms: u64,
    last_sync_time: Option<DateTime<Utc>>,
    recent_syncs: Vec<SyncRecord>,
}
```

**显示内容**:
- 总同步次数（成功/失败）
- 平均同步速度
- 最近 10 次同步记录（表格）
- 同步速率图表（最近 1 小时）

#### 1.3 快速操作按钮

```
┌─────────────────────────────────────────┐
│ [▶ 启动所有服务] [⏸ 停止所有服务]      │
│ [🔄 触发同步]    [🧪 运行测试]          │
│ [📁 打开日志目录] [⚙️ 打开配置]        │
└─────────────────────────────────────────┘
```

#### 1.4 近期活动时间线

显示最近 20 条系统事件：
- 服务启动/停止
- 同步完成/失败
- 配置变更
- 错误告警

## 2. ⚙️ 环境配置（Environment Config）

### 2.1 环境基本信息

```rust
struct EnvironmentConfig {
    id: String,
    name: String,                // 环境名称
    location: String,            // 站点标识 (SITE1112, SITE7000)
    location_dbs: Vec<i32>,      // 数据库编号列表
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

**UI 布局**:
```
环境名称:     [____________________]
站点标识:     [____________________]
数据库编号:   [1112, 7000]  [+ 添加] [- 删除]
```

### 2.2 MQTT 配置

```rust
struct MqttConfig {
    host: String,                // 默认: 127.0.0.1
    port: u16,                   // 默认: 1883
    reconnect_initial_ms: Option<u64>,
    reconnect_max_ms: Option<u64>,
}
```

**UI 布局**:
```
MQTT 服务器:  [127.0.0.1____] : [1883]
重连初始延迟: [1000____] ms
重连最大延迟: [30000___] ms

[测试连接]
```

### 2.3 文件服务器配置

```rust
struct FileServerConfig {
    host: String,                // http://127.0.0.1:8081/assets/archives
    base_path: PathBuf,          // D:/work/plant/web-server/remote-test-dir/test-real/site-1112
}
```

**UI 布局**:
```
文件服务器 URL:  [http://127.0.0.1:8081/assets/archives___]
本地基础路径:    [D:\work\plant\web-server\remote-test-dir\..] [浏览...]

[测试连接]
```

### 2.4 AVEVA 项目配置

```
项目路径:        [D:\AVEVA\Projects\E3D2.1_______________] [浏览...]
包含的项目:      ☑ AvevaMarineSample
                 ☑ AvevaCatalogue
                 ☐ SCB
                 ☐ ZDJ

[验证路径]
```

### 2.5 操作按钮

```
[💾 保存配置] [🔄 重新加载] [📋 导出配置] [📥 导入配置]
```

## 3. 🌐 远程站点管理（Remote Sites）

### 3.1 站点列表视图

表格显示所有远程站点：

| 名称 | 位置 | HTTP 地址 | 数据库 | 状态 | 操作 |
|------|------|-----------|--------|------|------|
| Site-7000 | SITE7000 | http://127.0.0.1:8082/files | 7000 | 🟢 在线 | [编辑] [删除] [测试] |
| Site-1112 | SITE1112 | http://127.0.0.1:8081/files | 1112 | 🟢 在线 | [编辑] [删除] [测试] |

### 3.2 添加/编辑站点对话框

```rust
struct RemoteSite {
    id: String,
    env_id: String,
    name: String,              // 站点名称
    location: String,          // 位置标识
    http_host: String,         // HTTP 接收地址
    dbnums: Vec<i32>,          // 数据库编号列表
    notes: Option<String>,     // 备注
    enabled: bool,             // 是否启用
}
```

**对话框布局**:
```
┌─────────────────────────────────────┐
│ 添加远程站点                         │
├─────────────────────────────────────┤
│ 站点名称:  [Site-7000__________]    │
│ 位置标识:  [SITE7000___________]    │
│ HTTP 地址: [http://127.0.0.1:8082/files____] │
│ 数据库编号: [7000] [+ 添加]         │
│ 备注:      [________________________]│
│            [________________________]│
│ ☑ 启用此站点                         │
│                                      │
│ [测试连接] [取消] [保存]            │
└─────────────────────────────────────┘
```

### 3.3 站点连接测试

点击"测试连接"后:
1. 发送 HTTP GET 到 `{http_host}/health`
2. 显示响应时间
3. 显示连接状态（成功/失败）

## 4. 🔄 同步控制（Sync Control）

### 4.1 服务管理面板

```
┌─────────────────────────────────────────────────┐
│ 服务控制                                        │
├─────────────────────────────────────────────────┤
│ MQTT 服务器:      🟢 运行中  [停止] [重启]     │
│ SurrealDB-1112:   🟢 运行中  [停止] [重启]     │
│ SurrealDB-7000:   🟢 运行中  [停止] [重启]     │
│ WebServer-1112:   🟢 运行中  [停止] [重启]     │
│ WebServer-7000:   🟢 运行中  [停止] [重启]     │
│                                                 │
│ [▶ 启动全部] [⏸ 停止全部] [🔄 重启全部]       │
└─────────────────────────────────────────────────┘
```

### 4.2 手动同步触发

```
┌─────────────────────────────────────────────────┐
│ 手动同步                                        │
├─────────────────────────────────────────────────┤
│ 源站点:    [Site-1112 ▼]                       │
│ 目标站点:  [Site-7000 ▼]                       │
│ 文件路径:  [浏览...___________________________] │
│                                                 │
│ [🔄 立即同步]                                   │
└─────────────────────────────────────────────────┘
```

### 4.3 同步历史记录

表格显示同步历史：

| 时间 | 方向 | 文件 | 大小 | 记录数 | 状态 | 耗时 |
|------|------|------|------|--------|------|------|
| 14:25:32 | 1112→7000 | ams1112_0001.zip | 2.5MB | 1250 | ✓ 成功 | 1.2s |
| 14:24:15 | 7000→1112 | ams7000_0001.zip | 3.1MB | 1580 | ✗ 失败 | 5.0s |

**过滤选项**:
- 时间范围
- 站点
- 状态（成功/失败）

## 5. 📁 文件浏览器（File Browser）

### 5.1 目录树视图

```
📁 remote-test-dir/test-real/
├─ 📁 site-1112/
│  ├─ 📄 DbOption.toml
│  ├─ 📄 deployment_sites.sqlite
│  ├─ 📁 assets/
│  │  └─ 📁 archives/
│  │     ├─ 📦 ams1112_0001_20250118_142532.zip (2.5MB)
│  │     └─ 📦 ams1112_0001_20250118_141520.zip (2.3MB)
│  ├─ 📁 output/
│  │  └─ 📁 remote_sync/
│  │     └─ 📦 received_ams7000_0001_20250118.zip (3.1MB)
│  └─ 📁 surrealdb-data/
└─ 📁 site-7000/
   └─ ...
```

### 5.2 文件操作

**右键菜单**:
- 📂 在文件管理器中打开
- 📄 查看文件详情
- 🗑️ 删除文件
- 📋 复制路径
- 📤 导出

### 5.3 内置 Web 文件服务器

```
┌─────────────────────────────────────────────────┐
│ Web 文件浏览器                                  │
├─────────────────────────────────────────────────┤
│ 服务器状态: 🟢 运行中                           │
│ 端口: 8090                                      │
│ 访问地址: http://localhost:8090                │
│                                                 │
│ [打开浏览器] [停止服务]                        │
│                                                 │
│ 共享目录:                                       │
│ ☑ remote-test-dir/test-real/site-1112/         │
│ ☑ remote-test-dir/test-real/site-7000/         │
│ ☐ logs/                                         │
└─────────────────────────────────────────────────┘
```

## 6. 📝 日志查看器（Log Viewer）

### 6.1 日志源选择

```
日志源: [所有 ▼] [MQTT] [SurrealDB-1112] [SurrealDB-7000] [Web-1112] [Web-7000]
```

### 6.2 日志级别过滤

```
☑ ERROR  ☑ WARN  ☑ INFO  ☐ DEBUG  ☐ TRACE
```

### 6.3 日志内容显示

```
┌─────────────────────────────────────────────────┐
│ 14:25:32 [INFO]  Web-1112: Server started      │
│ 14:25:35 [ERROR] MQTT: Connection lost         │
│ 14:25:36 [WARN]  SurrealDB: Slow query (2.5s) │
│ ...                                             │
└─────────────────────────────────────────────────┘
```

**功能**:
- 🔍 搜索框（支持正则）
- ⏸️ 暂停/继续自动滚动
- 🗑️ 清空日志
- 💾 导出日志

### 6.4 日志统计

```
总条数: 15,234
错误: 12 | 警告: 45 | 信息: 15,177
```

## 7. 🧪 测试工具（Test Tools）

### 7.1 文件同步模拟器

```
┌─────────────────────────────────────────────────┐
│ 文件同步模拟器                                  │
├─────────────────────────────────────────────────┤
│ 同步间隔: [30] 秒                               │
│ 每次文件数: [1]                                 │
│ 同步方向: ⦿ 随机  ○ 1112→7000  ○ 7000→1112    │
│ 测试时长: [300] 秒 (0 = 无限)                  │
│                                                 │
│ 状态: ⏸️ 停止中                                 │
│ 已运行: 0秒 | 已同步: 0 次 | 文件: 0 个        │
│                                                 │
│ [▶ 开始模拟] [⏸ 暂停] [⏹ 停止]                │
└─────────────────────────────────────────────────┘
```

### 7.2 快速测试场景

预设的测试场景：

```
[📋 场景 1: 快速测试 (1分钟)]
[📋 场景 2: 标准测试 (5分钟)]
[📋 场景 3: 压力测试 (30分钟, 高频)]
[📋 场景 4: 双向同步测试]
```

### 7.3 自动化测试报告

```
最近测试:
┌─────────────────────────────────────────────────┐
│ 测试 #20250118-142530 (5分钟)                  │
│ 开始: 14:25:30 | 结束: 14:30:30                │
│ 同步操作: 10 | 成功: 10 | 失败: 0              │
│ 文件总数: 25 | 总大小: 62.5MB                  │
│                                                 │
│ [查看报告] [导出 HTML]                         │
└─────────────────────────────────────────────────┘
```

## 数据结构设计

### 应用状态

```rust
struct AppState {
    // 配置
    environment: EnvironmentConfig,
    mqtt_config: MqttConfig,
    file_server_config: FileServerConfig,
    remote_sites: Vec<RemoteSite>,

    // 运行时状态
    services: HashMap<String, ServiceStatus>,
    sync_stats: SyncStats,

    // 数据库连接
    db_connection: Arc<Mutex<Connection>>,

    // 后台任务
    service_handles: HashMap<String, JoinHandle<()>>,

    // 通道
    log_rx: broadcast::Receiver<LogEntry>,
    event_rx: broadcast::Receiver<SystemEvent>,

    // UI 状态
    selected_tab: Tab,
    show_add_site_dialog: bool,
    file_browser_path: PathBuf,
    log_filter: LogFilter,
}
```

### 事件系统

```rust
enum SystemEvent {
    ServiceStarted(String),
    ServiceStopped(String),
    ServiceError { service: String, error: String },
    SyncCompleted { source: String, target: String, duration: Duration },
    SyncFailed { source: String, target: String, error: String },
    ConfigUpdated,
}
```

## 实现技术要点

### 1. 进程管理

使用 `tokio::process::Command` 启动服务：

```rust
async fn start_service(service: &str) -> Result<Child> {
    let script = match service {
        "mqtt" => "scripts/test-real/start-mqtt-server.ps1",
        "surreal-1112" => "scripts/test-real/start-surreal-1112.ps1",
        // ...
    };

    Command::new("powershell")
        .arg("-ExecutionPolicy").arg("Bypass")
        .arg("-File").arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}
```

### 2. 实时日志流

使用 tokio channel 传递日志：

```rust
async fn stream_logs(service: String, tx: broadcast::Sender<LogEntry>) {
    let log_file = format!("logs/{}.log", service);
    let mut reader = BufReader::new(File::open(log_file)?);

    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;

        tx.send(LogEntry {
            timestamp: Utc::now(),
            level: parse_level(&line),
            service: service.clone(),
            message: line,
        })?;
    }
}
```

### 3. 内置 Web 文件服务器

使用 `axum` 或 `warp` 创建简单的文件服务器：

```rust
async fn start_file_server(port: u16, paths: Vec<PathBuf>) {
    let app = Router::new()
        .route("/", get(serve_index))
        .nest_service("/files", ServeDir::new(paths[0].clone()));

    axum::Server::bind(&([127, 0, 0, 1], port).into())
        .serve(app.into_make_service())
        .await?;
}
```

### 4. SQLite 数据库操作

```rust
impl AppState {
    fn load_environment(&self) -> Result<EnvironmentConfig> {
        let conn = self.db_connection.lock()?;
        let mut stmt = conn.prepare(
            "SELECT * FROM remote_sync_envs LIMIT 1"
        )?;
        // ... 反序列化
    }

    fn save_remote_site(&self, site: &RemoteSite) -> Result<()> {
        let conn = self.db_connection.lock()?;
        conn.execute(
            "INSERT OR REPLACE INTO remote_sync_sites (...) VALUES (...)",
            params![...],
        )?;
        Ok(())
    }
}
```

## 用户交互流程

### 启动流程

1. 用户启动 GUI 应用
2. 加载配置（从 SQLite）
3. 检测服务状态
4. 显示仪表盘

### 配置异地更新流程

1. 切换到"环境配置"标签
2. 填写环境信息（名称、位置、数据库编号）
3. 配置 MQTT 服务器
4. 配置文件服务器
5. 点击"保存配置" → 写入 SQLite
6. 切换到"站点管理"标签
7. 点击"添加站点"
8. 填写远程站点信息
9. 点击"测试连接" → 验证连通性
10. 点击"保存" → 写入数据库

### 启动服务并测试流程

1. 切换到"同步控制"标签
2. 点击"启动全部" → 后台启动所有服务
3. 观察服务状态变为"运行中"
4. 切换到"测试工具"标签
5. 配置同步模拟参数
6. 点击"开始模拟"
7. 实时查看仪表盘统计
8. 切换到"日志"标签查看详细日志
9. 测试完成后，点击"停止全部"

## 后续扩展建议

1. **性能图表**：使用 `egui_plot` 显示实时性能曲线
2. **告警系统**：错误自动弹窗提醒
3. **配置模板**：预设常用配置模板
4. **批量操作**：批量添加站点、批量测试
5. **远程管理**：通过 HTTP API 管理远程节点
6. **主题切换**：支持亮色/暗色主题
7. **国际化**：支持中英文切换
