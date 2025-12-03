use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

use crate::web_server::{AppState, remote_sync_handlers, sync_control_center::*};

// ========= 控制接口 =========

/// 测试触发文件下载（仅用于测试）
pub async fn trigger_file_download(
    _state: State<AppState>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::data_interface::tidb_manager::AiosDBManager;
    use crate::mqtt_service::SyncE3dFileMsg;

    let file_names = request["file_names"]
        .as_array()
        .ok_or(StatusCode::BAD_REQUEST)?
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let file_server_host = request["file_server_host"]
        .as_str()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let sync_e3d = SyncE3dFileMsg {
        file_names,
        file_hashes: vec![],
        file_server_host: file_server_host.to_string(),
        location: "test".to_string(),
        timestamp: aios_core::Datetime::default(),
        session_range: None,
        total_added: None,
        total_modified: None,
        total_deleted: None,
        is_full_sync: None,
        db_num: None,
    };

    // 创建一个临时的 watcher（实际使用时应该从全局状态获取）
    let watcher = pdms_io::watch::PdmsWatcher::new(Vec::<std::path::PathBuf>::new());

    match AiosDBManager::exec_delta_clone_remotes(&watcher, sync_e3d).await {
        Ok(_) => Ok(Json(json!({
            "status": "success",
            "message": "文件下载已触发"
        }))),
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("下载失败: {}", e)
        }))),
    }
}
pub async fn start_sync_service(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut center = SYNC_CONTROL_CENTER.write().await;

    // 使用 DbOption.location 作为站点标识
    match center.start().await {
        Ok(_) => {
            // 启动监控任务
            tokio::spawn(start_monitoring());

            Ok(Json(json!({
                "status": "success",
                "message": "同步服务已启动",
                "location": crate::web_server::sync_control_center::get_location(),
                "state": center.get_state_snapshot()
            })))
        }
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("启动失败: {}", e)
        }))),
    }
}

/// 停止同步服务
pub async fn stop_sync_service(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut center = SYNC_CONTROL_CENTER.write().await;

    match center.stop().await {
        Ok(_) => Ok(Json(json!({
            "status": "success",
            "message": "同步服务已停止",
            "state": center.get_state_snapshot()
        }))),
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("停止失败: {}", e)
        }))),
    }
}

/// 重启同步服务
pub async fn restart_sync_service(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::sync_control_center::get_location;
    
    let mut center = SYNC_CONTROL_CENTER.write().await;

    let location = get_location();
    if location.is_empty() {
        return Ok(Json(json!({
            "status": "error",
            "message": "未配置站点标识 (location)"
        })));
    }

    // 先停止
    let _ = center.stop().await;

    // 等待一下
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 重新启动
    match center.start().await {
        Ok(_) => Ok(Json(json!({
            "status": "success",
            "message": "同步服务已重启",
            "location": location,
            "state": center.get_state_snapshot()
        }))),
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("重启失败: {}", e)
        }))),
    }
}

/// 暂停同步
pub async fn pause_sync_service(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut center = SYNC_CONTROL_CENTER.write().await;

    match center.pause() {
        Ok(_) => Ok(Json(json!({
            "status": "success",
            "message": "同步已暂停",
            "state": center.get_state_snapshot()
        }))),
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("暂停失败: {}", e)
        }))),
    }
}

/// 恢复同步
pub async fn resume_sync_service(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut center = SYNC_CONTROL_CENTER.write().await;

    match center.resume() {
        Ok(_) => Ok(Json(json!({
            "status": "success",
            "message": "同步已恢复",
            "state": center.get_state_snapshot()
        }))),
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("恢复失败: {}", e)
        }))),
    }
}

// ========= 监控接口 =========

/// 获取同步状态
pub async fn get_sync_status(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let center = SYNC_CONTROL_CENTER.read().await;

    Ok(Json(json!({
        "status": "success",
        "state": center.get_state_snapshot(),
        "config": center.get_config(),
        "mqtt_server": center.mqtt_server.clone(),
        "queue_length": center.task_queue.len(),
        "running_tasks": center.running_tasks.len(),
        "history_count": center.history.len()
    })))
}

/// 获取最新事件（轮询方式）
/// 注：SSE 功能暂时简化为轮询实现
pub async fn sync_events_stream(
    _state: State<AppState>,
    Query(params): Query<EventQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 收集最近的事件
    let mut events = Vec::new();
    let mut rx = SYNC_EVENT_TX.subscribe();

    // 非阻塞地获取所有可用事件
    while let Ok(event) = rx.try_recv() {
        events.push(event);
        if events.len() >= 10 {
            // 最多返回10个事件
            break;
        }
    }

    Ok(Json(json!({
        "status": "success",
        "events": events,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    })))
}

/// 获取性能指标
pub async fn get_sync_metrics(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let center = SYNC_CONTROL_CENTER.read().await;

    let (cpu_usage, memory_usage) = sample_system_metrics().await;
    let (completed_files, completed_bytes, completed_records, failed_files_db) =
        collect_sync_totals();

    Ok(Json(json!({
        "status": "success",
        "metrics": {
            "sync_rate_mbps": center.state.sync_rate_mbps,
            "avg_sync_time_ms": center.state.avg_sync_time_ms,
            "total_synced": center.state.total_synced,
            "total_failed": center.state.total_failed,
            "success_rate": if center.state.total_synced + center.state.total_failed > 0 {
                (center.state.total_synced as f64) /
                ((center.state.total_synced + center.state.total_failed) as f64) * 100.0
            } else {
                0.0
            },
            "cpu_usage": cpu_usage,
            "memory_usage": memory_usage,
            "uptime_seconds": center.state.uptime_seconds,
            "completed_files_total": completed_files,
            "completed_bytes_total": completed_bytes,
            "completed_records_total": completed_records,
            "failed_files_total": failed_files_db
        }
    })))
}

/// 获取队列状态
pub async fn get_sync_queue(
    _state: State<AppState>,
    Query(params): Query<QueueQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let center = SYNC_CONTROL_CENTER.read().await;

    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let queue_items: Vec<_> = center
        .task_queue
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(Json(json!({
        "status": "success",
        "queue": queue_items,
        "total": center.task_queue.len(),
        "pending": center.state.pending_count,
        "running": center.running_tasks.len()
    })))
}

// ========= 配置接口 =========

/// 获取同步配置
pub async fn get_sync_config(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let center = SYNC_CONTROL_CENTER.read().await;

    Ok(Json(json!({
        "status": "success",
        "config": center.get_config()
    })))
}

/// 更新同步配置
pub async fn update_sync_config(
    _state: State<AppState>,
    Json(config): Json<SyncConfig>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut center = SYNC_CONTROL_CENTER.write().await;
    center.update_config(config);

    Ok(Json(json!({
        "status": "success",
        "message": "配置已更新",
        "config": center.get_config()
    })))
}

/// 测试连接
pub async fn test_sync_connection(
    _state: State<AppState>,
    Json(request): Json<TestConnectionRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use tokio::time::timeout;

    // 测试 MQTT 连接
    let mqtt_result = if let (Some(host), Some(port)) = (request.mqtt_host, request.mqtt_port) {
        let addr = format!("{}:{}", host, port);
        match timeout(
            Duration::from_secs(3),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        {
            Ok(Ok(_)) => json!({
                "status": "connected",
                "message": "MQTT连接成功"
            }),
            Ok(Err(e)) => json!({
                "status": "failed",
                "message": format!("MQTT连接失败: {}", e)
            }),
            Err(_) => json!({
                "status": "timeout",
                "message": "MQTT连接超时"
            }),
        }
    } else {
        json!({
            "status": "skipped",
            "message": "未提供MQTT配置"
        })
    };

    // 测试文件服务器连接
    let file_server_result = if let Some(url) = request.file_server_host {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        match client.get(&url).send().await {
            Ok(resp) => json!({
                "status": "connected",
                "message": format!("文件服务器连接成功，状态码: {}", resp.status())
            }),
            Err(e) => json!({
                "status": "failed",
                "message": format!("文件服务器连接失败: {}", e)
            }),
        }
    } else {
        json!({
            "status": "skipped",
            "message": "未提供文件服务器配置"
        })
    };

    Ok(Json(json!({
        "status": "success",
        "mqtt": mqtt_result,
        "file_server": file_server_result
    })))
}

// ========= 任务管理接口 =========

/// 添加同步任务
pub async fn add_sync_task(
    _state: State<AppState>,
    Json(request): Json<AddTaskRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut center = SYNC_CONTROL_CENTER.write().await;

    let task_id = center.add_task(NewSyncTaskParams {
        file_path: request.file_path,
        file_size: request.file_size,
        priority: request.priority.unwrap_or(5),
        file_name: request.file_name,
        file_hash: request.file_hash,
        record_count: request.record_count,
        env_id: request.env_id,
        source_env: request.source_env,
        target_site: request.target_site,
        direction: request.direction,
        notes: request.notes,
    });

    Ok(Json(json!({
        "status": "success",
        "message": "任务已添加到队列",
        "task_id": task_id,
        "queue_size": center.state.queue_size
    })))
}

/// 取消任务
pub async fn cancel_sync_task(
    _state: State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut center = SYNC_CONTROL_CENTER.write().await;

    // 从待处理队列移除
    let cancelled_in_queue = center.cancel_pending_task(&task_id, "用户取消");

    // 从运行中任务移除
    if center.running_tasks.contains_key(&task_id) {
        center.complete_task(&task_id, false, Some("用户取消".to_string()));
    } else if !cancelled_in_queue {
        // 未找到任务
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(json!({
        "status": "success",
        "message": "任务已取消"
    })))
}

/// 清空队列
pub async fn clear_sync_queue(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut center = SYNC_CONTROL_CENTER.write().await;
    let count = center.clear_queue("队列清空");

    Ok(Json(json!({
        "status": "success",
        "message": format!("已清空 {} 个任务", count)
    })))
}

/// 获取任务历史
pub async fn get_sync_history(
    _state: State<AppState>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let center = SYNC_CONTROL_CENTER.read().await;

    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let history: Vec<_> = center
        .history
        .iter()
        .rev() // 最新的在前
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(Json(json!({
        "status": "success",
        "history": history,
        "total": center.history.len()
    })))
}

// ========= MQTT 服务器管理 =========

/// 启动 MQTT 服务器
pub async fn start_mqtt_server_api(
    _state: State<AppState>,
    Json(request): Json<StartMqttRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let port = request.port.unwrap_or(1883);

    match start_mqtt_server(port).await {
        Ok(_) => Ok(Json(json!({
            "status": "success",
            "message": format!("MQTT服务器已启动在端口 {}", port),
            "port": port
        }))),
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("启动MQTT服务器失败: {}", e)
        }))),
    }
}

/// 停止 MQTT 服务器
pub async fn stop_mqtt_server_api(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match stop_mqtt_server().await {
        Ok(_) => Ok(Json(json!({
            "status": "success",
            "message": "MQTT服务器已停止"
        }))),
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("停止MQTT服务器失败: {}", e)
        }))),
    }
}

/// 获取 MQTT Broker 日志
pub async fn get_mqtt_broker_logs_api(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::sync_control_center::get_mqtt_broker_logs;

    let logs = get_mqtt_broker_logs().await;
    Ok(Json(json!({
        "status": "success",
        "logs": logs,
        "count": logs.len()
    })))
}

#[derive(Debug, Deserialize, Default)]
pub struct StartSubscriptionRequest {
    /// 要订阅的主节点位置（可选）
    #[serde(default)]
    pub master_location: Option<String>,
    /// 要订阅的主节点MQTT主机（可选）
    #[serde(default)]
    pub master_mqtt_host: Option<String>,
    /// 要订阅的主节点MQTT端口（可选）
    #[serde(default)]
    pub master_mqtt_port: Option<u16>,
    /// 环境ID（可选，默认使用 "default"）
    #[serde(default)]
    pub env_id: Option<String>,
}

/// 启动 MQTT 订阅客户端
pub async fn start_mqtt_subscription_api(
    _state: State<AppState>,
    request: Option<Json<StartSubscriptionRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // 允许空的请求体（主节点启动订阅时不需要参数）
    let request = request.map(|Json(req)| req).unwrap_or_default();
    use crate::web_server::remote_runtime;
    use aios_core::get_db_option;

    // 检查是否已经启动
    let guard = remote_runtime::REMOTE_RUNTIME.read().await;
    if guard.is_some() {
        return Ok(Json(json!({
            "status": "error",
            "message": "MQTT 订阅已经在运行中"
        })));
    }
    drop(guard);

    let db_option = get_db_option();
    let location = db_option.location.clone();
    let is_master_node = check_is_master_node(&location);
    let env_id = request.env_id.unwrap_or_else(|| "default".to_string());

    // 如果是从节点，需要检查或保存主节点配置
    if !is_master_node {
        use crate::web_server::remote_sync_handlers;
        let conn = match remote_sync_handlers::open_sqlite() {
            Ok(c) => c,
            Err(e) => {
                return Ok(Json(json!({
                    "status": "error",
                    "message": format!("打开数据库失败: {}", e)
                })));
            }
        };

        if let (Some(host), Some(port)) = (request.master_mqtt_host.clone(), request.master_mqtt_port) {
            // 请求中指定了主节点配置，保存到数据库
            // 尝试从 available_masters 中查找匹配的主节点位置
            let master_location_to_save = if let Some(master_loc) = request.master_location.clone() {
                // 验证 master_location 不是 'unknown' 或空
                if master_loc.is_empty() || master_loc == "unknown" {
                    None
                } else {
                    Some(master_loc)
                }
            } else {
                // 如果没有提供 master_location，尝试从 available_masters 中查找匹配的
                let available_masters = get_available_master_nodes();
                available_masters.iter()
                    .find(|m| {
                        m["mqtt_host"].as_str() == Some(&host) &&
                        m["mqtt_port"].as_u64() == Some(port as u64)
                    })
                    .and_then(|m| {
                        let loc = m["location"].as_str()?;
                        // 确保 location 不是 'unknown' 或空
                        if loc.is_empty() || loc == "unknown" {
                            None
                        } else {
                            Some(loc.to_string())
                        }
                    })
            };
            
            if let Some(master_loc) = master_location_to_save {
                // 保存主节点位置和 MQTT 配置
                let _ = conn.execute(
                    "UPDATE remote_sync_sites SET master_location = ?1, master_mqtt_host = ?2, master_mqtt_port = ?3, updated_at = datetime('now') WHERE location = ?4",
                    rusqlite::params![master_loc, host, port as i64, location.clone()],
                );
                log::info!("从节点 {} 已保存主节点配置: location={}, host={}, port={}", location, master_loc, host, port);
            } else {
                // 如果找不到有效的 master_location，拒绝保存配置
                return Ok(Json(json!({
                    "status": "error",
                    "message": format!("无法保存主节点配置：未找到有效的主节点位置。请确保主节点已正确配置 location 字段。host={}, port={}", host, port)
                })));
            }
        } else if let Some(master_loc) = request.master_location.clone() {
            // 根据主节点位置查找MQTT配置
            let available_masters = get_available_master_nodes();
            if let Some(master) = available_masters.iter().find(|m| {
                m["location"].as_str() == Some(&master_loc)
            }) {
                let host = master["mqtt_host"].as_str().unwrap_or("");
                let port = master["mqtt_port"].as_u64().unwrap_or(1883) as u16;
                
                let _ = conn.execute(
                    "UPDATE remote_sync_sites SET master_location = ?1, master_mqtt_host = ?2, master_mqtt_port = ?3, updated_at = datetime('now') WHERE location = ?4",
                    rusqlite::params![master_loc, host, port as i64, location.clone()],
                );
            } else {
                return Ok(Json(json!({
                    "status": "error",
                    "message": format!("未找到位置为 '{}' 的主节点配置", master_loc)
                })));
            }
        } else {
            // 尝试从数据库中读取已保存的主节点配置
            let has_master_config: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM remote_sync_sites WHERE location = ?1 AND master_mqtt_host IS NOT NULL AND master_mqtt_port IS NOT NULL",
                rusqlite::params![location.clone()],
                |row| row.get(0),
            ).unwrap_or(false);

            if !has_master_config {
                return Ok(Json(json!({
                    "status": "error",
                    "message": "从节点需要指定要订阅的主节点（master_location 或 master_mqtt_host/master_mqtt_port）"
                })));
            }
            // 已有配置，继续启动
        }
    }

    // 启动运行时
    match remote_runtime::start_runtime(env_id).await {
        Ok(_) => {
            #[cfg(feature = "web_server")]
            {
                use crate::web_server::mqtt_monitor_handlers;

                let node_name = format!("{}-{}", location, db_option.project_code);

                // 注册节点到监控系统
                mqtt_monitor_handlers::update_node_heartbeat(
                    location.clone(),
                    node_name,
                    vec!["Sync/E3d".to_string()],
                )
                .await;
            }

            let message = if is_master_node {
                "MQTT 订阅已启动（主节点模式）"
            } else {
                "MQTT 订阅已启动（从节点模式，已连接到主节点）"
            };

            // 发送状态变化事件
            let status_data = get_mqtt_subscription_status_internal().await;
            if let Ok(status_json) = status_data {
                use crate::web_server::sync_control_center::SYNC_EVENT_TX;
                use crate::web_server::sse_handlers::SyncEvent;
                let _ = SYNC_EVENT_TX.send(SyncEvent::MqttSubscriptionStatusChanged {
                    is_running: true,
                    is_server_running: status_json["is_server_running"].as_bool().unwrap_or(false),
                    location: location.clone(),
                    is_master_node,
                    node_role: if is_master_node { "master".to_string() } else { "client".to_string() },
                    connection_status: status_json["connection_status"].clone(),
                    available_masters: status_json["available_masters"].as_array().cloned().unwrap_or_default(),
                    timestamp: SyncEvent::now(),
                });
            }

            Ok(Json(json!({
                "status": "success",
                "message": message
            })))
        }
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("启动 MQTT 订阅失败: {}", e)
        }))),
    }
}

/// 停止 MQTT 订阅客户端
pub async fn stop_mqtt_subscription_api(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::remote_runtime;
    use aios_core::get_db_option;

    remote_runtime::stop_runtime().await;

    // 发送状态变化事件
    let db_option = get_db_option();
    let location = db_option.location.clone();
    let is_master_node = check_is_master_node(&location);
    let status_data = get_mqtt_subscription_status_internal().await;
    if let Ok(status_json) = status_data {
        use crate::web_server::sync_control_center::SYNC_EVENT_TX;
        use crate::web_server::sse_handlers::SyncEvent;
        let _ = SYNC_EVENT_TX.send(SyncEvent::MqttSubscriptionStatusChanged {
            is_running: false,
            is_server_running: status_json["is_server_running"].as_bool().unwrap_or(false),
            location: location.clone(),
            is_master_node,
            node_role: if is_master_node { "master".to_string() } else { "client".to_string() },
            connection_status: status_json["connection_status"].clone(),
            available_masters: status_json["available_masters"].as_array().cloned().unwrap_or_default(),
            timestamp: SyncEvent::now(),
        });
    }

    Ok(Json(json!({
        "status": "success",
        "message": "MQTT 订阅已停止"
    })))
}

/// 清除从节点的主节点配置（取消订阅主节点）
/// POST /api/mqtt/subscription/clear-master-config
pub async fn clear_master_config_api(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::remote_runtime;
    use crate::web_server::remote_sync_handlers;
    use aios_core::get_db_option;

    let db_option = get_db_option();
    let location = db_option.location.clone();

    // 检查是否为主节点
    if check_is_master_node(&location) {
        return Ok(Json(json!({
            "status": "error",
            "message": "主节点不需要清除主节点配置"
        })));
    }

    // 先停止正在运行的订阅
    remote_runtime::stop_runtime().await;

    // 清除数据库中的主节点配置
    let conn = match remote_sync_handlers::open_sqlite() {
        Ok(c) => c,
        Err(e) => {
            return Ok(Json(json!({
                "status": "error",
                "message": format!("打开数据库失败: {}", e)
            })));
        }
    };

    // 将 master_location、master_mqtt_host 和 master_mqtt_port 设置为 NULL
    let updated = conn.execute(
        "UPDATE remote_sync_sites SET master_location = NULL, master_mqtt_host = NULL, master_mqtt_port = NULL, updated_at = datetime('now') WHERE location = ?1",
        rusqlite::params![location.clone()],
    );

    match updated {
        Ok(rows) => {
            log::info!("从节点 {} 已清除主节点配置 (影响 {} 条记录)", location, rows);
            
            // 发送状态变化事件
            let status_data = get_mqtt_subscription_status_internal().await;
            if let Ok(status_json) = status_data {
                use crate::web_server::sync_control_center::SYNC_EVENT_TX;
                use crate::web_server::sse_handlers::SyncEvent;
                let _ = SYNC_EVENT_TX.send(SyncEvent::MqttSubscriptionStatusChanged {
                    is_running: false,
                    is_server_running: status_json["is_server_running"].as_bool().unwrap_or(false),
                    location: location.clone(),
                    is_master_node: false,
                    node_role: "client".to_string(),
                    connection_status: json!({
                        "connected": false,
                        "master_location": null,
                        "master_host": null,
                        "master_port": null,
                        "diagnostic_message": "主节点配置已清除，请重新选择主节点"
                    }),
                    available_masters: status_json["available_masters"].as_array().cloned().unwrap_or_default(),
                    timestamp: SyncEvent::now(),
                });
            }

            Ok(Json(json!({
                "status": "success",
                "message": "主节点配置已清除，订阅已停止。请重新选择主节点并启动订阅。"
            })))
        }
        Err(e) => {
            Ok(Json(json!({
                "status": "error",
                "message": format!("清除主节点配置失败: {}", e)
            })))
        }
    }
}

/// 清除从节点的主节点配置（内部实现，可被其他模块调用）
pub async fn clear_master_config_internal() -> anyhow::Result<()> {
    use crate::web_server::remote_runtime;
    use crate::web_server::remote_sync_handlers;
    use aios_core::get_db_option;

    let db_option = get_db_option();
    let location = db_option.location.clone();

    // 先停止正在运行的订阅
    remote_runtime::stop_runtime().await;

    // 清除数据库中的主节点配置
    let conn = remote_sync_handlers::open_sqlite()
        .map_err(|e| anyhow::anyhow!("打开数据库失败: {}", e))?;

    // 将 master_location、master_mqtt_host 和 master_mqtt_port 设置为 NULL
    conn.execute(
        "UPDATE remote_sync_sites SET master_location = NULL, master_mqtt_host = NULL, master_mqtt_port = NULL, updated_at = datetime('now') WHERE location = ?1",
        rusqlite::params![location.clone()],
    ).map_err(|e| anyhow::anyhow!("清除主节点配置失败: {}", e))?;

    log::info!("从节点 {} 已清除主节点配置", location);
    
    Ok(())
}

/// 获取 MQTT 订阅状态（内部实现，可复用）
async fn get_mqtt_subscription_status_internal() -> Result<serde_json::Value, StatusCode> {
    use crate::web_server::remote_runtime;
    use aios_core::get_db_option;

    let guard = remote_runtime::REMOTE_RUNTIME.read().await;
    let is_subscription_running = guard.is_some();

    let db_option = get_db_option();
    let location = db_option.location.clone();

    // 检查是否为主节点（从 SQLite 读取）
    let is_master_node = check_is_master_node(&location);

    // 检查 MQTT server 状态（自动验证进程真实状态）
    let center = SYNC_CONTROL_CENTER.read().await;
    let mut is_server_running = center.mqtt_server.is_some();
    let mut mqtt_server_port: Option<u16> = center.mqtt_server.as_ref().map(|s| s.port);
    
    // 如果是主节点，验证进程真实状态
    if is_master_node {
        use crate::web_server::sync_control_center::check_mqtt_process_status;
        let (process_running, pid, exit_info) = check_mqtt_process_status().await;
        
        // 如果进程真实状态与内存状态不一致，更新状态
        if process_running != is_server_running {
            drop(center);
            let mut center = SYNC_CONTROL_CENTER.write().await;
            if process_running {
                // 进程在运行但内存状态显示未运行，更新为运行中
                // 尝试从配置文件读取端口，如果失败则使用默认端口1883
                let port = {
                    use crate::web_server::sync_control_center::resolve_rumqttd_config_path;
                    if let Ok(config_path) = resolve_rumqttd_config_path() {
                        // 尝试读取配置文件中的端口
                        if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                            // 简单的端口解析（rumqttd配置文件格式）
                            if let Some(port_str) = config_content
                                .lines()
                                .find(|l| l.trim().starts_with("listener") && l.contains("port"))
                                .and_then(|l| {
                                    l.split_whitespace()
                                        .find(|s| s.parse::<u16>().is_ok())
                                        .and_then(|s| s.parse::<u16>().ok())
                                })
                            {
                                port_str
                            } else {
                                1883 // 默认端口
                            }
                        } else {
                            1883 // 默认端口
                        }
                    } else {
                        1883 // 默认端口
                    }
                };
                
                center.mqtt_server = Some(crate::web_server::sync_control_center::MqttServerState {
                    is_running: true,
                    port,
                    client_count: 0,
                    message_count: 0,
                    started_at: Some(std::time::SystemTime::now()),
                });
                is_server_running = true;
                mqtt_server_port = Some(port);
                log::info!("检测到 MQTT Broker 进程正在运行（PID: {:?}，端口: {}），已自动更新状态", pid, port);
            } else {
                // 进程已退出但内存状态显示运行中，更新为未运行
                center.mqtt_server = None;
                is_server_running = false;
                mqtt_server_port = None;
                log::warn!("检测到 MQTT Broker 进程已退出（{:?}），但内存状态显示运行中，已自动更新状态", exit_info);
            }
        } else if is_server_running {
            // 状态一致，确保端口信息已保存
            mqtt_server_port = center.mqtt_server.as_ref().map(|s| s.port);
        }
    }

    // 获取所有可用的主节点列表
    let available_masters = get_available_master_nodes();
    
    // 获取主节点信息和连接状态（从节点需要）
    let mut master_info = json!({});
    let mut connection_status = json!({
        "connected": false,
        "master_location": null,
        "master_host": null,
        "master_port": null
    });
    let mut selected_master = None::<String>;

    if !is_master_node {
        // 从节点：查找当前选择的主节点（从站点配置中获取）
        // 优先从数据库的 master_location 字段获取主节点位置
        let master_config = get_master_config_from_db(&location);
        
        if let Some((master_location_db, master_host, master_port)) = master_config {
            // 检查主节点是否在线（通过 MQTT 节点监控）
            use crate::web_server::mqtt_monitor_handlers;
            let nodes = mqtt_monitor_handlers::MQTT_NODES.read().await;
            
            // 使用数据库中的 master_location
            let master_location = if master_location_db.is_empty() || master_location_db == "unknown" {
                // 回退：从 available_masters 查找
                available_masters.iter()
                    .find(|m| {
                        m["mqtt_host"].as_str() == Some(&master_host) &&
                        m["mqtt_port"].as_u64() == Some(master_port as u64)
                    })
                    .and_then(|m| m["location"].as_str())
                    .map(|s| s.to_string())
            } else {
                Some(master_location_db)
            };
            
            // 首先尝试从内存状态检查（适用于同一进程内的节点）
            let mut is_master_online = if let Some(loc) = &master_location {
                nodes.get(loc).map(|n| n.is_online).unwrap_or(false)
            } else {
                false
            };
            
            // 如果内存状态中找不到主节点，可能是跨进程的情况，使用 HTTP 健康检查作为回退
            if !is_master_online {
                // 从 mqtt_host 构建 HTTP URL（假设 HTTP 服务在同一台机器的 8080 端口）
                let http_check_url = if master_host.contains(":1883") {
                    format!("http://{}", master_host.replace(":1883", ":8080"))
                } else if master_host.contains(':') {
                    let host_part = master_host.split(':').next().unwrap_or(&master_host);
                    format!("http://{}:8080", host_part)
                } else {
                    format!("http://{}:8080", master_host)
                };
                
                log::debug!("检查主节点在线状态: {}", http_check_url);
                is_master_online = mqtt_monitor_handlers::check_site_http_status(Some(&http_check_url)).await;
            }

            // 诊断信息：为什么未连接
            let mut diagnostic_message = None;
            if !is_subscription_running && !is_master_online {
                diagnostic_message = Some("从节点订阅未启动，且主节点未在线（主节点需要启动 MQTT 订阅）".to_string());
            } else if !is_subscription_running {
                diagnostic_message = Some("从节点订阅未启动，请点击'启动订阅'按钮".to_string());
            } else if !is_master_online {
                diagnostic_message = Some("主节点未在线（主节点需要启动 MQTT 订阅并发送心跳）".to_string());
            }

            connection_status = json!({
                "connected": is_subscription_running && is_master_online,
                "master_location": master_location.clone(),
                "master_host": master_host.clone(),
                "master_port": master_port,
                "master_online": is_master_online,
                "diagnostic_message": diagnostic_message
            });

            if let Some(loc) = master_location {
                master_info = json!({
                    "location": loc,
                    "host": master_host,
                    "port": master_port,
                    "online": is_master_online
                });
                selected_master = Some(format!("{}:{}", master_host, master_port));
            }
        } else if !available_masters.is_empty() {
            // 有可用主节点但未选择
            connection_status = json!({
                "connected": false,
                "master_location": null,
                "master_host": null,
                "master_port": null,
                "master_online": false,
                "diagnostic_message": "请从下方列表中选择要订阅的主节点"
            });
        } else {
            // 找不到主节点配置
            connection_status = json!({
                "connected": false,
                "master_location": null,
                "master_host": null,
                "master_port": null,
                "master_online": false,
                "diagnostic_message": "未找到主节点配置，请确保主节点已设置为'主节点'"
            });
        }
    }

    Ok(json!({
        "status": "success",
        "is_subscription_running": is_subscription_running,
        "is_server_running": is_server_running,
        "mqtt_server_port": mqtt_server_port,
        "location": location,
        "is_master_node": is_master_node,
        "node_role": if is_master_node { "master" } else { "client" },
        "master_info": master_info,
        "connection_status": connection_status,
        "available_masters": available_masters,
        "selected_master": selected_master
    }))
}

/// 获取 MQTT 订阅状态（公开 API）
pub async fn get_mqtt_subscription_status(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match get_mqtt_subscription_status_internal().await {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(e),
    }
}

// 辅助函数：从数据库获取主节点完整配置（包括 master_location）
fn get_master_config_from_db(current_location: &str) -> Option<(String, String, u16)> {
    use crate::web_server::remote_sync_handlers;
    
    let conn = remote_sync_handlers::open_sqlite().ok()?;
    
    // 查询当前站点配置的主节点信息（使用 master_location 字段）
    // 注意：如果 master_location 为 NULL、空字符串或 'unknown'，返回 None
    conn.query_row(
        "SELECT master_location, master_mqtt_host, COALESCE(master_mqtt_port, 1883)
         FROM remote_sync_sites 
         WHERE location = ?1 AND master_mqtt_host IS NOT NULL AND master_mqtt_port IS NOT NULL
         LIMIT 1",
        rusqlite::params![current_location],
        |row| {
            let master_location: Option<String> = row.get(0)?;
            let host: String = row.get(1)?;
            let port: i64 = row.get(2)?;
            // 如果 master_location 为 NULL、空字符串或 'unknown'，返回错误（会被 .ok() 转换为 None）
            if let Some(loc) = master_location {
                if loc.is_empty() || loc == "unknown" {
                    return Err(rusqlite::Error::InvalidColumnType(0, "master_location".to_string(), rusqlite::types::Type::Text));
                }
                Ok((loc, host, port as u16))
            } else {
                Err(rusqlite::Error::InvalidColumnType(0, "master_location".to_string(), rusqlite::types::Type::Null))
            }
        },
    ).ok()
}

// 辅助函数：查找主节点位置
fn find_master_node_location() -> Option<String> {
    let db_path = if std::path::Path::new("DbOption.toml").exists() {
        config::Config::builder()
            .add_source(config::File::with_name("DbOption"))
            .build()
            .ok()
            .and_then(|b| b.get_string("deployment_sites_sqlite_path").ok())
            .unwrap_or_else(|| "deployment_sites.sqlite".to_string())
    } else {
        "deployment_sites.sqlite".to_string()
    };

    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    // 创建表（如果不存在）
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS node_config (
            location TEXT PRIMARY KEY,
            is_master BOOLEAN NOT NULL DEFAULT 0,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    );

    // 查询主节点
    conn.query_row(
        "SELECT location FROM node_config WHERE is_master = 1 LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

/// 获取所有可用的主节点列表（从 node_config 和 remote_sync_sites 表中查询）
pub(crate) fn get_available_master_nodes() -> Vec<serde_json::Value> {
    let db_path = if std::path::Path::new("DbOption.toml").exists() {
        config::Config::builder()
            .add_source(config::File::with_name("DbOption"))
            .build()
            .ok()
            .and_then(|b| b.get_string("deployment_sites_sqlite_path").ok())
            .unwrap_or_else(|| "deployment_sites.sqlite".to_string())
    } else {
        "deployment_sites.sqlite".to_string()
    };

    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    // 创建表（如果不存在）
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS node_config (
            location TEXT PRIMARY KEY,
            is_master BOOLEAN NOT NULL DEFAULT 0,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    );

    let mut master_nodes = Vec::new();

    // 1. 从 node_config 表查询主节点
    let mut stmt = match conn.prepare("SELECT location FROM node_config WHERE is_master = 1") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = match stmt.query_map([], |row| {
        Ok(row.get::<_, String>(0)?)
    }) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    for row in rows {
        if let Ok(location) = row {
            // 从 remote_sync_sites 表查询该主节点的 MQTT 配置和 HTTP 地址
            // 注意：主节点的 master_mqtt_host 字段存储的是它自己的 MQTT 服务器地址
            let mqtt_config = conn
                .query_row(
                    "SELECT master_mqtt_host, master_mqtt_port, http_host FROM remote_sync_sites WHERE location = ?1 LIMIT 1",
                    [&location],
                    |row| {
                        let host: Option<String> = row.get(0)?;
                        let port: Option<i64> = row.get(1)?;
                        let http_host: Option<String> = row.get(2).ok().flatten();
                        Ok((host, port, http_host))
                    },
                )
                .ok();

            let (mqtt_host, mqtt_port, http_host) = if let Some((Some(host), port, http_host)) = mqtt_config {
                (Some(host), port.map(|p| p as u16), http_host)
            } else {
                // 如果站点表中没有配置，尝试从环境配置获取
                // 或者从主节点自己的配置获取（主节点的 mqtt_host 就是它自己的 MQTT 服务器地址）
                use aios_core::get_db_option;
                let db_option = get_db_option();
                // 如果当前节点就是主节点，使用自己的配置
                if db_option.location == location {
                    (Some(db_option.mqtt_host.clone()), Some(db_option.mqtt_port), Some(db_option.file_server_host.clone()))
                } else {
                    // 否则尝试从环境配置获取
                    let env_config = conn
                        .query_row(
                            "SELECT mqtt_host, mqtt_port FROM remote_sync_envs WHERE location_dbs LIKE '%' || ?1 || '%' LIMIT 1",
                            [&location],
                            |row| {
                                let host: Option<String> = row.get(0)?;
                                let port: Option<i64> = row.get(1)?;
                                Ok((host, port))
                            },
                        )
                        .ok();
                    
                    // 尝试从 remote_sync_sites 获取 http_host
                    let http_host_from_site = conn
                        .query_row(
                            "SELECT http_host FROM remote_sync_sites WHERE location = ?1 LIMIT 1",
                            [&location],
                            |row| row.get::<_, Option<String>>(0),
                        )
                        .ok()
                        .flatten();
                    
                    if let Some((Some(host), port)) = env_config {
                        (Some(host), port.map(|p| p as u16), http_host_from_site)
                    } else {
                        (Some(db_option.mqtt_host.clone()), Some(db_option.mqtt_port), http_host_from_site)
                    }
                }
            };

            master_nodes.push(json!({
                "location": location,
                "mqtt_host": mqtt_host,
                "mqtt_port": mqtt_port.unwrap_or(1883u16),
                "http_host": http_host,
                "source": "node_config"
            }));
        }
    }

    // 2. 从 remote_sync_sites 表查询有主节点MQTT配置的站点（作为备选）
    let mut stmt = match conn.prepare(
        "SELECT DISTINCT master_mqtt_host, master_mqtt_port, location, http_host 
         FROM remote_sync_sites 
         WHERE master_mqtt_host IS NOT NULL AND master_mqtt_port IS NOT NULL"
    ) {
        Ok(s) => s,
        Err(_) => return master_nodes,
    };

    let rows = match stmt.query_map([], |row| {
        let host: String = row.get(0)?;
        let port: i64 = row.get(1)?;
        let location: Option<String> = row.get(2).ok();
        let http_host: Option<String> = row.get(3).ok().flatten();
        Ok((host, port as u16, location, http_host))
    }) {
        Ok(r) => r,
        Err(_) => return master_nodes,
    };

    for row in rows {
        if let Ok((host, port, location, http_host)) = row {
            // 检查是否已经在列表中
            let exists = master_nodes.iter().any(|n| {
                n["mqtt_host"].as_str() == Some(&host) && 
                n["mqtt_port"].as_u64() == Some(port as u64)
            });

            if !exists {
                // 只添加有有效 location 的主节点，跳过 location 为 None 的记录
                if let Some(loc) = location {
                    if !loc.is_empty() && loc != "unknown" {
                        master_nodes.push(json!({
                            "location": loc,
                            "mqtt_host": host,
                            "mqtt_port": port,
                            "http_host": http_host,
                            "source": "remote_sync_sites"
                        }));
                    }
                }
            }
        }
    }

    master_nodes
}

/// 获取 MQTT 服务器状态（检查进程真实状态）
pub async fn get_mqtt_server_status(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::web_server::sync_control_center::check_mqtt_process_status;

    // 检查进程真实状态
    let (is_running, pid, exit_info) = check_mqtt_process_status().await;

    // 获取保存的状态信息
    let center = SYNC_CONTROL_CENTER.read().await;
    let mqtt_server = center.mqtt_server.clone();
    drop(center);

    // 如果进程真实运行状态与保存状态不一致，以真实状态为准
    let actual_status = if is_running {
        mqtt_server.map(|mut s| {
            s.is_running = true;
            s
        })
    } else {
        None
    };

    Ok(Json(json!({
        "status": "success",
        "mqtt_server": actual_status,
        "process_running": is_running,
        "process_pid": pid,
        "exit_info": exit_info
    })))
}

// ========= 请求/响应类型 =========

// StartSyncRequest 已移除，start_sync_service 不再需要请求参数
// 统一使用 DbOption.location 作为站点标识

#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub mqtt_host: Option<String>,
    pub mqtt_port: Option<u16>,
    pub file_server_host: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddTaskRequest {
    pub file_path: String,
    pub file_size: u64,
    pub priority: Option<u8>,
    pub file_name: Option<String>,
    pub file_hash: Option<String>,
    pub record_count: Option<u64>,
    pub env_id: Option<String>,
    pub source_env: Option<String>,
    pub target_site: Option<String>,
    pub direction: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventQuery {
    pub since: Option<u64>, // 时间戳
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct QueueQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StartMqttRequest {
    pub port: Option<u16>,
}

// ========= 辅助函数 =========

async fn sample_system_metrics() -> (f32, f32) {
    match tokio::task::spawn_blocking(|| {
        use std::thread;
        use std::time::Duration as StdDuration;
        use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

        let mut system = System::new();
        system.refresh_specifics(
            RefreshKind::everything()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        thread::sleep(StdDuration::from_millis(100));
        system.refresh_specifics(
            RefreshKind::everything()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );

        let cpu_usage = system.global_cpu_usage();
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory_usage = if total_memory == 0 {
            0.0
        } else {
            (used_memory as f64 / total_memory as f64 * 100.0) as f32
        };

        (cpu_usage, memory_usage)
    })
    .await
    {
        Ok(metrics) => metrics,
        Err(_) => (0.0, 0.0),
    }
}

fn collect_sync_totals() -> (u64, u64, u64, u64) {
    let Ok(conn) = remote_sync_handlers::open_sqlite() else {
        return (0, 0, 0, 0);
    };

    let completed = conn
        .query_row(
            "SELECT COUNT(*), SUM(COALESCE(file_size, 0)), SUM(COALESCE(record_count, 0)) \
             FROM remote_sync_logs WHERE status = 'completed'",
            [],
            |row| {
                let count: i64 = row.get(0)?;
                let bytes: Option<i64> = row.get(1)?;
                let records: Option<i64> = row.get(2)?;
                Ok((count, bytes.unwrap_or(0), records.unwrap_or(0)))
            },
        )
        .unwrap_or((0, 0, 0));

    let failed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM remote_sync_logs WHERE status = 'failed'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    (
        clamp_u64(completed.0),
        clamp_u64(completed.1),
        clamp_u64(completed.2),
        clamp_u64(failed),
    )
}

fn clamp_u64(v: i64) -> u64 {
    if v < 0 { 0 } else { v as u64 }
}

// ========= 节点角色管理 =========

/// 设置当前节点为主节点
pub async fn set_as_master_node(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use aios_core::get_db_option;
    use crate::web_server::remote_sync_handlers;

    let db_option = get_db_option();
    let location = db_option.location.clone();
    let mqtt_host = db_option.mqtt_host.clone();
    let mqtt_port = db_option.mqtt_port;

    // 1. 将主节点标记保存到 SQLite (node_config 表)
    if let Err(e) = save_master_node_flag(&location, true) {
        return Ok(Json(json!({
            "status": "error",
            "message": format!("设置主节点失败: {}", e)
        })));
    }

    // 2. 同时将主节点的 MQTT 信息保存到 remote_sync_sites 表
    // 这样从节点可以通过 get_available_master_nodes() 获取到正确的 MQTT 地址
    if let Ok(conn) = remote_sync_handlers::open_sqlite() {
        // 确保当前站点在 remote_sync_sites 表中有记录
        let _ = conn.execute(
            "INSERT OR IGNORE INTO remote_sync_sites (id, location, env_id, created_at, updated_at)
             VALUES (?1, ?1, 'default', datetime('now'), datetime('now'))",
            rusqlite::params![location],
        );
        
        // 更新主节点的 MQTT 信息（使用 master_mqtt_host/master_mqtt_port 字段存储自己的 MQTT 地址）
        let result = conn.execute(
            "UPDATE remote_sync_sites 
             SET master_mqtt_host = ?1, master_mqtt_port = ?2, master_location = ?3, updated_at = datetime('now')
             WHERE location = ?3",
            rusqlite::params![mqtt_host, mqtt_port as i64, location],
        );
        
        if let Err(e) = result {
            log::warn!("保存主节点 MQTT 配置失败: {}", e);
        } else {
            log::info!(
                "主节点 {} 已保存 MQTT 配置: host={}, port={}",
                location, mqtt_host, mqtt_port
            );
        }
    }

    Ok(Json(json!({
        "status": "success",
        "message": format!("节点 {} 已设置为主节点 (MQTT: {}:{})", location, mqtt_host, mqtt_port)
    })))
}

/// 设置当前节点为从节点（客户端）
pub async fn set_as_client_node(
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use aios_core::get_db_option;

    let db_option = get_db_option();
    let location = db_option.location.clone();

    // 将主节点标记保存到 SQLite
    match save_master_node_flag(&location, false) {
        Ok(_) => Ok(Json(json!({
            "status": "success",
            "message": format!("节点 {} 已设置为从节点（客户端）", location)
        }))),
        Err(e) => Ok(Json(json!({
            "status": "error",
            "message": format!("设置从节点失败: {}", e)
        }))),
    }
}

// 辅助函数：检查是否为主节点
fn check_is_master_node(location: &str) -> bool {
    let db_path = if std::path::Path::new("DbOption.toml").exists() {
        config::Config::builder()
            .add_source(config::File::with_name("DbOption"))
            .build()
            .ok()
            .and_then(|b| b.get_string("deployment_sites_sqlite_path").ok())
            .unwrap_or_else(|| "deployment_sites.sqlite".to_string())
    } else {
        "deployment_sites.sqlite".to_string()
    };

    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // 创建表（如果不存在）
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS node_config (
            location TEXT PRIMARY KEY,
            is_master BOOLEAN NOT NULL DEFAULT 0,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    );

    // 查询主节点标记
    conn.query_row(
        "SELECT is_master FROM node_config WHERE location = ?1",
        [location],
        |row| row.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

// 辅助函数：保存主节点标记
fn save_master_node_flag(location: &str, is_master: bool) -> anyhow::Result<()> {
    let db_path = if std::path::Path::new("DbOption.toml").exists() {
        config::Config::builder()
            .add_source(config::File::with_name("DbOption"))
            .build()
            .ok()
            .and_then(|b| b.get_string("deployment_sites_sqlite_path").ok())
            .unwrap_or_else(|| "deployment_sites.sqlite".to_string())
    } else {
        "deployment_sites.sqlite".to_string()
    };

    let conn = rusqlite::Connection::open(&db_path)?;

    // 创建表（如果不存在）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS node_config (
            location TEXT PRIMARY KEY,
            is_master BOOLEAN NOT NULL DEFAULT 0,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // 插入或更新
    conn.execute(
        "INSERT OR REPLACE INTO node_config (location, is_master, updated_at)
         VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![location, is_master],
    )?;

    Ok(())
}

// ========= 页面渲染 =========

/// 同步控制面板页面
pub async fn sync_control_page() -> Html<String> {
    use crate::web_server::layout;
    let content = render_sync_control_page();
    let wrapped =
        layout::wrap_external_html_in_layout("同步控制 - AIOS", Some("sync-control"), &content);
    Html(wrapped)
}

fn render_sync_control_page() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>同步控制中心</title>
    <link rel="stylesheet" href="/static/simple-tailwind.css">
    <style>
        .status-card {{
            @apply bg-white rounded-lg shadow p-4 mb-4;
        }}
        .status-indicator {{
            @apply inline-block w-3 h-3 rounded-full mr-2;
        }}
        .status-running {{ @apply bg-green-500; }}
        .status-stopped {{ @apply bg-gray-400; }}
        .status-error {{ @apply bg-red-500; }}
        .status-warning {{ @apply bg-yellow-500; }}

        .control-button {{
            @apply px-4 py-2 rounded font-medium transition-colors;
        }}
        .btn-primary {{ @apply bg-blue-500 text-white hover:bg-blue-600; }}
        .btn-success {{ @apply bg-green-500 text-white hover:bg-green-600; }}
        .btn-danger {{ @apply bg-red-500 text-white hover:bg-red-600; }}
        .btn-warning {{ @apply bg-yellow-500 text-white hover:bg-yellow-600; }}

        .metric-card {{
            @apply bg-gray-50 rounded p-3 text-center;
        }}
        .metric-value {{
            @apply text-2xl font-bold text-gray-800;
        }}
        .metric-label {{
            @apply text-sm text-gray-600 mt-1;
        }}

        .log-entry {{
            @apply font-mono text-sm p-2 border-b border-gray-200;
        }}
        .log-entry.error {{ @apply bg-red-50 text-red-800; }}
        .log-entry.warning {{ @apply bg-yellow-50 text-yellow-800; }}
        .log-entry.info {{ @apply bg-blue-50 text-blue-800; }}
    </style>
</head>
<body class="bg-gray-100">
    <div class="container mx-auto p-4">
        <h1 class="text-3xl font-bold mb-6">同步控制中心</h1>

        <!-- 状态概览 -->
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
            <div class="status-card">
                <div class="flex items-center justify-between">
                    <span class="text-gray-600">服务状态</span>
                    <span id="service-status" class="flex items-center">
                        <span class="status-indicator status-stopped"></span>
                        <span>已停止</span>
                    </span>
                </div>
            </div>

            <div class="status-card">
                <div class="flex items-center justify-between">
                    <span class="text-gray-600">MQTT连接</span>
                    <span id="mqtt-status" class="flex items-center">
                        <span class="status-indicator status-stopped"></span>
                        <span>未连接</span>
                    </span>
                </div>
            </div>

            <div class="status-card">
                <div class="flex items-center justify-between">
                    <span class="text-gray-600">文件监听</span>
                    <span id="watcher-status" class="flex items-center">
                        <span class="status-indicator status-stopped"></span>
                        <span>未激活</span>
                    </span>
                </div>
            </div>

            <div class="status-card">
                <div class="flex items-center justify-between">
                    <span class="text-gray-600">队列长度</span>
                    <span id="queue-length" class="text-xl font-bold">0</span>
                </div>
            </div>
        </div>

        <!-- 节点角色管理 -->
        <div class="bg-white rounded-lg shadow p-6 mb-6">
            <h2 class="text-xl font-semibold mb-4">节点角色管理</h2>
            <div class="flex items-center gap-4 flex-wrap">
                <div class="flex items-center gap-3 px-4 py-3 rounded-lg border" id="node-role-card">
                    <span class="text-gray-600 font-medium">当前角色:</span>
                    <span id="node-role-badge" class="px-3 py-1 rounded-full text-sm font-bold">
                        加载中...
                    </span>
                    <span id="node-location" class="text-sm text-gray-500"></span>
                </div>
                <div class="flex items-center gap-2">
                    <button id="btn-set-master" class="control-button btn-primary" style="display: none;">
                        👑 设为主节点
                    </button>
                    <button id="btn-set-client" class="control-button btn-primary" style="display: none;">
                        👥 设为从节点
                    </button>
                    <span id="role-loading" class="text-gray-500 text-sm" style="display: none;">处理中...</span>
                </div>
            </div>
            
            <!-- 主节点选择（仅从节点显示） -->
            <div id="master-selection-section" class="mt-4 p-4 bg-blue-50 rounded-lg border border-blue-200" style="display: none;">
                <h3 class="text-sm font-semibold text-gray-700 mb-2">选择要订阅的主节点</h3>
                <div class="flex items-center gap-3 flex-wrap">
                    <select id="master-select" class="px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500">
                        <option value="">请选择主节点...</option>
                    </select>
                    <button id="btn-start-subscription" class="control-button btn-success text-sm">
                        ▶ 启动订阅
                    </button>
                    <button id="btn-stop-subscription" class="control-button btn-danger text-sm" style="display: none;">
                        ⏹ 停止订阅
                    </button>
                    <button id="btn-clear-master-config" class="control-button btn-warning text-sm" style="display: none;" title="清除主节点配置，可重新选择其他主节点">
                        🗑 取消订阅
                    </button>
                    <span id="subscription-status" class="text-sm text-gray-600"></span>
                </div>
                <div id="master-connection-info" class="mt-2 text-xs text-gray-600"></div>
            </div>
            
            <div class="mt-3 text-sm text-gray-600">
                <p><strong>主节点</strong>：可以启动 MQTT Broker，为其他节点提供消息中转服务</p>
                <p><strong>从节点</strong>：只能作为 MQTT 客户端订阅消息，接收增量更新</p>
            </div>
        </div>

        <!-- 控制按钮 -->
        <div class="bg-white rounded-lg shadow p-6 mb-6">
            <h2 class="text-xl font-semibold mb-4">服务控制</h2>
            <div class="flex gap-3 flex-wrap">
                <button id="btn-start" class="control-button btn-success">
                    启动服务
                </button>
                <button id="btn-stop" class="control-button btn-danger">
                    停止服务
                </button>
                <button id="btn-restart" class="control-button btn-warning">
                    重启服务
                </button>
                <button id="btn-pause" class="control-button btn-primary">
                    暂停同步
                </button>
                <button id="btn-resume" class="control-button btn-primary">
                    恢复同步
                </button>
                <button id="btn-clear-queue" class="control-button btn-danger">
                    清空队列
                </button>
            </div>
        </div>

        <!-- 性能指标 -->
        <div class="bg-white rounded-lg shadow p-6 mb-6">
            <h2 class="text-xl font-semibold mb-4">性能指标</h2>
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div class="metric-card">
                    <div class="metric-value" id="metric-sync-rate">0</div>
                    <div class="metric-label">同步速率 (MB/s)</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value" id="metric-total-synced">0</div>
                    <div class="metric-label">已同步文件</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value" id="metric-success-rate">0%</div>
                    <div class="metric-label">成功率</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value" id="metric-uptime">0s</div>
                    <div class="metric-label">运行时长</div>
                </div>
            </div>
        </div>

        <!-- 实时日志 -->
        <div class="bg-white rounded-lg shadow p-6">
            <h2 class="text-xl font-semibold mb-4">实时日志</h2>
            <div id="log-container" class="h-64 overflow-y-auto border border-gray-200 rounded">
                <!-- 日志内容将动态插入这里 -->
            </div>
        </div>
    </div>

    <script src="/static/sync-control.js"></script>
</body>
</html>"#
    )
}

// ============================================================================
// Metrics History API
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct MetricsHistoryQuery {
    pub time_range: Option<String>, // "hour", "day", "week", "month"
    pub limit: Option<usize>,
}

/// 获取性能指标历史
pub async fn get_sync_metrics_history(
    Query(params): Query<MetricsHistoryQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use rusqlite::Connection;

    let time_range = params.time_range.as_deref().unwrap_or("day");
    let limit = params.limit.unwrap_or(100).min(1000);

    // 计算时间范围
    let hours_ago = match time_range {
        "hour" => 1,
        "day" => 24,
        "week" => 24 * 7,
        "month" => 24 * 30,
        _ => 24,
    };

    let conn = Connection::open("deployment_sites.sqlite")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 从日志中聚合历史指标
    let sql = format!(
        "SELECT 
            datetime(created_at) as timestamp,
            COUNT(*) as task_count,
            SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed_count,
            SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed_count,
            SUM(CASE WHEN status = 'completed' THEN COALESCE(file_size, 0) ELSE 0 END) as total_bytes,
            AVG(CASE WHEN status = 'completed' AND completed_at IS NOT NULL AND started_at IS NOT NULL 
                THEN (julianday(completed_at) - julianday(started_at)) * 86400000 
                ELSE NULL END) as avg_sync_time_ms
         FROM remote_sync_logs
         WHERE datetime(created_at) >= datetime('now', '-{} hours')
         GROUP BY strftime('%Y-%m-%d %H:00:00', created_at)
         ORDER BY timestamp DESC
         LIMIT ?",
        hours_ago
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = stmt
        .query_map([limit as i64], |row| {
            Ok(json!({
                "timestamp": row.get::<_, String>(0).unwrap_or_default(),
                "task_count": row.get::<_, i64>(1).unwrap_or(0),
                "completed_count": row.get::<_, i64>(2).unwrap_or(0),
                "failed_count": row.get::<_, i64>(3).unwrap_or(0),
                "total_bytes": row.get::<_, i64>(4).unwrap_or(0),
                "avg_sync_time_ms": row.get::<_, Option<f64>>(5).unwrap_or(None).unwrap_or(0.0),
            }))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut history = Vec::new();
    for row in rows {
        history.push(row.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }

    Ok(Json(json!({
        "status": "success",
        "time_range": time_range,
        "history": history
    })))
}
