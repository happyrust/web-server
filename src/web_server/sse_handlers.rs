// SSE (Server-Sent Events) 事件流处理器

use axum::{
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::SystemTime;
use tokio_stream::wrappers::BroadcastStream;

use crate::web_server::sync_control_center::SYNC_EVENT_TX;

/// SSE 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SyncEvent {
    Started {
        env_id: String,
        timestamp: String,
    },
    Stopped {
        env_id: String,
        timestamp: String,
    },
    Paused {
        env_id: String,
        timestamp: String,
    },
    Resumed {
        env_id: String,
        timestamp: String,
    },
    SyncStarted {
        task_id: String,
        file_path: String,
        file_size: u64,
        timestamp: String,
    },
    SyncProgress {
        task_id: String,
        progress: u8,
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
    MqttConnected {
        env_id: String,
        timestamp: String,
    },
    MqttDisconnected {
        env_id: String,
        reason: String,
        timestamp: String,
    },
    QueueSizeChanged {
        env_id: String,
        queue_size: u32,
        timestamp: String,
    },
    MetricsUpdated {
        env_id: String,
        metrics: serde_json::Value,
        timestamp: String,
    },
    ConnectionChanged {
        mqtt_connected: bool,
        watcher_active: bool,
        timestamp: String,
    },
    ProgressUpdate {
        total: u64,
        completed: u64,
        failed: u64,
        pending: u64,
        timestamp: String,
    },
    Alert {
        level: String,
        message: String,
        timestamp: String,
    },
    /// MQTT 订阅状态变化
    MqttSubscriptionStatusChanged {
        is_running: bool,
        is_server_running: bool,
        location: String,
        is_master_node: bool,
        node_role: String,
        connection_status: serde_json::Value,
        available_masters: Vec<serde_json::Value>,
        timestamp: String,
    },
    /// 节点角色状态变化
    NodeRoleChanged {
        location: String,
        is_master_node: bool,
        node_role: String,
        master_info: serde_json::Value,
        connection_status: serde_json::Value,
        available_masters: Vec<serde_json::Value>,
        timestamp: String,
    },
}

impl SyncEvent {
    /// 获取当前时间戳字符串
    pub fn now() -> String {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    }
}

/// SSE 事件流处理器 - 同步事件
///
/// GET /api/sync/events/stream
///
/// 返回 Server-Sent Events 流，实时推送同步事件
pub async fn sync_events_handler() -> impl IntoResponse {
    // 订阅事件广播通道
    let rx = SYNC_EVENT_TX.subscribe();

    // 将 broadcast receiver 转换为 Stream
    let stream = BroadcastStream::new(rx);

    // 转换为 SSE 事件流
    let event_stream = stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                // 序列化事件为 JSON
                match serde_json::to_string(&event) {
                    Ok(json) => Some(Ok::<_, Infallible>(
                        Event::default().data(json).event("message"),
                    )),
                    Err(e) => {
                        eprintln!("Failed to serialize SSE event: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("SSE broadcast error: {}", e);
                None
            }
        }
    });

    // 返回 SSE 响应
    Sse::new(event_stream).keep_alive(KeepAlive::default())
}

/// SSE 事件流处理器 - MQTT 订阅状态和节点角色状态
///
/// GET /api/mqtt/subscription/status/stream
///
/// 返回 Server-Sent Events 流，实时推送 MQTT 订阅状态和节点角色状态变化
pub async fn mqtt_subscription_status_stream_handler(
    _state: axum::extract::State<crate::web_server::AppState>,
) -> impl IntoResponse {
    use crate::web_server::sync_control_center::SYNC_EVENT_TX;
    
    // 订阅事件广播通道
    let rx = SYNC_EVENT_TX.subscribe();
    
    // 将 broadcast receiver 转换为 Stream
    let stream = BroadcastStream::new(rx);
    
    // 过滤只关注 MQTT 订阅状态和节点角色相关的事件
    let event_stream = stream.filter_map(|result| async move {
        match result {
            Ok(event) => {
                // 只处理 MQTT 订阅状态和节点角色相关的事件
                match &event {
                    SyncEvent::MqttSubscriptionStatusChanged { .. } |
                    SyncEvent::NodeRoleChanged { .. } |
                    SyncEvent::MqttConnected { .. } |
                    SyncEvent::MqttDisconnected { .. } => {
                        // 序列化事件为 JSON
                        match serde_json::to_string(&event) {
                            Ok(json) => Some(Ok::<_, Infallible>(
                                Event::default().data(json).event("status"),
                            )),
                            Err(e) => {
                                eprintln!("Failed to serialize SSE event: {}", e);
                                None
                            }
                        }
                    }
                    _ => None, // 忽略其他事件
                }
            }
            Err(e) => {
                eprintln!("SSE broadcast error: {}", e);
                None
            }
        }
    });
    
    // 返回 SSE 响应
    Sse::new(event_stream).keep_alive(KeepAlive::default())
}

/// 测试 SSE 连接的处理器
///
/// GET /api/sync/events/test
///
/// 发送测试事件以验证 SSE 连接
pub async fn test_sse_handler() -> impl IntoResponse {
    let test_event = SyncEvent::Started {
        env_id: "test-env".to_string(),
        timestamp: SyncEvent::now(),
    };

    // 发送测试事件
    match SYNC_EVENT_TX.send(test_event) {
        Ok(_) => (StatusCode::OK, "Test event sent"),
        Err(e) => {
            eprintln!("Failed to send test event: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to send test event",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_event_serialization() {
        let event = SyncEvent::SyncStarted {
            task_id: "task-123".to_string(),
            file_path: "/path/to/file.cba".to_string(),
            file_size: 1024000,
            timestamp: "1234567890".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("SyncStarted"));
        assert!(json.contains("task-123"));
    }
}
