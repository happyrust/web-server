//! Dashboard API处理器
//!
//! 提供增量更新实时监控仪表盘所需的所有API接口

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::SystemTime;

use crate::data_interface::failed_task_queue::{FailedTask, FailedTaskQueue, FailedTaskType};
use crate::web_server::{AppState, sync_control_center::SYNC_CONTROL_CENTER};

/// 获取失败任务队列实例（共享同一持久化文件）
fn get_failed_queue() -> FailedTaskQueue {
    use std::path::PathBuf;
    FailedTaskQueue::new(PathBuf::from("assets/failed_tasks.json"))
}

// ========= 数据结构定义 =========

/// 仪表盘概览统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    /// 实时任务统计
    pub task_stats: TaskStatistics,

    /// 失败任务统计
    pub failed_task_stats: FailedTaskStatistics,

    /// 性能指标
    pub performance_metrics: PerformanceMetrics,

    /// 最近事件
    pub recent_events: Vec<RecentEvent>,
}

/// 任务统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatistics {
    /// 进行中的任务数
    pub in_progress: usize,

    /// 已完成的任务数（24小时内）
    pub completed_today: usize,

    /// 失败的任务数（24小时内）
    pub failed_today: usize,

    /// 待处理的任务数
    pub pending: usize,
}

/// 失败任务统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedTaskStatistics {
    /// 总失败任务数
    pub total: usize,

    /// 待重试任务数
    pub pending_retry: usize,

    /// 等待中任务数
    pub waiting: usize,

    /// 已耗尽任务数（需要人工介入）
    pub exhausted: usize,

    /// 按类型分组的统计
    pub by_type: HashMap<String, usize>,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 平均同步时间（秒）
    pub avg_sync_time_secs: f64,

    /// 同步成功率（百分比）
    pub success_rate: f64,

    /// 当前同步速率（MB/s）
    pub current_sync_rate_mbps: f64,

    /// 最近1小时处理的任务数
    pub tasks_last_hour: usize,
}

/// 最近事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEvent {
    /// 事件时间
    pub timestamp: DateTime<Utc>,

    /// 事件类型
    pub event_type: EventType,

    /// 事件描述
    pub message: String,

    /// 关联的任务ID（如果有）
    pub task_id: Option<String>,
}

/// 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    SystemAlert,
    RetrySuccess,
}

/// 活跃任务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTaskInfo {
    /// 任务ID
    pub task_id: String,

    /// 任务名称
    pub task_name: String,

    /// 任务状态
    pub status: String,

    /// 进度百分比（0-100）
    pub progress: f32,

    /// 开始时间
    pub started_at: DateTime<Utc>,

    /// 预计剩余时间（秒）
    pub estimated_remaining_secs: Option<u64>,

    /// 文件路径
    pub file_path: Option<String>,

    /// 数据库编号
    pub db_num: Option<u32>,
}

/// 任务详细信息（包含增删改统计）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetailInfo {
    /// 任务ID
    pub task_id: String,

    /// 任务名称
    pub task_name: String,

    /// 文件路径
    pub file_path: String,

    /// 会话号范围
    pub sesno_range: Option<String>,

    /// 数据库编号
    pub db_num: Option<u32>,

    /// 增删改统计
    pub change_statistics: ChangeStatistics,

    /// RefNo列表（前100个）
    pub refnos: Vec<String>,

    /// 任务耗时（毫秒）
    pub duration_ms: u64,

    /// 任务状态
    pub status: String,

    /// 错误信息（如果失败）
    pub error: Option<String>,
}

/// 增删改统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeStatistics {
    /// 新增元素数量
    pub added: usize,

    /// 修改元素数量
    pub modified: usize,

    /// 删除元素数量
    pub deleted: usize,

    /// 总数
    pub total: usize,
}

/// 时间线查询参数
#[derive(Debug, Deserialize)]
pub struct TimelineQueryParams {
    /// 时间窗口：1h, 24h, 7d, 30d
    #[serde(default = "default_window")]
    pub window: String,

    /// 环境ID过滤
    pub env_id: Option<String>,
}

fn default_window() -> String {
    "24h".to_string()
}

/// 时间线数据点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineDataPoint {
    /// 时间戳
    pub timestamp: DateTime<Utc>,

    /// 成功任务数
    pub success_count: usize,

    /// 失败任务数
    pub failure_count: usize,

    /// 平均耗时（秒）
    pub avg_duration_secs: f64,

    /// 总数据量（字节）
    pub total_bytes: u64,
}

/// 失败任务查询参数
#[derive(Debug, Deserialize)]
pub struct FailedTaskQueryParams {
    /// 按任务类型过滤
    pub task_type: Option<String>,

    /// 按优先级过滤
    pub priority: Option<u8>,

    /// 起始时间（Unix timestamp）
    pub since: Option<u64>,

    /// 仅显示待重试的任务
    #[serde(default)]
    pub pending_only: bool,

    /// 仅显示已耗尽的任务
    #[serde(default)]
    pub exhausted_only: bool,
}

// ========= API Handlers =========

/// 获取仪表盘概览统计
pub async fn get_dashboard_summary(
    _state: State<AppState>,
) -> Result<Json<DashboardSummary>, StatusCode> {
    // 1. 获取同步控制中心状态
    let sync_state = {
        let center = SYNC_CONTROL_CENTER.read().await;
        center.get_state_snapshot()
    };

    // 2. 获取失败任务统计
    let failed_stats = get_failed_task_statistics().await;

    // 3. 获取性能指标
    let performance = PerformanceMetrics {
        avg_sync_time_secs: sync_state.avg_sync_time_ms as f64 / 1000.0,
        success_rate: calculate_success_rate(&sync_state).await,
        current_sync_rate_mbps: sync_state.sync_rate_mbps,
        tasks_last_hour: get_tasks_last_hour().await,
    };

    // 4. 获取最近事件
    let recent_events = get_recent_events().await;

    // 5. 构建任务统计
    let task_stats = TaskStatistics {
        in_progress: sync_state.queue_size as usize,
        completed_today: get_completed_tasks_today().await,
        failed_today: sync_state.total_failed as usize,
        pending: sync_state.pending_count as usize,
    };

    Ok(Json(DashboardSummary {
        task_stats,
        failed_task_stats: failed_stats,
        performance_metrics: performance,
        recent_events,
    }))
}

/// 获取活跃任务列表
pub async fn get_active_tasks(
    _state: State<AppState>,
) -> Result<Json<Vec<ActiveTaskInfo>>, StatusCode> {
    let center = SYNC_CONTROL_CENTER.read().await;
    let running_tasks = &center.running_tasks;

    let mut active_tasks = Vec::new();

    for (task_id, task) in running_tasks.iter() {
        // 获取任务进度（从ProgressHub或估算）
        let progress = estimate_task_progress(task);

        let started_at = system_time_to_datetime(task.started_at.unwrap_or(task.created_at));

        active_tasks.push(ActiveTaskInfo {
            task_id: task_id.clone(),
            task_name: task
                .file_name
                .clone()
                .unwrap_or_else(|| "未知任务".to_string()),
            status: format!("{:?}", task.status),
            progress,
            started_at,
            estimated_remaining_secs: None,
            file_path: Some(task.file_path.clone()),
            db_num: None, // SyncTask 不包含 db_num 字段
        });
    }

    Ok(Json(active_tasks))
}

/// 获取失败任务列表（支持过滤）
pub async fn get_failed_tasks(
    _state: State<AppState>,
    Query(params): Query<FailedTaskQueryParams>,
) -> Result<Json<Vec<FailedTask>>, StatusCode> {
    let queue = get_failed_queue();
    let all_tasks = queue.get_all_tasks().await;

    // 应用过滤条件
    let filtered_tasks: Vec<FailedTask> = all_tasks
        .into_iter()
        .filter(|task| {
            // 任务类型过滤
            if let Some(ref type_filter) = params.task_type {
                let task_type_str = match &task.task_type {
                    FailedTaskType::DatabaseQuery { .. } => "database_query",
                    FailedTaskType::Compression { .. } => "compression",
                    FailedTaskType::IncrementUpdate { .. } => "increment_update",
                    FailedTaskType::MqttPublish { .. } => "mqtt_publish",
                };
                if task_type_str != type_filter {
                    return false;
                }
            }

            // 优先级过滤
            if let Some(priority) = params.priority {
                if task.priority != priority {
                    return false;
                }
            }

            // 时间过滤
            if let Some(since) = params.since {
                let since_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(since);
                if task.first_failed_at < since_time {
                    return false;
                }
            }

            // 待重试过滤
            if params.pending_only && !task.should_retry() {
                return false;
            }

            // 已耗尽过滤
            if params.exhausted_only && !task.is_exhausted() {
                return false;
            }

            true
        })
        .collect();

    Ok(Json(filtered_tasks))
}

/// 获取任务详细信息
pub async fn get_task_details(
    _state: State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskDetailInfo>, StatusCode> {
    // 尝试从历史记录中获取任务详情
    let details = get_task_details_from_db(&task_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(details))
}

/// 获取时间线统计数据
pub async fn get_timeline_stats(
    _state: State<AppState>,
    Query(params): Query<TimelineQueryParams>,
) -> Result<Json<Vec<TimelineDataPoint>>, StatusCode> {
    let datapoints = query_timeline_data(&params.window, params.env_id.as_deref()).await;
    Ok(Json(datapoints))
}

/// 手动重试失败任务
pub async fn retry_failed_task(
    _state: State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let queue = get_failed_queue();
    let tasks = queue.get_all_tasks().await;

    // 查找目标任务
    let task = tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // TODO: 调用实际的重试逻辑
    // 这里需要根据任务类型调用相应的重试函数

    Ok(Json(json!({
        "status": "success",
        "message": format!("任务 {} 已加入重试队列", task_id),
        "task_id": task_id
    })))
}

/// 清理已耗尽的失败任务
pub async fn cleanup_exhausted_tasks(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let queue = get_failed_queue();
    let removed_count = queue.cleanup_exhausted().await;

    Ok(Json(json!({
        "status": "success",
        "message": format!("已清理 {} 个已耗尽的任务", removed_count),
        "removed_count": removed_count
    })))
}

// ========= 辅助函数 =========

/// 获取失败任务统计
async fn get_failed_task_statistics() -> FailedTaskStatistics {
    let queue = get_failed_queue();
    let stats = queue.get_stats().await;
    let all_tasks = queue.get_all_tasks().await;

    // 按类型分组统计
    let mut by_type = HashMap::new();
    for task in &all_tasks {
        let type_name = match &task.task_type {
            FailedTaskType::DatabaseQuery { .. } => "database_query",
            FailedTaskType::Compression { .. } => "compression",
            FailedTaskType::IncrementUpdate { .. } => "increment_update",
            FailedTaskType::MqttPublish { .. } => "mqtt_publish",
        };
        *by_type.entry(type_name.to_string()).or_insert(0) += 1;
    }

    FailedTaskStatistics {
        total: stats.total,
        pending_retry: stats.pending,
        waiting: stats.waiting,
        exhausted: stats.exhausted,
        by_type,
    }
}

/// 计算成功率
async fn calculate_success_rate(
    sync_state: &crate::web_server::sync_control_center::SyncControlState,
) -> f64 {
    let total = sync_state.total_synced + sync_state.total_failed;
    if total == 0 {
        return 100.0;
    }
    (sync_state.total_synced as f64 / total as f64) * 100.0
}

/// 获取最近1小时的任务数
async fn get_tasks_last_hour() -> usize {
    // TODO: 从数据库查询最近1小时的任务数
    0
}

/// 获取今天完成的任务数
async fn get_completed_tasks_today() -> usize {
    // TODO: 从数据库查询今天完成的任务数
    0
}

/// 获取最近事件
async fn get_recent_events() -> Vec<RecentEvent> {
    let center = SYNC_CONTROL_CENTER.read().await;
    let logs = center.get_logs(10);

    logs.into_iter()
        .map(|log| {
            let event_type = if log.message.contains("成功") {
                EventType::TaskCompleted
            } else if log.message.contains("失败") {
                EventType::TaskFailed
            } else if log.message.contains("开始") {
                EventType::TaskStarted
            } else {
                EventType::SystemAlert
            };

            RecentEvent {
                timestamp: system_time_to_datetime(log.timestamp),
                event_type,
                message: log.message,
                task_id: None,
            }
        })
        .collect()
}

/// 估算任务进度
fn estimate_task_progress(task: &crate::web_server::sync_control_center::SyncTask) -> f32 {
    // 简单估算：根据任务状态返回进度
    use crate::web_server::sync_control_center::SyncTaskStatus;
    match task.status {
        SyncTaskStatus::Pending => 0.0,
        SyncTaskStatus::Running => 50.0,
        SyncTaskStatus::Completed => 100.0,
        SyncTaskStatus::Failed => 0.0,
        SyncTaskStatus::Cancelled => 0.0,
    }
}

/// 从数据库获取任务详情
async fn get_task_details_from_db(task_id: &str) -> anyhow::Result<TaskDetailInfo> {
    // TODO: 从 remote_sync_logs 表查询任务详情
    // 这里先返回一个占位符
    Err(anyhow::anyhow!("Task not found"))
}

/// 查询时间线数据
async fn query_timeline_data(window: &str, env_id: Option<&str>) -> Vec<TimelineDataPoint> {
    use rusqlite::Connection;
    use std::path::Path;

    // 计算时间范围
    let (hours_ago, interval_minutes) = match window {
        "1h" => (1, 5),       // 最近1小时，5分钟间隔
        "24h" => (24, 60),    // 最近24小时，1小时间隔
        "7d" => (168, 360),   // 最近7天，6小时间隔
        "30d" => (720, 1440), // 最近30天，1天间隔
        _ => (24, 60),        // 默认24小时
    };

    let db_path = Path::new("deployment_sites.sqlite");
    if !db_path.exists() {
        return Vec::new();
    }

    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // 查询时间线数据
    let query = format!(
        "SELECT
            datetime((started_at / 1000000000 / {} * {}) * {}, 'unixepoch') as time_bucket,
            COUNT(*) as total_count,
            SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as success_count,
            SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failure_count,
            AVG(CASE WHEN completed_at IS NOT NULL AND started_at IS NOT NULL
                THEN (completed_at - started_at) / 1000000000.0 ELSE 0 END) as avg_duration,
            SUM(file_size) as total_bytes
        FROM remote_sync_logs
        WHERE started_at >= (strftime('%s', 'now') - {} * 3600) * 1000000000
        {}
        GROUP BY time_bucket
        ORDER BY time_bucket ASC",
        interval_minutes * 60,
        interval_minutes * 60,
        interval_minutes * 60,
        hours_ago,
        env_id
            .map(|id| format!("AND env_id = '{}'", id))
            .unwrap_or_default()
    );

    let mut stmt = match conn.prepare(&query) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map([], |row| {
        let timestamp_str: String = row.get(0)?;
        let success_count: i64 = row.get(1)?;
        let failure_count: i64 = row.get(2)?;
        let avg_duration: f64 = row.get(3)?;
        let total_bytes: i64 = row.get(4)?;

        // 解析时间戳
        let timestamp = chrono::NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
            .unwrap_or_else(Utc::now);

        Ok(TimelineDataPoint {
            timestamp,
            success_count: success_count as usize,
            failure_count: failure_count as usize,
            avg_duration_secs: avg_duration,
            total_bytes: total_bytes as u64,
        })
    });

    match rows {
        Ok(iter) => iter.filter_map(Result::ok).collect(),
        Err(_) => Vec::new(),
    }
}

/// SystemTime转DateTime
fn system_time_to_datetime(st: SystemTime) -> DateTime<Utc> {
    let duration = st
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or_else(|| Utc::now())
}

// ========= 增量元素详情 API =========

/// 增量同步元素详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementElementDetail {
    /// 元素REFNO
    pub refno: String,
    /// 元素类型(noun)
    pub noun: String,
    /// 操作类型 (Add/Modified/Deleted)
    pub operation: String,
    /// 数据库编号
    pub db_num: i32,
    /// 会话号
    pub sesno: Option<i32>,
    /// 元素名称
    pub name: Option<String>,
    /// 元素属性(JSON格式)
    pub attributes: Option<serde_json::Value>,
}

/// 增量元素列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementElementsResponse {
    /// session范围描述
    pub session_range: String,
    /// 统计信息
    pub stats: ElementOperationStats,
    /// 元素列表
    pub elements: Vec<IncrementElementDetail>,
    /// 总数
    pub total: usize,
}

/// 元素操作统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementOperationStats {
    pub total_added: usize,
    pub total_modified: usize,
    pub total_deleted: usize,
}

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct IncrementElementsQuery {
    /// 操作类型过滤 (Add/Modified/Deleted)
    pub operation: Option<String>,
    /// 分页 - 限制数量
    pub limit: Option<usize>,
    /// 分页 - 偏移量
    pub offset: Option<usize>,
}

/// 获取增量同步的元素详情列表
///
/// 路径参数:
/// - sync_id: e3d_sync表的记录ID
pub async fn get_increment_elements(
    _state: State<AppState>,
    Path(sync_id): Path<String>,
    Query(params): Query<IncrementElementsQuery>,
) -> Result<Json<IncrementElementsResponse>, StatusCode> {
    use aios_core::SUL_DB;
    use aios_core::rs_surreal::query_ext::SurrealQueryExt;

    println!(
        "📋 查询增量元素详情: sync_id={}, filter={:?}",
        sync_id, params.operation
    );

    // 1. 查询 e3d_sync 表获取 session_range 和统计信息
    let sync_query = format!(
        "SELECT session_range, total_added, total_modified, total_deleted, db_num FROM e3d_sync WHERE id = e3d_sync:{}",
        sync_id
    );

    let sync_records: Vec<serde_json::Value> =
        SUL_DB.query_take(&sync_query, 0).await.map_err(|e| {
            eprintln!("查询e3d_sync失败: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if sync_records.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let sync_record = &sync_records[0];
    let session_range = sync_record
        .get("session_range")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let total_added = sync_record
        .get("total_added")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let total_modified = sync_record
        .get("total_modified")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let total_deleted = sync_record
        .get("total_deleted")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let db_num = sync_record
        .get("db_num")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    // 2. 解析session_range (例如 "1154..=1190")
    let (start_sesno, end_sesno) = parse_session_range(&session_range).unwrap_or((0, 0));

    if start_sesno == 0 && end_sesno == 0 {
        return Ok(Json(IncrementElementsResponse {
            session_range,
            stats: ElementOperationStats {
                total_added,
                total_modified,
                total_deleted,
            },
            elements: Vec::new(),
            total: 0,
        }));
    }

    // 3. 查询 pe 表获取该session范围内的元素
    // 注意: 由于 pe 表不存储 operation 类型,我们只能返回该 sesno 范围内的所有元素
    // 如果需要区分操作类型,需要从 PDMS 文件实时解析或者建立新的存储表

    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let elements_query = format!(
        "SELECT refno, noun, sesno, db_num, name FROM pe WHERE sesno >= {} AND sesno <= {} AND db_num = {} LIMIT {} START {}",
        start_sesno, end_sesno, db_num, limit, offset
    );

    let pe_records: Vec<serde_json::Value> =
        SUL_DB.query_take(&elements_query, 0).await.map_err(|e| {
            eprintln!("查询pe表失败: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    println!("✅ 查询到 {} 个元素", pe_records.len());

    let elements: Vec<IncrementElementDetail> = pe_records
        .into_iter()
        .map(|record| {
            let refno = record
                .get("refno")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let noun = record
                .get("noun")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let sesno = record
                .get("sesno")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);

            let db_num_val = record
                .get("db_num")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32)
                .unwrap_or(db_num);

            let name = record
                .get("name")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());

            // 由于pe表不存储operation类型,我们设置为"Unknown"
            // 实际的操作类型需要从PDMS文件解析或建立element_changes表存储
            let operation = "Unknown".to_string();

            IncrementElementDetail {
                refno,
                noun,
                operation,
                db_num: db_num_val,
                sesno,
                name,
                attributes: Some(record.clone()),
            }
        })
        .collect();

    let total = elements.len();

    Ok(Json(IncrementElementsResponse {
        session_range,
        stats: ElementOperationStats {
            total_added,
            total_modified,
            total_deleted,
        },
        elements,
        total,
    }))
}

/// 解析 session_range 字符串 (例如 "1154..=1190")
fn parse_session_range(range_str: &str) -> Option<(i32, i32)> {
    let parts: Vec<&str> = range_str.split("..=").collect();
    if parts.len() == 2 {
        let start = parts[0].trim().parse::<i32>().ok()?;
        let end = parts[1].trim().parse::<i32>().ok()?;
        Some((start, end))
    } else {
        None
    }
}
