// Type definitions for Remote Sync GUI

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Active tab in the main window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    EnvConfig,
    RemoteSites,
    SyncControl,
    FileBrowser,
    LogViewer,
    TestTools,
}

impl Tab {
    pub fn label(&self) -> &str {
        match self {
            Tab::Dashboard => "📊 仪表盘",
            Tab::EnvConfig => "⚙️ 环境配置",
            Tab::RemoteSites => "🌐 站点管理",
            Tab::SyncControl => "🔄 同步控制",
            Tab::FileBrowser => "📁 文件浏览",
            Tab::LogViewer => "📝 日志",
            Tab::TestTools => "🧪 测试工具",
        }
    }

    pub fn all() -> Vec<Tab> {
        vec![
            Tab::Dashboard,
            Tab::EnvConfig,
            Tab::RemoteSites,
            Tab::SyncControl,
            Tab::FileBrowser,
            Tab::LogViewer,
            Tab::TestTools,
        ]
    }
}

/// Service status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

impl ServiceStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, ServiceStatus::Running)
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            ServiceStatus::Running => egui::Color32::from_rgb(0, 200, 0),
            ServiceStatus::Stopped => egui::Color32::GRAY,
            ServiceStatus::Starting | ServiceStatus::Stopping => egui::Color32::from_rgb(255, 165, 0),
            ServiceStatus::Error(_) => egui::Color32::from_rgb(255, 0, 0),
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            ServiceStatus::Running => "✓",
            ServiceStatus::Stopped => "■",
            ServiceStatus::Starting | ServiceStatus::Stopping => "⏳",
            ServiceStatus::Error(_) => "✗",
        }
    }
}

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub status: ServiceStatus,
    pub port: Option<u16>,
    pub script_path: PathBuf,
    pub log_file: String,
    pub started_at: Option<DateTime<Utc>>,
    pub process_id: Option<u32>,
    pub cpu_usage: f32,
    pub memory_mb: u64,
}

impl ServiceInfo {
    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|start| {
            let now = Utc::now();
            (now - start).to_std().unwrap_or_default()
        })
    }
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    pub fn color(&self) -> egui::Color32 {
        match self {
            HealthStatus::Healthy => egui::Color32::from_rgb(0, 200, 0),
            HealthStatus::Degraded => egui::Color32::from_rgb(255, 165, 0),
            HealthStatus::Unhealthy => egui::Color32::from_rgb(255, 0, 0),
            HealthStatus::Unknown => egui::Color32::GRAY,
        }
    }
}

/// Environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub id: String,
    pub name: String,
    pub location: String,
    pub location_dbs: Vec<i32>,
    pub mqtt_host: String,
    pub mqtt_port: u16,
    pub file_server_host: String,
    pub reconnect_initial_ms: Option<u64>,
    pub reconnect_max_ms: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "测试环境".to_string(),
            location: "SITE1112".to_string(),
            location_dbs: vec![1112],
            mqtt_host: "127.0.0.1".to_string(),
            mqtt_port: 1883,
            file_server_host: "http://127.0.0.1:8081/assets/archives".to_string(),
            reconnect_initial_ms: Some(1000),
            reconnect_max_ms: Some(30000),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Remote site information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSite {
    pub id: String,
    pub env_id: String,
    pub name: String,
    pub location: String,
    pub http_host: String,
    pub dbnums: Vec<i32>,
    pub notes: Option<String>,
    pub enabled: bool,
    pub health: HealthStatus,
    pub last_check: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for RemoteSite {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            env_id: String::new(),
            name: String::new(),
            location: String::new(),
            http_host: String::new(),
            dbnums: Vec::new(),
            notes: None,
            enabled: true,
            health: HealthStatus::Unknown,
            last_check: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Sync statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncStats {
    pub total_synced: u64,
    pub total_failed: u64,
    pub pending_count: u32,
    pub sync_rate_mbps: f64,
    pub avg_sync_time_ms: u64,
    pub last_sync_time: Option<DateTime<Utc>>,
}

/// Sync record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub target: String,
    pub file_path: String,
    pub file_size: u64,
    pub record_count: Option<u64>,
    pub status: SyncStatus,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

/// Sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl SyncStatus {
    pub fn icon(&self) -> &str {
        match self {
            SyncStatus::Pending => "⏳",
            SyncStatus::InProgress => "🔄",
            SyncStatus::Completed => "✓",
            SyncStatus::Failed => "✗",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            SyncStatus::Completed => egui::Color32::from_rgb(0, 200, 0),
            SyncStatus::Failed => egui::Color32::from_rgb(255, 0, 0),
            SyncStatus::InProgress => egui::Color32::from_rgb(0, 150, 255),
            SyncStatus::Pending => egui::Color32::GRAY,
        }
    }
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub service: String,
    pub message: String,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warn" | "warning" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            LogLevel::Error => egui::Color32::from_rgb(255, 0, 0),
            LogLevel::Warn => egui::Color32::from_rgb(255, 165, 0),
            LogLevel::Info => egui::Color32::from_rgb(200, 200, 200),
            LogLevel::Debug => egui::Color32::from_rgb(150, 150, 150),
            LogLevel::Trace => egui::Color32::from_rgb(100, 100, 100),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

/// System event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    ServiceStarted { service: String, timestamp: DateTime<Utc> },
    ServiceStopped { service: String, timestamp: DateTime<Utc> },
    ServiceError { service: String, error: String, timestamp: DateTime<Utc> },
    SyncCompleted { source: String, target: String, duration_ms: u64, timestamp: DateTime<Utc> },
    SyncFailed { source: String, target: String, error: String, timestamp: DateTime<Utc> },
    ConfigUpdated { timestamp: DateTime<Utc> },
}

impl SystemEvent {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            SystemEvent::ServiceStarted { timestamp, .. } => *timestamp,
            SystemEvent::ServiceStopped { timestamp, .. } => *timestamp,
            SystemEvent::ServiceError { timestamp, .. } => *timestamp,
            SystemEvent::SyncCompleted { timestamp, .. } => *timestamp,
            SystemEvent::SyncFailed { timestamp, .. } => *timestamp,
            SystemEvent::ConfigUpdated { timestamp } => *timestamp,
        }
    }

    pub fn description(&self) -> String {
        match self {
            SystemEvent::ServiceStarted { service, .. } => format!("服务启动: {}", service),
            SystemEvent::ServiceStopped { service, .. } => format!("服务停止: {}", service),
            SystemEvent::ServiceError { service, error, .. } => format!("服务错误 {}: {}", service, error),
            SystemEvent::SyncCompleted { source, target, duration_ms, .. } => {
                format!("同步完成: {} → {} ({}ms)", source, target, duration_ms)
            }
            SystemEvent::SyncFailed { source, target, error, .. } => {
                format!("同步失败: {} → {} ({})", source, target, error)
            }
            SystemEvent::ConfigUpdated { .. } => "配置已更新".to_string(),
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            SystemEvent::ServiceStarted { .. } => "▶",
            SystemEvent::ServiceStopped { .. } => "■",
            SystemEvent::ServiceError { .. } => "✗",
            SystemEvent::SyncCompleted { .. } => "✓",
            SystemEvent::SyncFailed { .. } => "✗",
            SystemEvent::ConfigUpdated { .. } => "⚙",
        }
    }
}

/// File browser item
#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
}

/// Test scenario
#[derive(Debug, Clone)]
pub struct TestScenario {
    pub name: String,
    pub description: String,
    pub duration_seconds: u32,
    pub sync_interval_seconds: u32,
    pub files_per_sync: u32,
    pub direction: SyncDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Site1112To7000,
    Site7000To1112,
    Bidirectional,
    Random,
}

impl SyncDirection {
    pub fn label(&self) -> &str {
        match self {
            SyncDirection::Site1112To7000 => "1112 → 7000",
            SyncDirection::Site7000To1112 => "7000 → 1112",
            SyncDirection::Bidirectional => "双向",
            SyncDirection::Random => "随机",
        }
    }
}
