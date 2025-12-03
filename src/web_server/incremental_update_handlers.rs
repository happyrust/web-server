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

use crate::web_server::{AppState, resolve_archives_dir};

/// 增量更新检测状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateDetectionStatus {
    /// 空闲，未运行检测
    Idle,
    /// 正在扫描变更
    Scanning,
    /// 检测到变更，等待处理
    ChangesDetected,
    /// 正在同步
    Syncing,
    /// 同步完成
    Completed,
    /// 错误
    Error(String),
}

/// 增量更新信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalUpdateInfo {
    /// 站点ID
    pub site_id: String,
    /// 站点名称
    pub site_name: String,
    /// 上次同步时间
    pub last_sync_time: Option<DateTime<Utc>>,
    /// 检测状态
    pub detection_status: UpdateDetectionStatus,
    /// 待同步项目数
    pub pending_items: usize,
    /// 已同步项目数
    pub synced_items: usize,
    /// 变更文件列表
    pub changed_files: Vec<ChangedFile>,
    /// 增量大小（字节）
    pub increment_size: u64,
    /// 预计同步时间（秒）
    pub estimated_sync_time: u32,
}

/// 变更文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedFile {
    /// 文件路径
    pub path: String,
    /// 变更类型
    pub change_type: ChangeType,
    /// 文件大小
    pub size: u64,
    /// 修改时间
    pub modified_time: DateTime<Utc>,
    /// 数据库编号
    pub db_num: Option<u32>,
}

/// 变更类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

/// 从 SurrealDB 读取增量同步历史记录（支持分页）
async fn get_increment_sync_history_paged(limit: usize, offset: usize) -> Vec<serde_json::Value> {
    use crate::web_server::mqtt_monitor_handlers::MQTT_MESSAGE_DELIVERY;
    use aios_core::SUL_DB;
    use aios_core::rs_surreal::query_ext::SurrealQueryExt;

    // 查询历史记录，支持分页
    // 注意：使用 SELECT VALUE 或选择具体字段，避免返回 record 类型的 id 字段
    let query = format!(
        "SELECT file_names, file_hashes, timestamp, location, file_server_host, session_range, total_added, total_modified, total_deleted, is_full_sync, db_num FROM e3d_sync ORDER BY timestamp DESC LIMIT {} START {}",
        limit, offset
    );

    // 使用 query_take 扩展方法（遵循 AGENTS.md 规范）
    match SUL_DB.query_take::<Vec<serde_json::Value>>(&query, 0).await {
        Ok(records) => {
            // 读取MQTT消息投递状态
            let delivery = MQTT_MESSAGE_DELIVERY.read().await;

            // 处理并规范化 JSON 格式
            records
                .into_iter()
                .map(|mut record| {
                    // 计算文件数量
                    let file_count = record
                        .get("file_names")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.len())
                        .unwrap_or(0);

                    // 提取所需的字段（在获得可变引用之前）
                    let location = record
                        .get("location")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let record_timestamp = record.get("timestamp").cloned();

                    // 添加 file_count 字段
                    if let Some(obj) = record.as_object_mut() {
                        obj.insert("file_count".to_string(), json!(file_count));

                        // 尝试关联MQTT消息投递状态
                        // 通过location和发送时间匹配消息（查找最近发送的、匹配location的消息）
                        if let Some(location_str) = location {
                            // 查找匹配的投递状态：查找发送者location匹配且时间最接近的消息
                            let mut matched_status: Option<
                                &crate::web_server::mqtt_monitor_handlers::MessageDeliveryStatus,
                            > = None;
                            let mut min_time_diff: Option<chrono::Duration> = None;

                            for (_, status) in delivery.iter() {
                                // 检查发送者location是否匹配
                                if status.sender_location == location_str {
                                    // 计算时间差（如果record有timestamp）
                                    if let Some(ts) = &record_timestamp {
                                        // 尝试解析时间戳
                                        let record_time = if let Some(ts_str) = ts.as_str() {
                                            chrono::DateTime::parse_from_rfc3339(ts_str)
                                                .ok()
                                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                        } else {
                                            None
                                        };

                                        if let Some(rt) = record_time {
                                            let diff = (rt - status.sent_at).abs();
                                            if min_time_diff.is_none()
                                                || diff < min_time_diff.unwrap()
                                            {
                                                min_time_diff = Some(diff);
                                                matched_status = Some(status);
                                            }
                                        } else {
                                            // 如果无法解析时间，直接使用第一个匹配的
                                            if matched_status.is_none() {
                                                matched_status = Some(status);
                                            }
                                        }
                                    } else {
                                        // 如果没有时间戳，使用第一个匹配的
                                        if matched_status.is_none() {
                                            matched_status = Some(status);
                                        }
                                    }
                                }
                            }

                            if let Some(delivery_status) = matched_status {
                                // 添加站点接收状态信息
                                let receivers: Vec<serde_json::Value> = delivery_status
                                    .receivers
                                    .iter()
                                    .map(|r| {
                                        json!({
                                            "location": r.location,
                                            "received": r.received,
                                            "received_at": r.received_at,
                                            "status": format!("{:?}", r.status)
                                        })
                                    })
                                    .collect();

                                obj.insert("site_receivers".to_string(), json!(receivers));
                                obj.insert(
                                    "total_receivers".to_string(),
                                    json!(delivery_status.receivers.len()),
                                );
                                obj.insert(
                                    "received_count".to_string(),
                                    json!(
                                        delivery_status
                                            .receivers
                                            .iter()
                                            .filter(|r| r.received)
                                            .count()
                                    ),
                                );
                            }
                        }
                    }

                    record
                })
                .collect()
        }
        Err(e) => {
            eprintln!("❌ 查询 e3d_sync 失败: {:?}", e);
            vec![]
        }
    }
}

/// 从 SurrealDB 读取增量同步历史记录（默认20条）
async fn get_increment_sync_history() -> Vec<serde_json::Value> {
    get_increment_sync_history_paged(20, 0).await
}

/// 获取历史记录总数
async fn get_sync_history_count() -> u64 {
    use aios_core::SUL_DB;

    let query = "SELECT count() as total FROM e3d_sync GROUP ALL";

    match SUL_DB.query(query).await {
        Ok(mut response) => match response.take::<Vec<serde_json::Value>>(0) {
            Ok(records) => {
                if let Some(first) = records.first() {
                    if let Some(total) = first.get("total") {
                        return total.as_u64().unwrap_or(0);
                    }
                }
                0
            }
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// 获取所有部署站点的增量更新状态
pub async fn get_all_incremental_status(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::sync_control_center::SYNC_CONTROL_CENTER;

    // 从同步控制中心获取实际数据
    let center = SYNC_CONTROL_CENTER.read().await;
    let state = &center.state;

    // 从 SurrealDB 读取历史同步记录
    let sync_history = get_increment_sync_history().await;

    // 检查本地增量监测是否启用
    let db_option = aios_core::get_db_option();
    let local_detection_enabled = db_option.sync_live.unwrap_or(false);

    // 如果本地增量监测和远程同步服务都未启动，返回提示信息
    if !local_detection_enabled && !state.is_running {
        return Ok(Json(json!({
            "success": true,
            "sites": [],
            "total_pending": 0,
            "total_synced": 0,
            "last_check": Utc::now(),
            "message": "增量监测服务未启动（sync_live = false）"
        })));
    }

    // 构造站点状态信息
    let site_info = IncrementalUpdateInfo {
        site_id: state
            .current_env
            .clone()
            .unwrap_or_else(|| "default".to_string()),
        site_name: state
            .env_name
            .clone()
            .unwrap_or_else(|| "当前环境".to_string()),
        last_sync_time: state.last_sync_time.map(|st| DateTime::<Utc>::from(st)),
        detection_status: if local_detection_enabled {
            // 本地增量监测模式
            if state.pending_count > 0 {
                UpdateDetectionStatus::ChangesDetected
            } else if !center.running_tasks.is_empty() {
                UpdateDetectionStatus::Syncing
            } else if center.task_queue.is_empty() && center.running_tasks.is_empty() {
                UpdateDetectionStatus::Completed
            } else {
                UpdateDetectionStatus::Scanning
            }
        } else if state.is_paused {
            UpdateDetectionStatus::Idle
        } else if state.pending_count > 0 {
            UpdateDetectionStatus::ChangesDetected
        } else if !center.running_tasks.is_empty() {
            UpdateDetectionStatus::Syncing
        } else {
            UpdateDetectionStatus::Completed
        },
        pending_items: state.pending_count as usize,
        synced_items: state.total_synced as usize,
        changed_files: center
            .task_queue
            .iter()
            .take(20)
            .map(|task| {
                // 从文件名解析数据库编号
                let db_num = task
                    .file_name
                    .as_ref()
                    .and_then(|name| name.strip_suffix(".cba"))
                    .and_then(|name| name.rsplit_once('_'))
                    .and_then(|(_, num)| num.parse::<u32>().ok());

                // 从notes中解析会话信息 (格式: "DB#1112 | 会话: 1162 → 1163 | ...")
                let change_type = if task
                    .notes
                    .as_ref()
                    .map(|n| n.contains("→"))
                    .unwrap_or(false)
                {
                    ChangeType::Modified
                } else {
                    ChangeType::Added
                };

                ChangedFile {
                    path: task
                        .file_name
                        .clone()
                        .unwrap_or_else(|| task.file_path.clone()),
                    change_type,
                    size: task.file_size,
                    modified_time: DateTime::<Utc>::from(task.created_at),
                    db_num,
                }
            })
            .collect(),
        increment_size: center.task_queue.iter().map(|t| t.file_size).sum(),
        estimated_sync_time: if state.sync_rate_mbps > 0.0 {
            let total_mb =
                center.task_queue.iter().map(|t| t.file_size).sum::<u64>() as f64 / 1024.0 / 1024.0;
            (total_mb / state.sync_rate_mbps * 60.0) as u32
        } else {
            0
        },
    };

    let sites = vec![site_info];

    Ok(Json(json!({
        "success": true,
        "sites": sites,
        "total_pending": center.task_queue.len(),
        "total_synced": state.total_synced,
        "last_check": Utc::now(),
        "is_running": local_detection_enabled || state.is_running,
        "watcher_active": local_detection_enabled || state.watcher_active,
        "local_detection_mode": local_detection_enabled,
        "remote_sync_mode": state.is_running,
        // 新增：数据库历史记录和当前任务队列
        "sync_history_from_db": sync_history,
        "current_task_queue": center.task_queue.iter().take(20).map(|task| json!({
            "id": task.id,
            "file_name": task.file_name,
            "file_path": task.file_path,
            "file_size": task.file_size,
            "status": format!("{:?}", task.status),
            "notes": task.notes,
            "created_at": DateTime::<Utc>::from(task.created_at).to_rfc3339(),
            "priority": task.priority,
        })).collect::<Vec<_>>(),
    })))
}

/// 获取特定站点的增量更新详情
pub async fn get_site_incremental_details(
    _state: State<AppState>,
    Path(site_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: 从数据库获取实际数据
    let site_info = IncrementalUpdateInfo {
        site_id: site_id.clone(),
        site_name: "生产环境-主站".to_string(),
        last_sync_time: Some(Utc::now() - chrono::Duration::hours(2)),
        detection_status: UpdateDetectionStatus::ChangesDetected,
        pending_items: 15,
        synced_items: 0,
        changed_files: vec![
            ChangedFile {
                path: "/desi/7999/model.xkt".to_string(),
                change_type: ChangeType::Modified,
                size: 1024 * 1024 * 5,
                modified_time: Utc::now() - chrono::Duration::minutes(30),
                db_num: Some(7999),
            },
            ChangedFile {
                path: "/desi/8001/model.xkt".to_string(),
                change_type: ChangeType::Added,
                size: 1024 * 1024 * 3,
                modified_time: Utc::now() - chrono::Duration::minutes(15),
                db_num: Some(8001),
            },
            ChangedFile {
                path: "/desi/8002/metadata.json".to_string(),
                change_type: ChangeType::Modified,
                size: 1024 * 50,
                modified_time: Utc::now() - chrono::Duration::minutes(5),
                db_num: Some(8002),
            },
        ],
        increment_size: 1024 * 1024 * 8,
        estimated_sync_time: 120,
    };

    Ok(Json(json!({
        "success": true,
        "site": site_info,
        "sync_history": [
            {
                "time": Utc::now() - chrono::Duration::hours(2),
                "items_synced": 45,
                "size": 1024 * 1024 * 120,
                "duration": 300,
                "status": "completed"
            },
            {
                "time": Utc::now() - chrono::Duration::hours(8),
                "items_synced": 23,
                "size": 1024 * 1024 * 56,
                "duration": 180,
                "status": "completed"
            }
        ]
    })))
}

/// 启动增量检测
pub async fn start_incremental_detection(
    _state: State<AppState>,
    Path(site_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::sync_control_center::SYNC_CONTROL_CENTER;

    let center = SYNC_CONTROL_CENTER.read().await;

    if !center.state.is_running {
        return Ok(Json(json!({
            "success": false,
            "message": "增量监测服务未启动，请先启用 sync_live 配置",
        })));
    }

    println!("增量检测正在自动运行中（站点: {}）", site_id);

    Ok(Json(json!({
        "success": true,
        "message": format!("站点 {} 的增量检测正在后台运行", site_id),
        "task_id": format!("detect_{}_{}",site_id, Utc::now().timestamp()),
        "info": "增量监测由 async_watch 自动触发，无需手动启动",
    })))
}

/// 启动增量同步
pub async fn start_incremental_sync(
    _state: State<AppState>,
    Path(site_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: 实际触发增量同步逻辑
    println!("启动站点 {} 的增量同步", site_id);

    Ok(Json(json!({
        "success": true,
        "message": format!("已启动站点 {} 的增量同步", site_id),
        "task_id": format!("sync_{}_{}",site_id, Utc::now().timestamp()),
    })))
}

/// 获取检测任务状态
pub async fn get_detection_task_status(
    _state: State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: 从任务管理器获取实际状态
    Ok(Json(json!({
        "success": true,
        "task_id": task_id,
        "status": "running",
        "progress": 65,
        "scanned_files": 1250,
        "detected_changes": 15,
        "estimated_remaining": 30, // seconds
    })))
}

/// 取消检测或同步任务
pub async fn cancel_task(
    _state: State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: 实际取消任务逻辑
    println!("取消任务: {}", task_id);

    Ok(Json(json!({
        "success": true,
        "message": format!("任务 {} 已取消", task_id),
    })))
}

/// 获取增量更新配置
pub async fn get_incremental_config(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(json!({
        "success": true,
        "config": {
            "auto_detect": true,
            "detect_interval": 300, // 5 minutes
            "auto_sync": false,
            "sync_batch_size": 10,
            "max_concurrent_syncs": 3,
            "retry_on_failure": true,
            "max_retries": 3,
            "notification_enabled": true,
            "notification_threshold": 50, // MB
        }
    })))
}

/// 更新增量更新配置
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub auto_detect: Option<bool>,
    pub detect_interval: Option<u32>,
    pub auto_sync: Option<bool>,
    pub sync_batch_size: Option<usize>,
    pub notification_enabled: Option<bool>,
}

pub async fn update_incremental_config(
    _state: State<AppState>,
    Json(config): Json<UpdateConfigRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: 保存配置到数据库
    println!("更新增量配置: {:?}", config);

    Ok(Json(json!({
        "success": true,
        "message": "配置已更新",
    })))
}

/// 获取增量检测日志
pub async fn get_increment_logs(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::sync_control_center::SYNC_CONTROL_CENTER;
    use chrono::DateTime;

    let center = SYNC_CONTROL_CENTER.read().await;
    let logs = center.get_logs(200); // 获取最近200条日志

    let log_entries: Vec<serde_json::Value> = logs
        .iter()
        .map(|log| {
            json!({
                "timestamp": DateTime::<Utc>::from(log.timestamp).to_rfc3339(),
                "level": log.level,
                "message": log.message,
            })
        })
        .collect();

    Ok(Json(json!({
        "success": true,
        "logs": log_entries,
    })))
}

/// 获取分页的同步历史记录
pub async fn get_sync_history_paged(
    _state: State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use std::collections::HashMap;

    // 解析分页参数
    let page = params
        .get("page")
        .and_then(|p| p.parse::<usize>().ok())
        .unwrap_or(1);

    let page_size = params
        .get("page_size")
        .and_then(|ps| ps.parse::<usize>().ok())
        .unwrap_or(20)
        .min(100); // 最大100条

    let offset = (page - 1) * page_size;

    // 获取历史记录
    let records = get_increment_sync_history_paged(page_size, offset).await;

    // 获取总数
    let total = get_sync_history_count().await;
    let total_pages = ((total as f64) / (page_size as f64)).ceil() as u64;

    Ok(Json(json!({
        "success": true,
        "data": records,
        "pagination": {
            "page": page,
            "page_size": page_size,
            "total": total,
            "total_pages": total_pages,
        }
    })))
}

/// 列出归档目录下的CBA文件
pub async fn list_cba_files(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use aios_core::{SUL_DB, rs_surreal::query_ext::SurrealQueryExt};

    // 使用智能路径解析函数，确保无论工作目录在哪里都能找到目录
    let archives_dir = resolve_archives_dir();
    let mut files = Vec::new();

    // 确保目录存在
    if let Ok(entries) = std::fs::read_dir(&archives_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.ends_with(".cba") {
                        let modified: DateTime<Utc> = metadata
                            .modified()
                            .unwrap_or(std::time::SystemTime::now())
                            .into();
                        let size = metadata.len();

                        // 从文件名提取不带扩展名的名称（用于查询 db_file_info）
                        let file_name_without_ext =
                            file_name.strip_suffix(".cba").unwrap_or(&file_name);

                        // 从 db_file_info 表查询 sesno 和 dbnum（使用 query_take 扩展方法）
                        let (sesno, dbnum) = match SUL_DB
                            .query_take::<Option<serde_json::Value>>(
                                &format!(
                                    "SELECT sesno, dbnum FROM db_file_info:{} LIMIT 1",
                                    file_name_without_ext
                                ),
                                0,
                            )
                            .await
                        {
                            Ok(Some(record)) => {
                                let sesno = record
                                    .get("sesno")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32);
                                let dbnum = record
                                    .get("dbnum")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32);
                                (sesno, dbnum)
                            }
                            Ok(None) => (None, None),
                            Err(_) => (None, None),
                        };

                        files.push(json!({
                            "name": file_name,
                            "size": size,
                            "modified": modified,
                            "path": format!("/assets/archives/{}", file_name),
                            "sesno": sesno,
                            "dbnum": dbnum,
                        }));
                    }
                }
            }
        }
    }

    // 按修改时间倒序排序
    files.sort_by(|a, b| {
        let time_a = a["modified"].as_str().unwrap_or("");
        let time_b = b["modified"].as_str().unwrap_or("");
        time_b.cmp(time_a)
    });

    Ok(Json(json!({
        "success": true,
        "files": files,
        "count": files.len()
    })))
}

/// 处理归档目录根路径请求（返回目录列表）
pub async fn serve_archives_root() -> Result<axum::response::Html<String>, StatusCode> {
    list_archives_page_internal().await
}

/// 处理归档目录文件路径请求（返回文件内容）
pub async fn serve_archives_file(
    axum::extract::Path(path): axum::extract::Path<String>,
    req: axum::http::Request<axum::body::Body>,
) -> Result<axum::response::Response, StatusCode> {
    use axum::body::Body;
    use axum::http::StatusCode as HttpStatusCode;
    use tower_http::services::ServeDir;
    use tower::ServiceExt;

    let archives_dir = resolve_archives_dir();
    let service = ServeDir::new(archives_dir);
    
    // 构建新的请求路径
    let path = path.trim_start_matches('/');
    let new_path = format!("/{}", path);
    let new_uri = axum::http::Uri::builder()
        .path_and_query(new_path)
        .build()
        .map_err(|_| HttpStatusCode::BAD_REQUEST)?;
    
    let mut new_req = req;
    *new_req.uri_mut() = new_uri;
    
    service
        .oneshot(new_req)
        .await
        .map_err(|_| HttpStatusCode::INTERNAL_SERVER_ERROR)
        .map(|resp| resp.map(Body::new))
}

/// 显示归档目录的 HTML 列表页面（内部函数）
async fn list_archives_page_internal() -> Result<axum::response::Html<String>, StatusCode> {

    // 使用智能路径解析函数
    let archives_dir = resolve_archives_dir();
    let mut files = Vec::new();

    // 读取目录内容
    if let Ok(entries) = std::fs::read_dir(&archives_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.ends_with(".cba") {
                        let modified: DateTime<Utc> = metadata
                            .modified()
                            .unwrap_or(std::time::SystemTime::now())
                            .into();
                        let size = metadata.len();
                        files.push((file_name, size, modified));
                    }
                }
            }
        }
    }

    // 按修改时间倒序排序
    files.sort_by(|a, b| b.2.cmp(&a.2));

    // 生成 HTML
    let mut html = String::from(r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>归档文件列表 - CBA Archives</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            padding: 20px;
        }
        h1 {
            margin-top: 0;
            color: #333;
            border-bottom: 2px solid #007bff;
            padding-bottom: 10px;
        }
        .file-count {
            color: #666;
            margin-bottom: 20px;
        }
        table {
            width: 100%;
            border-collapse: collapse;
        }
        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }
        th {
            background-color: #f8f9fa;
            font-weight: 600;
            color: #495057;
        }
        tr:hover {
            background-color: #f8f9fa;
        }
        .file-name {
            font-family: 'Courier New', monospace;
        }
        .file-size {
            color: #666;
            white-space: nowrap;
        }
        .file-date {
            color: #666;
            white-space: nowrap;
        }
        a {
            color: #007bff;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
        .no-files {
            text-align: center;
            padding: 40px;
            color: #999;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>📦 CBA 归档文件列表</h1>
        <div class="file-count">共 <strong>"#);
    html.push_str(&format!("{}", files.len()));
    html.push_str(r#"</strong> 个文件</div>
        <table>
            <thead>
                <tr>
                    <th>文件名</th>
                    <th>大小</th>
                    <th>修改时间</th>
                </tr>
            </thead>
            <tbody>
"#);

    if files.is_empty() {
        html.push_str(r#"                <tr><td colspan="3" class="no-files">暂无文件</td></tr>"#);
    } else {
        for (file_name, size, modified) in files {
            let size_str = if size < 1024 {
                format!("{} B", size)
            } else if size < 1024 * 1024 {
                format!("{:.2} KB", size as f64 / 1024.0)
            } else if size < 1024 * 1024 * 1024 {
                format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
            } else {
                format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
            };

            let file_url = format!("/assets/archives/{}", file_name);
            let modified_str = modified.format("%Y-%m-%d %H:%M:%S").to_string();

            html.push_str(&format!(
                r#"                <tr>
                    <td class="file-name"><a href="{}">{}</a></td>
                    <td class="file-size">{}</td>
                    <td class="file-date">{}</td>
                </tr>
"#,
                file_url, file_name, size_str, modified_str
            ));
        }
    }

    html.push_str(r#"            </tbody>
        </table>
    </div>
</body>
</html>"#);

    Ok(axum::response::Html(html))
}

/// 获取增量更新统计数据（用于 Dashboard 可视化）
pub async fn get_incremental_stats(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::sync_control_center::SYNC_CONTROL_CENTER;
    use aios_core::SUL_DB;

    // 1. 获取今天的更新记录数
    let today_count_query = r#"
        SELECT count() as total FROM e3d_sync
        WHERE timestamp > time::now() - 1d
        GROUP ALL
    "#;

    let today_count = match SUL_DB.query(today_count_query).await {
        Ok(mut response) => match response.take::<Vec<serde_json::Value>>(0) {
            Ok(records) => records
                .first()
                .and_then(|r| r.get("total"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            Err(_) => 0,
        },
        Err(_) => 0,
    };

    // 2. 获取总更新记录数
    let total_count = get_sync_history_count().await;

    // 3. 获取本周的更新记录数
    let week_count_query = r#"
        SELECT count() as total FROM e3d_sync
        WHERE timestamp > time::now() - 7d
        GROUP ALL
    "#;

    let week_count = match SUL_DB.query(week_count_query).await {
        Ok(mut response) => match response.take::<Vec<serde_json::Value>>(0) {
            Ok(records) => records
                .first()
                .and_then(|r| r.get("total"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            Err(_) => 0,
        },
        Err(_) => 0,
    };

    // 4. 获取最近一次同步时间
    let last_sync_query = r#"
        SELECT timestamp FROM e3d_sync
        ORDER BY timestamp DESC
        LIMIT 1
    "#;

    let last_sync_time = match SUL_DB.query(last_sync_query).await {
        Ok(mut response) => match response.take::<Vec<serde_json::Value>>(0) {
            Ok(records) => records
                .first()
                .and_then(|r| r.get("timestamp"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            Err(_) => None,
        },
        Err(_) => None,
    };

    // 5. 从 SyncControlCenter 获取任务队列信息
    let center = SYNC_CONTROL_CENTER.read().await;
    let queued_updates = center.task_queue.len();
    let running_tasks = center.running_tasks.len();
    let state = &center.state;
    let pending_count = state.pending_count;

    Ok(Json(json!({
        "success": true,
        "stats": {
            // 历史统计
            "total_synced": total_count,
            "week_synced": week_count,
            "today_synced": today_count,
            "last_sync_time": last_sync_time,

            // 实时状态
            "queued_updates": queued_updates,
            "running_tasks": running_tasks,
            "pending_items": pending_count,

            // 服务状态
            "is_running": state.is_running,
            "is_paused": state.is_paused,
        },
        "timestamp": Utc::now()
    })))
}
