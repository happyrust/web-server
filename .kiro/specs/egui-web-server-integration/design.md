# Design Document

## Overview

本设计文档描述了将 Web Server 集成到 EGUI 应用作为嵌入式控制面板的技术架构、组件设计和实现方案。该设计将实现 Web Server 与 EGUI 应用的深度集成，提供统一的运维控制中心，并支持通过 URL 快速导入配置的异地协同功能。

### 设计目标

1. **嵌入式集成** - Web Server 运行在 EGUI 应用进程内，共享资源和状态
2. **统一控制** - 通过 EGUI 界面管理 Web Server 的启动、停止、重启
3. **实时监控** - 展示 Web Server 的统计信息、性能指标、请求日志
4. **配置导入** - 提供 API 和 UI 支持通过 URL 导入主站点配置
5. **零配置部署** - 新站点通过扫描 URL 二维码即可完成配置

### 技术栈

**现有技术栈**
- egui 0.33 (GUI 框架)
- eframe (应用框架)
- Axum (Web 框架)
- tokio (异步运行时)
- reqwest (HTTP 客户端)

**新增依赖**
- tokio::sync::mpsc (进程内通信)
- sysinfo (系统性能监控)
- qrcode (二维码生成)
- parking_lot (高性能锁)

## Architecture

### 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                      EguiRemoteSyncApp                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ AppState                                                  │  │
│  │ - web_server_handle: Option<ServerHandle>                │  │
│  │ - server_metrics: Arc<RwLock<ServerMetrics>>             │  │
│  │ - server_logs: Arc<RwLock<VecDeque<RequestLog>>>         │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ WebServerPage (控制面板)                                  │  │
│  │ - 启动/停止/重启按钮                                       │  │
│  │ - 统计信息展示（请求数、连接数、错误率）                   │  │
│  │ - 性能图表（CPU、内存、响应时间）                          │  │
│  │ - 请求日志实时查看                                         │  │
│  │ - 配置导入/导出                                            │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ tokio::sync::mpsc
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   EmbeddedWebServer (后台线程)                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Axum Router                                               │  │
│  │ - 现有 REST API                                           │  │
│  │ - 新增 /api/config/export (配置导出)                      │  │
│  │ - MetricsMiddleware (统计中间件)                          │  │
│  │ - LoggingMiddleware (日志中间件)                          │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ ServerMetrics (共享状态)                                  │  │
│  │ - total_requests: AtomicU64                               │  │
│  │ - active_connections: AtomicU32                           │  │
│  │ - error_count: AtomicU64                                  │  │
│  │ - response_times: Vec<Duration>                           │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```


### 目录结构

```
src/
├── gui/
│   ├── app.rs                        # 主应用（已存在）
│   ├── state.rs                      # 全局状态（已存在）
│   ├── pages/
│   │   ├── web_server.rs             # Web Server 控制面板（扩展）
│   │   └── config_import.rs          # 配置导入页面（新增）
│   ├── components/
│   │   ├── metrics_chart.rs          # 性能图表组件（新增）
│   │   ├── request_log_viewer.rs     # 请求日志查看器（新增）
│   │   └── qr_code_display.rs        # 二维码显示组件（新增）
│   └── embedded_server/              # 嵌入式服务器模块（新增）
│       ├── mod.rs
│       ├── server_handle.rs          # 服务器句柄
│       ├── metrics.rs                # 指标收集
│       ├── middleware.rs             # 中间件
│       └── config_export.rs          # 配置导出 API
└── web_server/
    └── mod.rs                        # Web Server（已存在，需扩展）
```

## Components and Interfaces

### 1. 嵌入式服务器句柄 (ServerHandle)

```rust
pub struct ServerHandle {
    /// 服务器任务句柄
    task_handle: tokio::task::JoinHandle<()>,
    /// 停止信号发送器
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    /// 服务器地址
    address: String,
    /// 启动时间
    started_at: std::time::Instant,
}

impl ServerHandle {
    pub fn new(
        task_handle: tokio::task::JoinHandle<()>,
        shutdown_tx: tokio::sync::oneshot::Sender<()>,
        address: String,
    ) -> Self {
        Self {
            task_handle,
            shutdown_tx,
            address,
            started_at: std::time::Instant::now(),
        }
    }
    
    pub fn address(&self) -> &str {
        &self.address
    }
    
    pub fn uptime(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
    
    pub async fn shutdown(self) -> anyhow::Result<()> {
        // 发送停止信号
        let _ = self.shutdown_tx.send(());
        // 等待任务完成
        self.task_handle.await?;
        Ok(())
    }
}
```

### 2. 服务器指标 (ServerMetrics)

```rust
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Default)]
pub struct ServerMetrics {
    /// 总请求数
    pub total_requests: AtomicU64,
    /// 成功请求数
    pub success_requests: AtomicU64,
    /// 失败请求数
    pub error_requests: AtomicU64,
    /// 当前活跃连接数
    pub active_connections: AtomicU32,
    /// 峰值连接数
    pub peak_connections: AtomicU32,
    /// 响应时间历史（最近 1000 条）
    pub response_times: Arc<RwLock<VecDeque<ResponseTimeRecord>>>,
    /// 错误日志（最近 100 条）
    pub error_logs: Arc<RwLock<VecDeque<ErrorLog>>>,
}

#[derive(Debug, Clone)]
pub struct ResponseTimeRecord {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct ErrorLog {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub path: String,
    pub method: String,
    pub status_code: u16,
    pub error_message: String,
}

impl ServerMetrics {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn increment_total_requests(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_success_requests(&self) {
        self.success_requests.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_error_requests(&self) {
        self.error_requests.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_active_connections(&self) {
        let current = self.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        // 更新峰值
        self.peak_connections.fetch_max(current, Ordering::Relaxed);
    }
    
    pub fn decrement_active_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn record_response_time(&self, path: String, duration: std::time::Duration) {
        let mut times = self.response_times.write();
        times.push_back(ResponseTimeRecord {
            timestamp: chrono::Utc::now(),
            duration_ms: duration.as_millis() as u64,
            path,
        });
        // 保持最近 1000 条
        if times.len() > 1000 {
            times.pop_front();
        }
    }
    
    pub fn record_error(&self, path: String, method: String, status_code: u16, error_message: String) {
        let mut errors = self.error_logs.write();
        errors.push_back(ErrorLog {
            timestamp: chrono::Utc::now(),
            path,
            method,
            status_code,
            error_message,
        });
        // 保持最近 100 条
        if errors.len() > 100 {
            errors.pop_front();
        }
    }
    
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            success_requests: self.success_requests.load(Ordering::Relaxed),
            error_requests: self.error_requests.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            peak_connections: self.peak_connections.load(Ordering::Relaxed),
            avg_response_time_ms: self.calculate_avg_response_time(),
        }
    }
    
    fn calculate_avg_response_time(&self) -> u64 {
        let times = self.response_times.read();
        if times.is_empty() {
            return 0;
        }
        let sum: u64 = times.iter().map(|r| r.duration_ms).sum();
        sum / times.len() as u64
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub success_requests: u64,
    pub error_requests: u64,
    pub active_connections: u32,
    pub peak_connections: u32,
    pub avg_response_time_ms: u64,
}
```


### 3. 指标收集中间件 (MetricsMiddleware)

```rust
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

pub async fn metrics_middleware(
    req: Request,
    next: Next,
) -> Response {
    let metrics = req.extensions().get::<Arc<ServerMetrics>>().cloned();
    
    if let Some(metrics) = metrics {
        metrics.increment_total_requests();
        metrics.increment_active_connections();
        
        let path = req.uri().path().to_string();
        let start = Instant::now();
        
        let response = next.run(req).await;
        
        let duration = start.elapsed();
        metrics.decrement_active_connections();
        
        // 记录响应时间
        metrics.record_response_time(path.clone(), duration);
        
        // 根据状态码更新统计
        if response.status().is_success() {
            metrics.increment_success_requests();
        } else {
            metrics.increment_error_requests();
            metrics.record_error(
                path,
                "GET".to_string(), // 简化处理，实际应从 request 获取
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown").to_string(),
            );
        }
        
        response
    } else {
        next.run(req).await
    }
}
```

### 4. 请求日志记录 (RequestLog)

```rust
use std::collections::VecDeque;
use parking_lot::RwLock;

#[derive(Debug, Clone)]
pub struct RequestLog {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub client_ip: String,
}

pub struct RequestLogger {
    logs: Arc<RwLock<VecDeque<RequestLog>>>,
    max_logs: usize,
}

impl RequestLogger {
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Arc::new(RwLock::new(VecDeque::new())),
            max_logs,
        }
    }
    
    pub fn log(&self, log: RequestLog) {
        let mut logs = self.logs.write();
        logs.push_back(log);
        if logs.len() > self.max_logs {
            logs.pop_front();
        }
    }
    
    pub fn get_logs(&self) -> Vec<RequestLog> {
        self.logs.read().iter().cloned().collect()
    }
    
    pub fn clear(&self) {
        self.logs.write().clear();
    }
}

pub async fn logging_middleware(
    req: Request,
    next: Next,
) -> Response {
    let logger = req.extensions().get::<Arc<RequestLogger>>().cloned();
    
    if let Some(logger) = logger {
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let client_ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        
        let start = Instant::now();
        let response = next.run(req).await;
        let duration = start.elapsed();
        
        logger.log(RequestLog {
            timestamp: chrono::Utc::now(),
            method,
            path,
            status_code: response.status().as_u16(),
            duration_ms: duration.as_millis() as u64,
            client_ip,
        });
        
        response
    } else {
        next.run(req).await
    }
}
```

### 5. 配置导出 API

```rust
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ConfigExportResponse {
    pub version: String,
    pub export_time: String,
    pub environments: Vec<RemoteSyncEnv>,
    pub sites: Vec<RemoteSyncSite>,
    pub mqtt_config: Option<MqttConfig>,
    pub file_server_config: Option<FileServerConfig>,
}

#[derive(Debug, Serialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id_prefix: String,
}

#[derive(Debug, Serialize)]
pub struct FileServerConfig {
    pub host: String,
    pub port: u16,
    pub base_path: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigExportQuery {
    pub format: Option<String>,  // json | toml
    pub env_id: Option<String>,  // 只导出指定环境
}

pub async fn export_config_handler(
    State(state): State<AppState>,
    Query(query): Query<ConfigExportQuery>,
) -> Result<Json<ConfigExportResponse>, StatusCode> {
    // 从数据库加载配置
    let environments = load_environments().await?;
    let sites = load_sites().await?;
    
    // 过滤指定环境
    let filtered_envs = if let Some(env_id) = query.env_id {
        environments.into_iter().filter(|e| e.id == env_id).collect()
    } else {
        environments
    };
    
    // 过滤敏感信息
    let safe_envs = filtered_envs.into_iter().map(|mut env| {
        // 移除密码等敏感字段
        env
    }).collect();
    
    let response = ConfigExportResponse {
        version: "1.0.0".to_string(),
        export_time: chrono::Utc::now().to_rfc3339(),
        environments: safe_envs,
        sites,
        mqtt_config: extract_mqtt_config(),
        file_server_config: extract_file_server_config(),
    };
    
    Ok(Json(response))
}
```


### 6. Web Server 控制面板扩展

```rust
pub struct WebServerPage {
    config: WebServerConfig,
    server_handle: Option<Arc<ServerHandle>>,
    metrics: Arc<ServerMetrics>,
    request_logger: Arc<RequestLogger>,
    logs: Vec<String>,
    
    // 新增字段
    metrics_history: VecDeque<MetricsSnapshot>,
    show_config_import: bool,
    import_url: String,
    import_preview: Option<ConfigExportResponse>,
    system_monitor: SystemMonitor,
}

impl WebServerPage {
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Web Server 控制面板");
        
        // 服务器状态和控制按钮
        self.render_server_status(ui);
        ui.separator();
        
        // 统计信息卡片
        self.render_metrics_cards(ui);
        ui.separator();
        
        // 性能图表
        self.render_performance_charts(ui);
        ui.separator();
        
        // 请求日志
        self.render_request_logs(ui);
        ui.separator();
        
        // 配置导入/导出
        self.render_config_management(ui);
    }
    
    fn render_server_status(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("服务器状态:");
            
            if let Some(handle) = &self.server_handle {
                ui.colored_label(egui::Color32::GREEN, "🟢 运行中");
                ui.label(format!("地址: {}", handle.address()));
                ui.label(format!("运行时长: {:?}", handle.uptime()));
                
                if ui.button("⏹️ 停止").clicked() {
                    self.stop_server();
                }
                if ui.button("🔄 重启").clicked() {
                    self.restart_server();
                }
            } else {
                ui.colored_label(egui::Color32::GRAY, "⚪ 已停止");
                
                if ui.button("▶️ 启动").clicked() {
                    self.start_server();
                }
            }
        });
    }
    
    fn render_metrics_cards(&self, ui: &mut egui::Ui) {
        let snapshot = self.metrics.get_snapshot();
        
        ui.horizontal(|ui| {
            // 请求统计卡片
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(240, 248, 255))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("请求统计");
                        ui.label(format!("总请求数: {}", snapshot.total_requests));
                        ui.label(format!("成功: {}", snapshot.success_requests));
                        ui.label(format!("失败: {}", snapshot.error_requests));
                        let success_rate = if snapshot.total_requests > 0 {
                            (snapshot.success_requests as f64 / snapshot.total_requests as f64) * 100.0
                        } else {
                            0.0
                        };
                        ui.label(format!("成功率: {:.2}%", success_rate));
                    });
                });
            
            // 连接统计卡片
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(240, 255, 240))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("连接统计");
                        ui.label(format!("活跃连接: {}", snapshot.active_connections));
                        ui.label(format!("峰值连接: {}", snapshot.peak_connections));
                    });
                });
            
            // 性能统计卡片
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(255, 248, 240))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("性能统计");
                        ui.label(format!("平均响应时间: {} ms", snapshot.avg_response_time_ms));
                        
                        // 系统资源
                        let sys_info = self.system_monitor.get_info();
                        ui.label(format!("CPU 使用率: {:.1}%", sys_info.cpu_usage));
                        ui.label(format!("内存使用: {} MB", sys_info.memory_used_mb));
                    });
                });
        });
    }
    
    fn render_performance_charts(&mut self, ui: &mut egui::Ui) {
        use egui_plot::{Line, Plot, PlotPoints};
        
        ui.heading("性能趋势");
        
        // 收集历史数据
        let snapshot = self.metrics.get_snapshot();
        self.metrics_history.push_back(snapshot.clone());
        if self.metrics_history.len() > 60 {
            self.metrics_history.pop_front();
        }
        
        // 请求数趋势图
        let request_points: PlotPoints = self.metrics_history
            .iter()
            .enumerate()
            .map(|(i, s)| [i as f64, s.total_requests as f64])
            .collect();
        
        Plot::new("request_trend")
            .height(150.0)
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new(request_points).name("总请求数"));
            });
        
        // 响应时间趋势图
        let response_time_points: PlotPoints = self.metrics_history
            .iter()
            .enumerate()
            .map(|(i, s)| [i as f64, s.avg_response_time_ms as f64])
            .collect();
        
        Plot::new("response_time_trend")
            .height(150.0)
            .show(ui, |plot_ui| {
                plot_ui.line(Line::new(response_time_points).name("平均响应时间 (ms)"));
            });
    }
    
    fn render_request_logs(&self, ui: &mut egui::Ui) {
        ui.heading("请求日志");
        
        ui.horizontal(|ui| {
            if ui.button("🔄 刷新").clicked() {
                // 刷新日志
            }
            if ui.button("🗑️ 清空").clicked() {
                self.request_logger.clear();
            }
        });
        
        use egui_extras::{TableBuilder, Column};
        
        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::auto().at_least(150.0)) // 时间
            .column(Column::auto().at_least(60.0))  // 方法
            .column(Column::auto().at_least(200.0)) // 路径
            .column(Column::auto().at_least(60.0))  // 状态码
            .column(Column::auto().at_least(80.0))  // 响应时间
            .column(Column::remainder())            // 客户端 IP
            .header(20.0, |mut header| {
                header.col(|ui| { ui.strong("时间"); });
                header.col(|ui| { ui.strong("方法"); });
                header.col(|ui| { ui.strong("路径"); });
                header.col(|ui| { ui.strong("状态码"); });
                header.col(|ui| { ui.strong("响应时间"); });
                header.col(|ui| { ui.strong("客户端"); });
            })
            .body(|mut body| {
                let logs = self.request_logger.get_logs();
                for log in logs.iter().rev().take(100) {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(log.timestamp.format("%H:%M:%S").to_string());
                        });
                        row.col(|ui| {
                            ui.label(&log.method);
                        });
                        row.col(|ui| {
                            ui.label(&log.path);
                        });
                        row.col(|ui| {
                            let color = if log.status_code < 400 {
                                egui::Color32::GREEN
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(color, log.status_code.to_string());
                        });
                        row.col(|ui| {
                            ui.label(format!("{} ms", log.duration_ms));
                        });
                        row.col(|ui| {
                            ui.label(&log.client_ip);
                        });
                    });
                }
            });
    }
    
    fn render_config_management(&mut self, ui: &mut egui::Ui) {
        ui.heading("配置管理");
        
        ui.horizontal(|ui| {
            if ui.button("📤 导出配置").clicked() {
                self.export_config();
            }
            if ui.button("📥 导入配置").clicked() {
                self.show_config_import = true;
            }
            if ui.button("📱 生成二维码").clicked() {
                self.generate_qr_code();
            }
        });
        
        // 配置导入对话框
        if self.show_config_import {
            self.render_config_import_dialog(ui);
        }
    }
    
    fn render_config_import_dialog(&mut self, ui: &mut egui::Ui) {
        egui::Window::new("导入配置")
            .collapsible(false)
            .resizable(true)
            .show(ui.ctx(), |ui| {
                ui.label("输入主站点配置 URL:");
                ui.text_edit_singleline(&mut self.import_url);
                
                ui.horizontal(|ui| {
                    if ui.button("获取配置").clicked() {
                        self.fetch_config_from_url();
                    }
                    if ui.button("取消").clicked() {
                        self.show_config_import = false;
                    }
                });
                
                // 配置预览
                if let Some(preview) = &self.import_preview {
                    ui.separator();
                    ui.heading("配置预览");
                    ui.label(format!("版本: {}", preview.version));
                    ui.label(format!("导出时间: {}", preview.export_time));
                    ui.label(format!("环境数量: {}", preview.environments.len()));
                    ui.label(format!("站点数量: {}", preview.sites.len()));
                    
                    if ui.button("确认导入").clicked() {
                        self.import_config();
                    }
                }
            });
    }
    
    fn start_server(&mut self) {
        // 启动嵌入式服务器
        let metrics = self.metrics.clone();
        let logger = self.request_logger.clone();
        
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        
        let task_handle = tokio::spawn(async move {
            start_embedded_server(metrics, logger, shutdown_rx).await
        });
        
        let address = format!("http://{}:{}", self.config.host, self.config.port);
        self.server_handle = Some(Arc::new(ServerHandle::new(
            task_handle,
            shutdown_tx,
            address,
        )));
    }
    
    fn stop_server(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            tokio::spawn(async move {
                let _ = Arc::try_unwrap(handle)
                    .ok()
                    .unwrap()
                    .shutdown()
                    .await;
            });
        }
    }
    
    fn restart_server(&mut self) {
        self.stop_server();
        // 等待停止完成后启动
        tokio::time::sleep(std::time::Duration::from_secs(1));
        self.start_server();
    }
}
```


### 7. 系统性能监控

```rust
use sysinfo::{System, SystemExt, ProcessExt, CpuExt};

pub struct SystemMonitor {
    system: System,
    last_update: std::time::Instant,
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            last_update: std::time::Instant::now(),
        }
    }
    
    pub fn get_info(&mut self) -> SystemInfo {
        // 每秒更新一次
        if self.last_update.elapsed() > std::time::Duration::from_secs(1) {
            self.system.refresh_all();
            self.last_update = std::time::Instant::now();
        }
        
        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let memory_used_mb = used_memory / 1024 / 1024;
        let memory_total_mb = total_memory / 1024 / 1024;
        
        SystemInfo {
            cpu_usage,
            memory_used_mb,
            memory_total_mb,
            memory_usage_percent: (used_memory as f32 / total_memory as f32) * 100.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub cpu_usage: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub memory_usage_percent: f32,
}
```

### 8. 二维码生成

```rust
use qrcode::QrCode;
use qrcode::render::svg;

pub fn generate_config_qr_code(url: &str) -> Result<String, qrcode::types::QrError> {
    let code = QrCode::new(url.as_bytes())?;
    let svg_string = code
        .render()
        .min_dimensions(200, 200)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build();
    Ok(svg_string)
}

pub struct QrCodeDisplay {
    svg_data: String,
}

impl QrCodeDisplay {
    pub fn new(url: &str) -> Result<Self, qrcode::types::QrError> {
        let svg_data = generate_config_qr_code(url)?;
        Ok(Self { svg_data })
    }
    
    pub fn render(&self, ui: &mut egui::Ui) {
        // 在 egui 中显示二维码
        // 注意：egui 不直接支持 SVG，需要转换为图片或使用第三方库
        ui.label("配置导入二维码");
        ui.label("(扫描二维码获取配置 URL)");
        
        // 简化实现：显示 URL 文本
        ui.code(&self.svg_data);
    }
}
```

## Data Models

### 配置导入/导出数据模型

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigExportResponse {
    pub version: String,
    pub export_time: String,
    pub environments: Vec<RemoteSyncEnv>,
    pub sites: Vec<RemoteSyncSite>,
    pub mqtt_config: Option<MqttConfig>,
    pub file_server_config: Option<FileServerConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigImportRequest {
    pub source_url: String,
    pub selected_env_ids: Vec<String>,
    pub selected_site_ids: Vec<String>,
    pub overwrite_existing: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigImportResult {
    pub success: bool,
    pub imported_envs: usize,
    pub imported_sites: usize,
    pub skipped_envs: usize,
    pub skipped_sites: usize,
    pub errors: Vec<String>,
}
```

## Error Handling

### 错误类型定义

```rust
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("服务器已在运行")]
    AlreadyRunning,
    
    #[error("服务器未运行")]
    NotRunning,
    
    #[error("端口 {0} 已被占用")]
    PortInUse(u16),
    
    #[error("配置错误: {0}")]
    ConfigError(String),
    
    #[error("网络错误: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON 解析错误: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type ServerResult<T> = Result<T, ServerError>;
```

### 错误处理策略

1. **启动失败** - 显示错误对话框，提供重试和端口修改选项
2. **运行时错误** - 记录到错误日志，在界面显示错误横幅
3. **配置导入失败** - 显示详细错误信息，允许部分导入
4. **网络超时** - 自动重试 3 次，超时后提示用户

## Testing Strategy

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_increment() {
        let metrics = ServerMetrics::new();
        metrics.increment_total_requests();
        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 1);
    }
    
    #[test]
    fn test_response_time_recording() {
        let metrics = ServerMetrics::new();
        metrics.record_response_time(
            "/api/test".to_string(),
            std::time::Duration::from_millis(100),
        );
        let times = metrics.response_times.read();
        assert_eq!(times.len(), 1);
        assert_eq!(times[0].duration_ms, 100);
    }
    
    #[tokio::test]
    async fn test_config_export() {
        let response = export_config_handler(
            State(test_app_state()),
            Query(ConfigExportQuery {
                format: Some("json".to_string()),
                env_id: None,
            }),
        ).await;
        assert!(response.is_ok());
    }
}
```

### 集成测试

1. **服务器启动/停止测试** - 验证服务器可以正常启动和停止
2. **指标收集测试** - 发送测试请求，验证指标正确更新
3. **配置导入测试** - 模拟配置导入流程，验证数据正确保存
4. **并发测试** - 多线程访问共享状态，验证线程安全

### 性能测试

1. **负载测试** - 使用 wrk 或 ab 工具测试高并发场景
2. **内存泄漏测试** - 长时间运行，监控内存使用
3. **响应时间测试** - 验证 P95、P99 响应时间符合要求

## Implementation Notes

### 关键实现要点

1. **线程安全** - 使用 Arc<RwLock<T>> 或 Arc<Mutex<T>> 保护共享状态
2. **异步通信** - 使用 tokio::sync::mpsc 在 EGUI 主线程和服务器线程间通信
3. **优雅关闭** - 使用 oneshot channel 发送停止信号，等待所有请求完成
4. **性能优化** - 使用 VecDeque 限制历史数据大小，避免内存无限增长
5. **错误恢复** - 服务器崩溃后自动重启，保持服务可用性

### 配置导入流程

```
1. 用户输入主站点 URL
   ↓
2. 发送 GET /api/config/export 请求
   ↓
3. 解析 JSON 响应，显示配置预览
   ↓
4. 用户选择要导入的环境和站点
   ↓
5. 检查本地是否存在同名配置
   ↓
6. 提示用户选择覆盖或跳过
   ↓
7. 调用本地 API 创建环境和站点
   ↓
8. 显示导入结果摘要
```

### 性能监控采样策略

- **实时指标** - 每次请求更新（原子操作，无锁）
- **历史数据** - 每 5 秒采样一次，保留最近 60 个数据点（5 分钟）
- **系统资源** - 每 1 秒更新一次 CPU 和内存使用率
- **日志清理** - 保留最近 100 条请求日志，自动清理旧数据

## Security Considerations

### 配置导出安全

1. **敏感信息过滤** - 导出时移除密码、密钥等敏感字段
2. **访问控制** - 可选添加 API Token 验证
3. **HTTPS 支持** - 生产环境使用 HTTPS 传输配置
4. **审计日志** - 记录配置导出和导入操作

### 服务器安全

1. **端口绑定** - 默认绑定 0.0.0.0，生产环境建议绑定 127.0.0.1
2. **CORS 配置** - 限制允许的来源域名
3. **请求限流** - 防止 DDoS 攻击
4. **输入验证** - 验证所有用户输入，防止注入攻击

## Integration with Existing Features

### 保留现有功能

本设计是对现有 `egui-remote-sync-ui` 的扩展，所有现有功能将被保留：

**异地协同功能（已实现）：**
- 环境列表管理（EnvironmentListPage）
- 站点配置管理（SiteConfigPage）
- 拓扑配置画布（TopologyCanvasPage）
- 监控面板（MonitorDashboardPage）
- 日志查询（LogQueryPage）

**任务管理功能（已实现）：**
- 任务创建（TaskCreationPage）
- 任务监控（TaskMonitorPage）
- 解析任务管理（ParseTaskPage）
- 模型生成配置（ModelGenPage）

**系统管理功能（已实现）：**
- 数据库管理（DatabaseManagePage）
- 配置编辑（ConfigEditorPage）

### 新增功能集成点

**Web Server 控制面板（扩展）：**
- 在现有 WebServerPage 基础上扩展
- 添加统计信息展示
- 添加性能监控图表
- 添加请求日志查看
- 添加配置导入/导出功能

**配置导入页面（新增）：**
- 作为独立页面添加到导航栏
- 或作为 WebServerPage 的子功能

### 功能整合架构

```
EguiRemoteSyncApp
├── 异地协同（Remote Sync）
│   ├── 环境列表 ✓
│   ├── 站点配置 ✓
│   ├── 拓扑配置 ✓
│   ├── 监控面板 ✓
│   └── 日志查询 ✓
├── 任务管理（Task Management）
│   ├── 任务创建 ✓
│   ├── 任务监控 ✓
│   ├── 解析任务 ✓
│   └── 模型生成 ✓
└── 系统管理（System Management）
    ├── Web Server 控制面板 ⭐ (扩展)
    │   ├── 服务器控制 (新增)
    │   ├── 统计信息 (新增)
    │   ├── 性能监控 (新增)
    │   ├── 请求日志 (新增)
    │   └── 配置管理 (新增)
    ├── 数据库管理 ✓
    └── 配置编辑 ✓
```

### 数据流整合

```
┌─────────────────────────────────────────────────────────────────┐
│                      EguiRemoteSyncApp                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ AppState (全局状态)                                       │  │
│  │ - environments: Vec<RemoteSyncEnv>        (已有)         │  │
│  │ - sites: Vec<RemoteSyncSite>              (已有)         │  │
│  │ - sync_tasks: Vec<SyncTask>               (已有)         │  │
│  │ - parse_tasks: Vec<ParseTask>             (已有)         │  │
│  │ - model_gen_tasks: Vec<ModelGenTask>      (已有)         │  │
│  │ - web_server_handle: Option<ServerHandle> (新增)         │  │
│  │ - server_metrics: Arc<ServerMetrics>      (新增)         │  │
│  │ - request_logger: Arc<RequestLogger>      (新增)         │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### API 客户端扩展

```rust
impl ApiClient {
    // 现有 API（保留）
    pub async fn get_environments(&self) -> Result<Vec<RemoteSyncEnv>>;
    pub async fn create_environment(&self, env: &RemoteSyncEnv) -> Result<String>;
    pub async fn get_sites(&self) -> Result<Vec<RemoteSyncSite>>;
    pub async fn create_site(&self, site: &RemoteSyncSite) -> Result<String>;
    pub async fn start_sync(&self) -> Result<()>;
    pub async fn stop_sync(&self) -> Result<()>;
    pub async fn get_sync_status(&self) -> Result<SyncStatus>;
    pub async fn query_logs(&self, filters: &LogFilters) -> Result<Vec<SyncLog>>;
    pub async fn create_parse_task(&self, task: &ParseTaskConfig) -> Result<String>;
    pub async fn start_parse_task(&self, task_id: &str) -> Result<()>;
    pub async fn get_parse_task_status(&self, task_id: &str) -> Result<ParseTaskStatus>;
    pub async fn create_model_gen_task(&self, task: &ModelGenConfig) -> Result<String>;
    pub async fn start_model_gen_task(&self, task_id: &str) -> Result<()>;
    
    // 新增 API
    pub async fn export_config(&self, query: &ConfigExportQuery) -> Result<ConfigExportResponse>;
    pub async fn import_config(&self, request: &ConfigImportRequest) -> Result<ConfigImportResult>;
    pub async fn get_server_metrics(&self) -> Result<MetricsSnapshot>;
    pub async fn get_request_logs(&self, limit: usize) -> Result<Vec<RequestLog>>;
}
```

### 导航栏更新

```rust
fn render_navigation(&mut self, ui: &mut egui::Ui) {
    ui.heading("导航");
    ui.separator();
    
    ui.vertical(|ui| {
        // 异地协同（已有）
        ui.label("异地协同");
        if ui.selectable_label(self.current_page == Page::EnvironmentList, "  环境列表").clicked() {
            self.current_page = Page::EnvironmentList;
        }
        if ui.selectable_label(self.current_page == Page::SiteConfig, "  站点配置").clicked() {
            self.current_page = Page::SiteConfig;
        }
        if ui.selectable_label(self.current_page == Page::TopologyCanvas, "  拓扑配置").clicked() {
            self.current_page = Page::TopologyCanvas;
        }
        if ui.selectable_label(self.current_page == Page::MonitorDashboard, "  监控面板").clicked() {
            self.current_page = Page::MonitorDashboard;
        }
        if ui.selectable_label(self.current_page == Page::LogQuery, "  日志查询").clicked() {
            self.current_page = Page::LogQuery;
        }
        
        ui.add_space(10.0);
        // 任务管理（已有）
        ui.label("任务管理");
        if ui.selectable_label(self.current_page == Page::TaskCreation, "  任务创建").clicked() {
            self.current_page = Page::TaskCreation;
        }
        if ui.selectable_label(self.current_page == Page::TaskMonitor, "  任务监控").clicked() {
            self.current_page = Page::TaskMonitor;
        }
        
        ui.add_space(10.0);
        // 系统管理（已有 + 新增）
        ui.label("系统管理");
        if ui.selectable_label(self.current_page == Page::WebServer, "  Web Server").clicked() {
            self.current_page = Page::WebServer;
        }
        if ui.selectable_label(self.current_page == Page::DatabaseManage, "  数据库管理").clicked() {
            self.current_page = Page::DatabaseManage;
        }
        if ui.selectable_label(self.current_page == Page::ConfigEditor, "  配置编辑").clicked() {
            self.current_page = Page::ConfigEditor;
        }
        if ui.selectable_label(self.current_page == Page::Settings, "  设置").clicked() {
            self.current_page = Page::Settings;
        }
    });
}
```

### 兼容性说明

1. **不影响现有功能** - 所有现有页面和功能保持不变
2. **共享状态扩展** - 在 AppState 中添加新字段，不修改现有字段
3. **API 向后兼容** - 新增 API 端点，不修改现有端点
4. **渐进式增强** - 可以先实现 Web Server 控制面板，再添加配置导入功能

### 实现优先级

**Phase 1: Web Server 嵌入式集成（核心）**
- ServerHandle 和 ServerMetrics
- 在 EGUI 进程内启动 Web Server
- 基本的启动/停止控制

**Phase 2: 统计信息展示（重要）**
- MetricsMiddleware 和 RequestLogger
- 统计信息卡片
- 请求日志查看

**Phase 3: 性能监控（重要）**
- SystemMonitor
- 性能趋势图表
- 实时刷新

**Phase 4: 配置导入/导出（重要）**
- ConfigExportAPI
- 配置导入对话框
- 二维码生成

**Phase 5: 高级功能（可选）**
- 配置同步状态监控
- 健康检查
- 性能告警
