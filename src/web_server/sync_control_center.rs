use crate::web_server::site_metadata::sanitize_path_segment;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::TryFrom,
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{
    fs,
    sync::{RwLock, broadcast},
    task::{JoinHandle, spawn_blocking},
};
use uuid::Uuid;

use anyhow::{Context, anyhow};
use chrono::{DateTime, Utc};

use crate::web_server::{
    remote_sync_handlers,
    site_metadata::{self, SiteMetadataEntry, SiteMetadataFile},
};

// ========= 全局状态管理 =========

/// 同步事件广播通道
pub static SYNC_EVENT_TX: Lazy<broadcast::Sender<SyncEvent>> = Lazy::new(|| {
    let (tx, _) = broadcast::channel(1000);
    tx
});

/// 同步控制中心全局实例
pub static SYNC_CONTROL_CENTER: Lazy<Arc<RwLock<SyncControlCenter>>> =
    Lazy::new(|| Arc::new(RwLock::new(SyncControlCenter::new())));

/// MQTT 服务器进程句柄（用于启动/停止控制）
pub static MQTT_SERVER_PROCESS: Lazy<Arc<RwLock<Option<tokio::process::Child>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// MQTT Broker 日志缓冲区（最多保存最近 500 条日志）
pub static MQTT_BROKER_LOGS: Lazy<Arc<RwLock<Vec<MqttBrokerLog>>>> =
    Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

const MAX_LOG_ENTRIES: usize = 500;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MqttBrokerLog {
    pub time: String,
    pub level: String,
    pub message: String,
}

// ========= 数据结构定义 =========

// Re-export SyncEvent from sse_handlers to avoid duplication
pub use crate::web_server::sse_handlers::SyncEvent;

/// 告警级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// 同步控制状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncControlState {
    /// 服务运行状态
    pub is_running: bool,
    /// 是否暂停
    pub is_paused: bool,
    /// 当前环境ID
    pub current_env: Option<String>,
    /// 环境名称
    pub env_name: Option<String>,

    /// 连接状态
    pub mqtt_connected: bool,
    pub watcher_active: bool,
    pub last_mqtt_connect_time: Option<SystemTime>,
    pub mqtt_reconnect_count: u32,

    /// 同步统计
    pub total_synced: u64,
    pub total_failed: u64,
    pub pending_count: u32,
    pub queue_size: u32,

    /// 性能指标
    pub sync_rate_mbps: f64,
    pub avg_sync_time_ms: u64,
    pub last_sync_time: Option<SystemTime>,

    /// 服务启动时间
    pub started_at: Option<SystemTime>,
    /// 累计运行时长（秒）
    pub uptime_seconds: u64,
}

impl Default for SyncControlState {
    fn default() -> Self {
        Self {
            is_running: false,
            is_paused: false,
            current_env: None,
            env_name: None,
            mqtt_connected: false,
            watcher_active: false,
            last_mqtt_connect_time: None,
            mqtt_reconnect_count: 0,
            total_synced: 0,
            total_failed: 0,
            pending_count: 0,
            queue_size: 0,
            sync_rate_mbps: 0.0,
            avg_sync_time_ms: 0,
            last_sync_time: None,
            started_at: None,
            uptime_seconds: 0,
        }
    }
}

/// 同步任务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTask {
    pub id: String,
    pub file_path: String,
    pub file_size: u64,
    pub file_name: Option<String>,
    pub file_hash: Option<String>,
    pub record_count: Option<u64>,
    pub env_id: Option<String>,
    pub source_env: Option<String>,
    pub target_site: Option<String>,
    pub direction: Option<String>,
    pub notes: Option<String>,
    pub status: SyncTaskStatus,
    pub priority: u8,
    pub retry_count: u32,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub error_message: Option<String>,
}

/// 新任务入队参数
#[derive(Debug, Clone)]
pub struct NewSyncTaskParams {
    pub file_path: String,
    pub file_size: u64,
    pub priority: u8,
    pub file_name: Option<String>,
    pub file_hash: Option<String>,
    pub record_count: Option<u64>,
    pub env_id: Option<String>,
    pub source_env: Option<String>,
    pub target_site: Option<String>,
    pub direction: Option<String>,
    pub notes: Option<String>,
}

/// 同步任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 同步配置
/// 注意：env_id 已移除，统一使用 DbOption.location 作为站点标识
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    pub auto_retry: bool,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub max_concurrent_syncs: u32,
    pub batch_size: u32,
    pub sync_interval_ms: u64,
    pub auto_pause_on_error: bool,
    pub alert_on_failure: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
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

/// 获取当前站点标识（使用 DbOption.location）
pub fn get_location() -> String {
    aios_core::get_db_option().location.clone()
}

// ========= 同步控制中心 =========

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: String,
    pub message: String,
}

pub struct SyncControlCenter {
    /// 当前状态
    pub state: SyncControlState,
    /// 同步配置
    pub config: SyncConfig,
    /// 任务队列
    pub task_queue: Vec<SyncTask>,
    /// 运行中的任务
    pub running_tasks: HashMap<String, SyncTask>,
    /// 历史记录（最近100条）
    pub history: Vec<SyncTask>,
    /// MQTT服务器状态
    pub mqtt_server: Option<MqttServerState>,
    /// 后台处理任务
    pub worker_handle: Option<JoinHandle<()>>,
    /// 日志缓冲区（最近500条）
    pub logs: Vec<LogEntry>,
    /// 进度广播中心（用于 WebSocket 实时推送）
    pub progress_hub: Option<Arc<crate::shared::ProgressHub>>,
}

/// MQTT服务器状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttServerState {
    pub is_running: bool,
    pub port: u16,
    pub client_count: u32,
    pub message_count: u64,
    pub started_at: Option<SystemTime>,
}

impl SyncControlCenter {
    pub fn new() -> Self {
        Self {
            state: SyncControlState::default(),
            config: SyncConfig::default(),
            task_queue: Vec::new(),
            running_tasks: HashMap::new(),
            history: Vec::new(),
            mqtt_server: None,
            worker_handle: None,
            logs: Vec::new(),
            progress_hub: None, // 初始化时为 None，由 web_server 启动时设置
        }
    }

    /// 添加日志条目
    pub fn add_log(&mut self, level: &str, message: String) {
        let entry = LogEntry {
            timestamp: SystemTime::now(),
            level: level.to_string(),
            message,
        };

        self.logs.push(entry);

        // 只保留最近500条日志
        if self.logs.len() > 500 {
            self.logs.drain(0..self.logs.len() - 500);
        }
    }

    /// 获取最近的日志
    pub fn get_logs(&self, limit: usize) -> Vec<LogEntry> {
        let start = if self.logs.len() > limit {
            self.logs.len() - limit
        } else {
            0
        };
        self.logs[start..].to_vec()
    }

    /// 启动同步服务
    /// 使用 DbOption.location 作为站点标识
    pub async fn start(&mut self) -> anyhow::Result<()> {
        if self.state.is_running {
            return Err(anyhow::anyhow!("同步服务已在运行"));
        }

        let location = get_location();
        
        // 停止现有运行时
        crate::web_server::remote_runtime::stop_runtime().await;

        // 启动新运行时
        crate::web_server::remote_runtime::start_runtime(location.clone()).await?;

        // 更新状态
        self.state.is_running = true;
        self.state.current_env = Some(location.clone());
        self.state.started_at = Some(SystemTime::now());
        self.spawn_worker();

        // 发送启动事件
        let _ = SYNC_EVENT_TX.send(SyncEvent::Started {
            env_id: location,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        Ok(())
    }

    /// 停止同步服务
    pub async fn stop(&mut self) -> anyhow::Result<()> {
        if !self.state.is_running {
            return Ok(());
        }

        // 停止运行时
        crate::web_server::remote_runtime::stop_runtime().await;

        // 更新状态
        self.state.is_running = false;
        self.state.is_paused = false;
        self.state.mqtt_connected = false;
        self.state.watcher_active = false;
        if let Some(handle) = self.worker_handle.take() {
            handle.abort();
        }

        // 发送停止事件
        let _ = SYNC_EVENT_TX.send(SyncEvent::Stopped {
            env_id: self.state.current_env.clone().unwrap_or_default(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        Ok(())
    }

    /// 暂停同步
    pub fn pause(&mut self) -> anyhow::Result<()> {
        if !self.state.is_running {
            return Err(anyhow::anyhow!("同步服务未运行"));
        }

        self.state.is_paused = true;
        Ok(())
    }

    /// 恢复同步
    pub fn resume(&mut self) -> anyhow::Result<()> {
        if !self.state.is_running {
            return Err(anyhow::anyhow!("同步服务未运行"));
        }

        self.state.is_paused = false;
        Ok(())
    }

    /// 添加同步任务
    pub fn add_task(&mut self, params: NewSyncTaskParams) -> String {
        let NewSyncTaskParams {
            file_path,
            file_size,
            priority,
            file_name,
            file_hash,
            record_count,
            env_id,
            source_env,
            target_site,
            direction,
            notes,
        } = params;

        // 使用传入的 env_id，如果没有则使用当前站点的 location
        let effective_env = env_id.or_else(|| {
            let location = get_location();
            if location.is_empty() {
                None
            } else {
                Some(location)
            }
        });

        let task = SyncTask {
            id: Uuid::new_v4().to_string(),
            file_path,
            file_size,
            file_name,
            file_hash,
            record_count,
            env_id: effective_env,
            source_env,
            target_site,
            direction,
            notes,
            status: SyncTaskStatus::Pending,
            priority,
            retry_count: 0,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        Self::persist_task_created(&task);

        let task_id = task.id.clone();
        self.task_queue.push(task);

        // 按优先级排序
        self.task_queue.sort_by(|a, b| b.priority.cmp(&a.priority));

        // 更新队列大小
        self.state.queue_size = self.task_queue.len() as u32;
        self.state.pending_count = self
            .task_queue
            .iter()
            .filter(|t| t.status == SyncTaskStatus::Pending)
            .count() as u32;

        task_id
    }

    fn spawn_worker(&mut self) {
        if self.worker_handle.is_some() {
            return;
        }
        let center_arc = SYNC_CONTROL_CENTER.clone();
        let handle = tokio::spawn(async move {
            loop {
                let (maybe_task, running) = {
                    let mut center = center_arc.write().await;
                    let running = center.state.is_running;
                    let task = if running {
                        center.get_next_task()
                    } else {
                        None
                    };
                    (task, running)
                };

                if !running {
                    break;
                }

                match maybe_task {
                    Some(task) => {
                        let result = process_sync_task(&task).await;
                        let mut center = center_arc.write().await;
                        match result {
                            Ok(_) => center.complete_task(&task.id, true, None),
                            Err(err) => {
                                center.complete_task(&task.id, false, Some(err.to_string()))
                            }
                        }
                    }
                    None => {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        });
        self.worker_handle = Some(handle);
    }

    fn persist_task_created(task: &SyncTask) {
        match remote_sync_handlers::open_sqlite() {
            Ok(mut conn) => {
                let created_at = Self::system_time_to_rfc3339(task.created_at);
                let now = Utc::now().to_rfc3339();
                let file_size = i64::try_from(task.file_size).unwrap_or(i64::MAX);
                let record_count = task.record_count.and_then(|v| i64::try_from(v).ok());
                if let Err(err) = conn.execute(
                    "INSERT OR REPLACE INTO remote_sync_logs (
                        id, task_id, env_id, source_env, target_site, site_id, direction,
                        file_path, file_size, record_count, status, error_message, notes,
                        started_at, completed_at, created_at, updated_at
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                        ?8, ?9, ?10, ?11, NULL, ?12,
                        NULL, NULL, ?13, ?14
                    )",
                    rusqlite::params![
                        &task.id,
                        &task.id,
                        task.env_id.as_deref(),
                        task.source_env.as_deref(),
                        task.target_site.as_deref(),
                        task.target_site.as_deref(),
                        task.direction.as_deref(),
                        &task.file_path,
                        file_size,
                        record_count,
                        Self::status_label(&SyncTaskStatus::Pending),
                        task.notes.as_deref(),
                        created_at,
                        now,
                    ],
                ) {
                    eprintln!("写入 remote_sync_logs 失败: {err}");
                }
            }
            Err(err) => {
                eprintln!("打开 remote_sync_logs 数据库失败: {err}");
            }
        }
    }

    fn persist_task_mark_running(task: &SyncTask) {
        match remote_sync_handlers::open_sqlite() {
            Ok(mut conn) => {
                let started_at = Self::option_system_time_to_rfc3339(task.started_at);
                let now = Utc::now().to_rfc3339();
                if let Err(err) = conn.execute(
                    "UPDATE remote_sync_logs
                     SET status = ?2,
                         started_at = COALESCE(?3, started_at),
                         updated_at = ?4
                     WHERE id = ?1",
                    rusqlite::params![
                        &task.id,
                        Self::status_label(&SyncTaskStatus::Running),
                        started_at,
                        now,
                    ],
                ) {
                    eprintln!("更新 remote_sync_logs 运行状态失败: {err}");
                }
            }
            Err(err) => {
                eprintln!("打开 remote_sync_logs 数据库失败: {err}");
            }
        }
    }

    fn persist_task_mark_finished(task: &SyncTask) {
        match remote_sync_handlers::open_sqlite() {
            Ok(mut conn) => {
                let started_at = Self::option_system_time_to_rfc3339(task.started_at);
                let completed_at = Self::option_system_time_to_rfc3339(task.completed_at);
                let now = Utc::now().to_rfc3339();
                if let Err(err) = conn.execute(
                    "UPDATE remote_sync_logs
                     SET status = ?2,
                         started_at = COALESCE(?3, started_at),
                         completed_at = ?4,
                         error_message = ?5,
                         updated_at = ?6
                     WHERE id = ?1",
                    rusqlite::params![
                        &task.id,
                        Self::status_label(&task.status),
                        started_at,
                        completed_at,
                        task.error_message.as_deref(),
                        now,
                    ],
                ) {
                    eprintln!("更新 remote_sync_logs 完成状态失败: {err}");
                }
            }
            Err(err) => {
                eprintln!("打开 remote_sync_logs 数据库失败: {err}");
            }
        }
    }

    fn persist_task_mark_pending(task: &SyncTask) {
        match remote_sync_handlers::open_sqlite() {
            Ok(mut conn) => {
                let now = Utc::now().to_rfc3339();
                if let Err(err) = conn.execute(
                    "UPDATE remote_sync_logs
                     SET status = ?2,
                         started_at = NULL,
                         completed_at = NULL,
                         error_message = NULL,
                         updated_at = ?3
                     WHERE id = ?1",
                    rusqlite::params![&task.id, Self::status_label(&SyncTaskStatus::Pending), now,],
                ) {
                    eprintln!("更新 remote_sync_logs 待处理状态失败: {err}");
                }
            }
            Err(err) => {
                eprintln!("打开 remote_sync_logs 数据库失败: {err}");
            }
        }
    }

    fn status_label(status: &SyncTaskStatus) -> &'static str {
        match status {
            SyncTaskStatus::Pending => "pending",
            SyncTaskStatus::Running => "running",
            SyncTaskStatus::Completed => "completed",
            SyncTaskStatus::Failed => "failed",
            SyncTaskStatus::Cancelled => "cancelled",
        }
    }

    fn system_time_to_rfc3339(time: SystemTime) -> String {
        let datetime: DateTime<Utc> = time.into();
        datetime.to_rfc3339()
    }

    fn option_system_time_to_rfc3339(time: Option<SystemTime>) -> Option<String> {
        time.map(Self::system_time_to_rfc3339)
    }

    /// 获取下一个待处理任务
    pub fn get_next_task(&mut self) -> Option<SyncTask> {
        if self.state.is_paused {
            return None;
        }

        // 检查并发限制
        if self.running_tasks.len() >= self.config.max_concurrent_syncs as usize {
            return None;
        }

        // 获取第一个待处理任务
        let index = self
            .task_queue
            .iter()
            .position(|t| t.status == SyncTaskStatus::Pending)?;

        let mut task = self.task_queue.remove(index);
        task.status = SyncTaskStatus::Running;
        task.started_at = Some(SystemTime::now());

        self.running_tasks.insert(task.id.clone(), task.clone());
        self.state.queue_size = self.task_queue.len() as u32;

        Self::persist_task_mark_running(&task);

        Some(task)
    }

    /// 完成任务
    pub fn complete_task(&mut self, task_id: &str, success: bool, error: Option<String>) {
        if let Some(mut task) = self.running_tasks.remove(task_id) {
            task.completed_at = Some(SystemTime::now());

            if success {
                task.status = SyncTaskStatus::Completed;
                self.state.total_synced += 1;

                // 发送完成事件
                if let Some(started_at) = task.started_at {
                    let duration_ms = SystemTime::now()
                        .duration_since(started_at)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let _ = SYNC_EVENT_TX.send(SyncEvent::SyncCompleted {
                        task_id: task.id.clone(),
                        file_path: task.file_path.clone(),
                        duration_ms,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                }

                Self::persist_task_mark_finished(&task);
            } else {
                if matches!(error.as_deref(), Some(msg) if msg == "用户取消") {
                    task.status = SyncTaskStatus::Cancelled;
                } else {
                    task.status = SyncTaskStatus::Failed;
                }
                task.error_message = error.clone();
                if task.status == SyncTaskStatus::Failed {
                    self.state.total_failed += 1;
                }

                // 发送失败事件
                let _ = SYNC_EVENT_TX.send(SyncEvent::SyncFailed {
                    task_id: task.id.clone(),
                    file_path: task.file_path.clone(),
                    error: error.unwrap_or_else(|| "未知错误".to_string()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });

                // 重试逻辑
                Self::persist_task_mark_finished(&task);

                if self.config.auto_retry
                    && task.retry_count < self.config.max_retries
                    && task.status == SyncTaskStatus::Failed
                {
                    task.retry_count += 1;
                    task.status = SyncTaskStatus::Pending;
                    task.started_at = None;
                    task.completed_at = None;
                    self.task_queue.push(task.clone());

                    Self::persist_task_mark_pending(&task);
                }
            }

            // 添加到历史记录
            self.history.push(task);
            if self.history.len() > 100 {
                self.history.remove(0);
            }

            // 更新统计
            self.update_statistics();
        }
    }

    /// 更新统计信息
    pub fn update_statistics(&mut self) {
        self.state.pending_count = self
            .task_queue
            .iter()
            .filter(|t| t.status == SyncTaskStatus::Pending)
            .count() as u32;

        // 计算平均同步时间
        let completed_tasks: Vec<_> = self
            .history
            .iter()
            .filter(|t| t.status == SyncTaskStatus::Completed)
            .filter_map(|t| match (t.started_at, t.completed_at) {
                (Some(start), Some(end)) => {
                    end.duration_since(start).ok().map(|d| d.as_millis() as u64)
                }
                _ => None,
            })
            .collect();

        if !completed_tasks.is_empty() {
            self.state.avg_sync_time_ms =
                completed_tasks.iter().sum::<u64>() / completed_tasks.len() as u64;
        }

        // 计算运行时长
        if let Some(started_at) = self.state.started_at {
            self.state.uptime_seconds = SystemTime::now()
                .duration_since(started_at)
                .unwrap_or_default()
                .as_secs();
        }
    }

    /// 从待处理队列取消指定任务
    pub fn cancel_pending_task(&mut self, task_id: &str, reason: &str) -> bool {
        if let Some(idx) = self.task_queue.iter().position(|t| t.id == task_id) {
            let mut task = self.task_queue.remove(idx);
            task.status = SyncTaskStatus::Cancelled;
            task.error_message = Some(reason.to_string());
            task.completed_at = Some(SystemTime::now());
            Self::persist_task_mark_finished(&task);
            self.history.push(task);
            if self.history.len() > 100 {
                self.history.remove(0);
            }
            self.state.queue_size = self.task_queue.len() as u32;
            self.update_statistics();
            return true;
        }
        false
    }

    /// 清空队列
    pub fn clear_queue(&mut self, reason: &str) -> usize {
        let mut removed = 0usize;
        let drained: Vec<_> = self.task_queue.drain(..).collect();
        for mut task in drained {
            task.status = SyncTaskStatus::Cancelled;
            task.error_message = Some(reason.to_string());
            task.completed_at = Some(SystemTime::now());
            Self::persist_task_mark_finished(&task);
            self.history.push(task);
            if self.history.len() > 100 {
                self.history.remove(0);
            }
            removed += 1;
        }
        self.state.queue_size = 0;
        self.state.pending_count = 0;
        self.update_statistics();
        removed
    }

    /// 获取状态快照
    pub fn get_state_snapshot(&self) -> SyncControlState {
        self.state.clone()
    }

    /// 获取配置
    pub fn get_config(&self) -> SyncConfig {
        self.config.clone()
    }

    /// 更新配置
    pub fn update_config(&mut self, config: SyncConfig) {
        self.config = config;
    }
}

async fn process_sync_task(task: &SyncTask) -> anyhow::Result<()> {
    if task.file_path.trim().is_empty() {
        tokio::time::sleep(Duration::from_millis(100)).await;
        return Ok(());
    }

    let metadata = fs::metadata(&task.file_path)
        .await
        .with_context(|| format!("无法访问同步文件: {}", task.file_path))?;

    if !metadata.is_file() {
        return Err(anyhow!("同步目标不是文件: {}", task.file_path));
    }

    let destination = resolve_sync_destination(task.clone())
        .await
        .context("计算同步目标位置失败")?;
    match &destination.target {
        ResolvedTarget::Local { final_path } => {
            let final_path = final_path.clone();
            if let Some(parent) = final_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("创建目录 {:?} 失败", parent))?;
            }
            fs::copy(&task.file_path, &final_path)
                .await
                .with_context(|| {
                    format!("复制文件到 {:?} 失败 (源: {})", final_path, task.file_path)
                })?;
            #[cfg(feature = "web_server")]
            {
                if let Some(base) = &destination.local_base {
                    if let Ok(file_meta) = fs::metadata(&final_path).await {
                        if let Err(err) = update_site_metadata(
                            base,
                            &destination,
                            &task,
                            &final_path,
                            file_meta.len(),
                        )
                        .await
                        {
                            eprintln!("更新站点元数据失败: {}", err);
                        }
                    }
                }
            }
        }
        ResolvedTarget::Http { url } => {
            let url = url.clone();
            let data = fs::read(&task.file_path)
                .await
                .with_context(|| format!("读取文件 {} 失败", task.file_path))?;
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .context("创建 HTTP 客户端失败")?;
            let response = client
                .put(&url)
                .body(data)
                .send()
                .await
                .with_context(|| format!("上传到 {} 失败", url))?;
            if !response.status().is_success() {
                return Err(anyhow!(
                    "HTTP 上传失败: {} (status: {})",
                    url,
                    response.status()
                ));
            }
        }
    }

    #[cfg(feature = "web_server")]
    if matches!(destination.target, ResolvedTarget::Http { .. }) {
        if let Err(err) = refresh_remote_site_metadata(&destination).await {
            eprintln!("刷新远程站点元数据失败: {}", err);
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
enum ResolvedTarget {
    Local { final_path: PathBuf },
    Http { url: String },
}

#[derive(Debug, Clone)]
struct SyncDestination {
    target: ResolvedTarget,
    local_base: Option<PathBuf>,
    env_id: Option<String>,
    env_name: Option<String>,
    site_id: Option<String>,
    site_name: Option<String>,
    site_http_host: Option<String>,
    env_file_host: Option<String>,
}

#[derive(Debug, Clone)]
struct DestinationContext {
    env_id: Option<String>,
    env_name: Option<String>,
    env_file_host: Option<String>,
    site_id: Option<String>,
    site_name: Option<String>,
    site_http_host: Option<String>,
}

async fn resolve_sync_destination(task: SyncTask) -> anyhow::Result<SyncDestination> {
    let task_for_lookup = task.clone();
    let destination_ctx = spawn_blocking({
        move || -> anyhow::Result<DestinationContext> {
            let mut ctx = DestinationContext {
                env_id: task_for_lookup.env_id.clone(),
                env_name: None,
                env_file_host: None,
                site_id: None,
                site_name: None,
                site_http_host: None,
            };

            let conn = remote_sync_handlers::open_sqlite()
                .map_err(|e| anyhow!("打开 remote_sync sqlite 失败: {}", e))?;

            if let Some(env_id) = task_for_lookup.env_id.as_deref() {
                let mut stmt = conn.prepare(
                    "SELECT name, file_server_host FROM remote_sync_envs WHERE id = ?1 LIMIT 1",
                )?;
                if let Ok((name, host)) = stmt.query_row([env_id], |row| {
                    let name: String = row.get(0)?;
                    let host: Option<String> = row.get(1)?;
                    Ok((name, host))
                }) {
                    ctx.env_name = Some(name);
                    ctx.env_file_host = host;
                }
            }

            if let Some(site_identifier) = task_for_lookup.target_site.as_deref() {
                let mut stmt = conn.prepare(
                    "SELECT id, name, http_host FROM remote_sync_sites \
                     WHERE id = ?1 OR name = ?1 LIMIT 1",
                )?;
                if let Ok((id, name, host)) = stmt.query_row([site_identifier], |row| {
                    let id: String = row.get(0)?;
                    let name: String = row.get(1)?;
                    let host: Option<String> = row.get(2)?;
                    Ok((id, name, host))
                }) {
                    ctx.site_id = Some(id);
                    ctx.site_name = Some(name);
                    ctx.site_http_host = host;
                }
            }

            Ok(ctx)
        }
    })
    .await
    .context("查询同步环境信息失败")??;

    let file_name = Path::new(&task.file_path)
        .file_name()
        .and_then(|os| os.to_str())
        .ok_or_else(|| anyhow!("无法解析文件名: {}", task.file_path))?;

    let mut path_segments: Vec<String> = Vec::new();
    if let Some(env) = destination_ctx
        .env_name
        .as_deref()
        .or(destination_ctx.env_id.as_deref())
    {
        path_segments.push(sanitize_path_segment(env));
    }
    if let Some(site) = destination_ctx
        .site_name
        .as_deref()
        .or(destination_ctx.site_id.as_deref())
    {
        path_segments.push(sanitize_path_segment(site));
    }
    if let Some(direction) = task.direction.as_deref() {
        path_segments.push(site_metadata::sanitize_path_segment(direction));
    }

    let local_base = destination_ctx
        .site_http_host
        .as_deref()
        .filter(|s| site_metadata::is_local_path_hint(s))
        .map(|s| site_metadata::normalize_local_base(s))
        .or_else(|| {
            destination_ctx
                .env_file_host
                .as_deref()
                .filter(|s| site_metadata::is_local_path_hint(s))
                .map(|s| site_metadata::normalize_local_base(s))
        });

    let http_base = destination_ctx
        .site_http_host
        .as_deref()
        .filter(|s| site_metadata::is_http_url(s))
        .map(|s| s.to_string())
        .or_else(|| {
            destination_ctx
                .env_file_host
                .as_deref()
                .filter(|s| site_metadata::is_http_url(s))
                .map(|s| s.to_string())
        });

    let sanitized_file = site_metadata::sanitize_path_segment(file_name);

    let env_id_clone = destination_ctx.env_id.clone();
    let env_name_clone = destination_ctx.env_name.clone();
    let site_id_clone = destination_ctx.site_id.clone();
    let site_name_clone = destination_ctx.site_name.clone();
    let site_http_clone = destination_ctx.site_http_host.clone();
    let env_file_host_clone = destination_ctx.env_file_host.clone();

    if let Some(http_base) = http_base {
        let mut url = http_base.trim_end_matches('/').to_string();
        for segment in &path_segments {
            url.push('/');
            url.push_str(segment);
        }
        url.push('/');
        url.push_str(&sanitized_file);
        return Ok(SyncDestination {
            target: ResolvedTarget::Http { url },
            local_base: None,
            env_id: env_id_clone,
            env_name: env_name_clone,
            site_id: site_id_clone,
            site_name: site_name_clone,
            site_http_host: site_http_clone,
            env_file_host: env_file_host_clone,
        });
    }

    let mut final_path = local_base
        .clone()
        .unwrap_or_else(|| PathBuf::from("output/remote_sync"));
    for segment in &path_segments {
        final_path.push(segment);
    }
    final_path.push(&sanitized_file);

    Ok(SyncDestination {
        target: ResolvedTarget::Local { final_path },
        local_base: Some(local_base.unwrap_or_else(|| PathBuf::from("output/remote_sync"))),
        env_id: env_id_clone,
        env_name: env_name_clone,
        site_id: site_id_clone,
        site_name: site_name_clone,
        site_http_host: site_http_clone,
        env_file_host: destination_ctx.env_file_host.clone(),
    })
}

#[cfg(feature = "web_server")]
async fn update_site_metadata(
    base: &Path,
    destination: &SyncDestination,
    task: &SyncTask,
    final_path: &Path,
    file_size: u64,
) -> anyhow::Result<()> {
    let mut metadata: SiteMetadataFile = match site_metadata::read_local_metadata(base).await {
        Ok(existing) => existing,
        Err(err) => {
            eprintln!("读取站点元数据失败，将创建新的 metadata.json: {err}");
            SiteMetadataFile::default()
        }
    };

    if metadata.generated_at.is_empty() {
        metadata.generated_at = site_metadata::timestamp_now();
    }
    if metadata.env_id.is_none() {
        metadata.env_id = destination.env_id.clone();
    }
    if metadata.env_name.is_none() {
        metadata.env_name = destination.env_name.clone();
    }
    if metadata.site_id.is_none() {
        metadata.site_id = destination.site_id.clone();
    }
    if metadata.site_name.is_none() {
        metadata.site_name = destination.site_name.clone();
    }
    if metadata.site_http_host.is_none() {
        metadata.site_http_host = destination
            .site_http_host
            .clone()
            .or_else(|| destination.env_file_host.clone());
    }

    let file_name = task
        .file_name
        .clone()
        .or_else(|| {
            final_path
                .file_name()
                .and_then(|os| os.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown.cba".to_string());

    let relative_path = final_path.strip_prefix(base).ok().map(|rel| {
        rel.components()
            .filter_map(|component| match component {
                Component::Normal(os_str) => Some(os_str.to_string_lossy().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/")
    });

    let download_host = destination
        .site_http_host
        .as_ref()
        .filter(|host| site_metadata::is_http_url(host))
        .cloned()
        .or_else(|| {
            destination
                .env_file_host
                .as_ref()
                .filter(|host| site_metadata::is_http_url(host))
                .cloned()
        });

    let download_url = download_host.map(|host| {
        format!(
            "{}/assets/archives/{}",
            host.trim_end_matches('/'),
            file_name.as_str()
        )
    });

    let updated_at = Utc::now().to_rfc3339();
    let file_path_display = final_path.to_string_lossy().to_string();

    if let Some(entry) = metadata
        .entries
        .iter_mut()
        .find(|entry| entry.file_name == file_name)
    {
        entry.file_path = file_path_display.clone();
        entry.file_size = file_size;
        entry.file_hash = task.file_hash.clone();
        entry.record_count = task.record_count;
        entry.direction = task.direction.clone();
        entry.source_env = task.source_env.clone();
        entry.download_url = download_url.clone();
        entry.updated_at = updated_at.clone();
        entry.relative_path = relative_path.clone();
    } else {
        metadata.entries.push(SiteMetadataEntry {
            file_name: file_name.clone(),
            file_path: file_path_display,
            file_size,
            file_hash: task.file_hash.clone(),
            record_count: task.record_count,
            direction: task.direction.clone(),
            source_env: task.source_env.clone(),
            download_url,
            relative_path: relative_path.clone(),
            updated_at: updated_at.clone(),
        });
    }

    metadata.generated_at = updated_at;

    site_metadata::write_local_metadata(base, &metadata).await?;

    if let Err(err) = site_metadata::write_cache(
        metadata.env_id.as_deref(),
        metadata.site_id.as_deref(),
        &metadata,
    )
    .await
    {
        eprintln!("写入站点元数据缓存失败: {err}");
    }

    Ok(())
}

#[cfg(feature = "web_server")]
async fn refresh_remote_site_metadata(destination: &SyncDestination) -> anyhow::Result<()> {
    let Some(http_host) = destination
        .site_http_host
        .as_deref()
        .filter(site_metadata::is_http_url_ref)
        .or_else(|| {
            destination
                .env_file_host
                .as_deref()
                .filter(site_metadata::is_http_url_ref)
        })
    else {
        return Ok(());
    };

    let mut metadata = site_metadata::fetch_remote_metadata(http_host).await?;
    if metadata.env_id.is_none() {
        metadata.env_id = destination.env_id.clone();
    }
    if metadata.env_name.is_none() {
        metadata.env_name = destination.env_name.clone();
    }
    if metadata.site_id.is_none() {
        metadata.site_id = destination.site_id.clone();
    }
    if metadata.site_name.is_none() {
        metadata.site_name = destination.site_name.clone();
    }
    if metadata.site_http_host.is_none() {
        metadata.site_http_host = destination
            .site_http_host
            .clone()
            .or_else(|| destination.env_file_host.clone());
    }
    if metadata.generated_at.is_empty() {
        metadata.generated_at = site_metadata::timestamp_now();
    }

    site_metadata::write_cache(
        metadata.env_id.as_deref(),
        metadata.site_id.as_deref(),
        &metadata,
    )
    .await?;

    Ok(())
}

// ========= MQTT 服务器管理 =========

/// 解析 rumqttd 配置文件路径（支持便携式部署）
pub fn resolve_rumqttd_config_path() -> anyhow::Result<std::path::PathBuf> {
    use std::env;

    // 1. 优先检查可执行文件同目录下的 rumqttd.toml（便携式部署，最高优先级）
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_config_path = exe_dir.join("rumqttd.toml");
            if exe_config_path.exists() {
                return Ok(exe_config_path);
            }
        }
    }

    // 2. 尝试当前工作目录
    let current_dir = env::current_dir()?;
    let current_config_path = current_dir.join("rumqttd.toml");
    if current_config_path.exists() {
        return Ok(current_config_path);
    }

    // 3. 尝试 remote-test-dir/test-real/rumqttd.toml（开发环境）
    let dev_config_path = current_dir
        .join("remote-test-dir")
        .join("test-real")
        .join("rumqttd.toml");
    if dev_config_path.exists() {
        return Ok(dev_config_path);
    }

    // 4. 如果都找不到，返回可执行文件同目录的路径（会在启动时失败，但至少不会编译错误）
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return Ok(exe_dir.join("rumqttd.toml"));
        }
    }

    Err(anyhow::anyhow!("无法找到 rumqttd.toml 配置文件"))
}

/// 解析 rumqttd 可执行文件路径（支持便携式部署）
fn resolve_rumqttd_binary() -> anyhow::Result<std::path::PathBuf> {
    use std::env;

    let binary_name = if cfg!(windows) {
        "rumqttd.exe"
    } else {
        "rumqttd"
    };

    // 1. 优先检查可执行文件同目录下的 rumqttd.exe（便携式部署，最高优先级）
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_binary_path = exe_dir.join(binary_name);
            if exe_binary_path.exists() {
                return Ok(exe_binary_path);
            }
        }
    }

    // 2. 尝试从 PATH 查找（系统安装）
    Ok(std::path::PathBuf::from(binary_name))
}

/// 启动 MQTT 服务器 (使用 rumqttd)
pub async fn start_mqtt_server(port: u16) -> anyhow::Result<()> {
    use std::env;
    use tokio::process::Command;

    // 检查是否已经在运行
    {
        let process_guard = MQTT_SERVER_PROCESS.read().await;
        if process_guard.is_some() {
            return Err(anyhow::anyhow!("MQTT 服务器已经在运行中"));
        }
    }

    // 解析配置文件路径（支持便携式部署）
    let config_path = resolve_rumqttd_config_path()?;

    // 检查配置文件是否存在
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "MQTT 配置文件不存在: {}",
            config_path.display()
        ));
    }

    // 解析可执行文件路径（支持便携式部署）
    let rumqttd_bin = resolve_rumqttd_binary()?;

    // 获取配置文件所在目录作为工作目录
    let config_dir = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("配置文件路径无效"))?;

    // 启动 rumqttd 进程
    let mut child = Command::new(&rumqttd_bin)
        .arg("-c")
        .arg(&config_path)
        .arg("-vv")
        .current_dir(config_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context(format!(
            "无法启动 rumqttd，请确保已安装或放置在可执行文件同目录: {}",
            rumqttd_bin.display()
        ))?;

    // 获取 stdout 和 stderr 管道
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // 启动日志收集任务
    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            capture_logs(stdout, "INFO").await;
        });
    }
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            capture_logs(stderr, "ERROR").await;
        });
    }

    // 等待一小段时间，确保进程启动成功
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 检查进程是否还在运行
    match child.try_wait() {
        Ok(Some(status)) => {
            // 进程已退出，说明启动失败
            return Err(anyhow::anyhow!("MQTT 服务器启动失败，退出状态: {}", status));
        }
        Ok(None) => {
            // 进程正在运行，成功
        }
        Err(e) => {
            return Err(anyhow::anyhow!("检查进程状态失败: {}", e));
        }
    }

    // 保存进程句柄
    {
        let mut process_guard = MQTT_SERVER_PROCESS.write().await;
        *process_guard = Some(child);
    }

    // 更新状态
    let mut center = SYNC_CONTROL_CENTER.write().await;
    center.mqtt_server = Some(MqttServerState {
        is_running: true,
        port,
        client_count: 0,
        message_count: 0,
        started_at: Some(SystemTime::now()),
    });

    log::info!("✅ MQTT 服务器已启动在端口 {}", port);
    Ok(())
}

/// 捕获日志输出到缓冲区
async fn capture_logs<R>(pipe: R, default_level: &str)
where
    R: tokio::io::AsyncRead + Unpin,
{
    use chrono::Local;
    use tokio::io::{AsyncBufReadExt, BufReader};

    let reader = BufReader::new(pipe);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let now = Local::now().format("%H:%M:%S").to_string();

        // 解析日志级别
        let level = if line.contains("INFO") {
            "INFO"
        } else if line.contains("WARN") {
            "WARN"
        } else if line.contains("ERROR") || line.contains("panicked") {
            "ERROR"
        } else if line.contains("DEBUG") {
            "DEBUG"
        } else {
            default_level
        };

        let log_entry = MqttBrokerLog {
            time: now,
            level: level.to_string(),
            message: line,
        };

        // 添加到缓冲区（保持最多 MAX_LOG_ENTRIES 条）
        let mut logs = MQTT_BROKER_LOGS.write().await;
        logs.push(log_entry);
        if logs.len() > MAX_LOG_ENTRIES {
            logs.remove(0);
        }
    }
}

/// 获取 MQTT Broker 日志
pub async fn get_mqtt_broker_logs() -> Vec<MqttBrokerLog> {
    let logs = MQTT_BROKER_LOGS.read().await;
    logs.clone()
}

/// 清空 MQTT Broker 日志
pub async fn clear_mqtt_broker_logs() {
    let mut logs = MQTT_BROKER_LOGS.write().await;
    logs.clear();
}

/// 检查 MQTT 进程是否真正在运行
/// 返回 (is_running, pid, exit_status)
pub async fn check_mqtt_process_status() -> (bool, Option<u32>, Option<String>) {
    let mut process_guard = MQTT_SERVER_PROCESS.write().await;

    if let Some(ref mut child) = *process_guard {
        // 尝试检查进程状态
        match child.try_wait() {
            Ok(Some(status)) => {
                // 进程已退出
                let exit_info = format!("exit code: {:?}", status.code());
                // 清理已退出的进程
                *process_guard = None;
                // 同时更新 SYNC_CONTROL_CENTER 的状态
                let mut center = SYNC_CONTROL_CENTER.write().await;
                center.mqtt_server = None;
                (false, None, Some(exit_info))
            }
            Ok(None) => {
                // 进程仍在运行
                let pid = child.id();
                (true, pid, None)
            }
            Err(e) => {
                // 无法检查状态
                (false, None, Some(format!("check error: {}", e)))
            }
        }
    } else {
        // 没有进程句柄
        (false, None, None)
    }
}

/// 停止 MQTT 服务器
pub async fn stop_mqtt_server() -> anyhow::Result<()> {
    // 获取并终止进程
    let mut process_guard = MQTT_SERVER_PROCESS.write().await;

    if let Some(mut child) = process_guard.take() {
        // 尝试优雅地终止进程
        match child.kill().await {
            Ok(_) => {
                log::info!("✅ MQTT 服务器进程已终止");
            }
            Err(e) => {
                log::warn!("⚠️ 终止 MQTT 服务器进程失败: {}", e);
            }
        }

        // 等待进程完全退出
        let _ = child.wait().await;
    } else {
        return Err(anyhow::anyhow!("MQTT 服务器未在运行"));
    }

    // 更新状态
    let mut center = SYNC_CONTROL_CENTER.write().await;
    center.mqtt_server = None;

    // 清空日志缓冲区
    clear_mqtt_broker_logs().await;

    log::info!("✅ MQTT 服务器已停止");
    Ok(())
}

// ========= 后台监控任务 =========

/// 启动监控任务
pub async fn start_monitoring() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

            // 更新连接状态
            update_connection_status().await;

            // 发送进度更新
            send_progress_update().await;

            // 检查告警条件
            check_alerts().await;
        }
    });
}

/// 更新连接状态
async fn update_connection_status() {
    use crate::data_interface::db_model::MQTT_CONNECT_STATUS;
    use crate::web_server::remote_runtime::REMOTE_RUNTIME;

    let mqtt_connected = {
        let status = MQTT_CONNECT_STATUS.lock().await;
        (*status).unwrap_or(false)
    };

    let watcher_active = {
        let runtime = REMOTE_RUNTIME.read().await;
        runtime.is_some()
    };

    let mut center = SYNC_CONTROL_CENTER.write().await;

    // 检测状态变化
    if center.state.mqtt_connected != mqtt_connected
        || center.state.watcher_active != watcher_active
    {
        center.state.mqtt_connected = mqtt_connected;
        center.state.watcher_active = watcher_active;

        if mqtt_connected {
            center.state.last_mqtt_connect_time = Some(SystemTime::now());
        }

        // 发送状态变更事件
        let _ = SYNC_EVENT_TX.send(SyncEvent::ConnectionChanged {
            mqtt_connected,
            watcher_active,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }
}

/// 发送进度更新
async fn send_progress_update() {
    let center = SYNC_CONTROL_CENTER.read().await;

    let _ = SYNC_EVENT_TX.send(SyncEvent::ProgressUpdate {
        total: center.state.total_synced + center.state.total_failed,
        completed: center.state.total_synced,
        failed: center.state.total_failed,
        pending: center.state.pending_count as u64,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
}

/// 检查告警条件
async fn check_alerts() {
    let center = SYNC_CONTROL_CENTER.read().await;

    // 检查MQTT断连
    if center.state.is_running && !center.state.mqtt_connected {
        if center.state.mqtt_reconnect_count > 5 {
            let _ = SYNC_EVENT_TX.send(SyncEvent::Alert {
                level: "critical".to_string(),
                message: "MQTT连接持续失败，请检查网络和服务器配置".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    // 检查队列积压
    if center.state.queue_size > 100 {
        let _ = SYNC_EVENT_TX.send(SyncEvent::Alert {
            level: "warning".to_string(),
            message: format!(
                "同步队列积压严重，当前待处理: {} 个文件",
                center.state.queue_size
            ),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    // 检查失败率
    let total = center.state.total_synced + center.state.total_failed;
    if total > 10 {
        let failure_rate = center.state.total_failed as f64 / total as f64;
        if failure_rate > 0.3 {
            let _ = SYNC_EVENT_TX.send(SyncEvent::Alert {
                level: "error".to_string(),
                message: format!("同步失败率过高: {:.1}%", failure_rate * 100.0),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    }
}
