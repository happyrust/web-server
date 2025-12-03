use once_cell::sync::Lazy;
use rusqlite::OptionalExtension;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::data_interface::tidb_manager::AiosDBManager;

pub struct RuntimeState {
    pub env_id: String,
    pub mgr: Arc<AiosDBManager>,
    pub watcher_handle: Option<tokio::task::JoinHandle<()>>,
    pub mqtt_handle: Option<tokio::task::JoinHandle<()>>,
    /// MQTT 发布器 EventLoop 句柄（用于发送消息）
    #[cfg(feature = "mqtt")]
    pub mqtt_publisher_handle: Option<tokio::task::JoinHandle<()>>,
}

pub static REMOTE_RUNTIME: Lazy<RwLock<Option<RuntimeState>>> = Lazy::new(|| RwLock::new(None));

/// 停止当前运行态（如存在）
pub async fn stop_runtime() {
    let mut guard = REMOTE_RUNTIME.write().await;
    if let Some(state) = guard.as_mut() {
        if let Some(h) = state.watcher_handle.take() {
            h.abort();
        }
        if let Some(h) = state.mqtt_handle.take() {
            h.abort();
        }
        #[cfg(feature = "mqtt")]
        if let Some(h) = state.mqtt_publisher_handle.take() {
            h.abort();
        }
    }
    *guard = None;
}

/// 获取当前站点的主节点MQTT配置
pub async fn get_master_mqtt_config(env_id: Option<&str>) -> anyhow::Result<Option<(String, u16)>> {
    use crate::web_server::remote_sync_handlers;
    use aios_core::get_db_option;

    let db_option = get_db_option();

    // 先按当前激活的环境查站点记录中的主节点配置
    let conn = remote_sync_handlers::open_sqlite()
        .map_err(|e| anyhow::anyhow!("打开数据库失败: {}", e))?;

    // 1) 优先使用 env_id 定位到对应环境的站点记录
    if let Some(eid) = env_id {
        let mut stmt = conn
            .prepare(
                "SELECT master_mqtt_host, master_mqtt_port 
             FROM remote_sync_sites 
             WHERE env_id = ?1 AND master_mqtt_host IS NOT NULL AND master_mqtt_port IS NOT NULL
             LIMIT 1",
            )
            .map_err(|e| anyhow::anyhow!("准备查询失败: {}", e))?;

        let result = stmt
            .query_row(rusqlite::params![eid], |row| {
                let host: Option<String> = row.get(0)?;
                let port: Option<i64> = row.get(1)?;
                Ok((host, port))
            })
            .optional()
            .map_err(|e| anyhow::anyhow!("查询失败: {}", e))?;

        if let Some((Some(host), port)) = result {
            let port = port.map(|p| p as u16).unwrap_or(1883);
            return Ok(Some((host, port)));
        }
    }

    // 2) 回退：按当前站点 location 查找
    let mut stmt = conn
        .prepare(
            "SELECT master_mqtt_host, master_mqtt_port 
         FROM remote_sync_sites 
         WHERE location = ?1 
         LIMIT 1",
        )
        .map_err(|e| anyhow::anyhow!("准备查询失败: {}", e))?;

    let result = stmt
        .query_row(rusqlite::params![db_option.location.clone()], |row| {
            let host: Option<String> = row.get(0)?;
            let port: Option<i64> = row.get(1)?;
            Ok((host, port))
        })
        .optional()
        .map_err(|e| anyhow::anyhow!("查询失败: {}", e))?;

    if let Some((Some(host), port)) = result {
        let port = port.map(|p| p as u16).unwrap_or(1883);
        return Ok(Some((host, port)));
    }

    Ok(None)
}

/// 使用当前 DbOption 配置启动 MQTT (Publisher + Subscriber)
/// 注意：文件监听 (async_watch) 由 web_server/mod.rs 在 sync_live=true 时启动
/// 此函数只负责启动 MQTT 功能，避免重复启动 watcher 导致事件被处理两次
pub async fn start_runtime(env_id: String) -> anyhow::Result<()> {
    let (init_ms, max_ms) = query_backoff_ms(&env_id).unwrap_or((1000, 30_000));
    let mgr = Arc::new(AiosDBManager::init_form_config().await?);
    
    // 必须调用 init_watcher() 来填充 db_path_map，否则 MQTT 收到消息后无法找到文件路径
    // 注意：这和 web_server/mod.rs 中的是不同的 AiosDBManager 实例
    if let Err(e) = mgr.init_watcher().await {
        log::error!("初始化 watcher 失败: {:?}", e);
    } else {
        log::info!("watcher 初始化完成，db_path_map 已填充");
    }

    // 检查当前站点的主节点MQTT配置
    let master_mqtt_config = get_master_mqtt_config(Some(&env_id)).await?;

    if let Some((host, port)) = &master_mqtt_config {
        log::info!("从节点模式：连接到主节点MQTT服务器 {}:{}", host, port);
    } else {
        log::info!("主节点模式：使用本地MQTT配置");
    }

    // 🔧 不再启动文件监听（避免重复）
    // async_watch 已由 web_server/mod.rs 在启动时运行
    let watcher_handle: Option<tokio::task::JoinHandle<()>> = None;

    // 启动 MQTT 订阅（传递主节点配置）
    let watcher_arc = mgr.watcher.clone();
    let master_config = master_mqtt_config.clone();
    let mqtt_handle = tokio::spawn(async move {
        // 忽略错误并常驻循环
        if let Some((host, port)) = master_config {
            AiosDBManager::poll_sync_e3d_mqtt_events_with_backoff_and_config(
                watcher_arc,
                init_ms,
                max_ms,
                Some(host),
                Some(port),
            )
            .await;
        } else {
            AiosDBManager::poll_sync_e3d_mqtt_events_with_backoff(watcher_arc, init_ms, max_ms)
                .await;
        }
    });

    // 启动 MQTT 发布器 EventLoop（用于发送消息）
    #[cfg(feature = "mqtt")]
    let mqtt_publisher_handle = {
        log::info!("启动 MQTT 发布器 EventLoop...");
        Some(mgr.start_mqtt_publisher())
    };

    let mut guard = REMOTE_RUNTIME.write().await;
    *guard = Some(RuntimeState {
        env_id,
        mgr,
        watcher_handle, // 现在为 None，因为 watcher 由 web_server/mod.rs 管理
        mqtt_handle: Some(mqtt_handle),
        #[cfg(feature = "mqtt")]
        mqtt_publisher_handle,
    });
    Ok(())
}

fn query_backoff_ms(env_id: &str) -> Option<(u64, u64)> {
    // 复用 handlers 中的配置文件约定
    use config as cfg;
    let db_path = if std::path::Path::new("DbOption.toml").exists() {
        cfg::Config::builder()
            .add_source(cfg::File::with_name("DbOption"))
            .build()
            .ok()
            .and_then(|b| b.get_string("deployment_sites_sqlite_path").ok())
            .unwrap_or_else(|| "deployment_sites.sqlite".to_string())
    } else {
        "deployment_sites.sqlite".to_string()
    };
    let conn = rusqlite::Connection::open(&db_path).ok()?;
    let mut stmt = conn
        .prepare("SELECT reconnect_initial_ms, reconnect_max_ms FROM remote_sync_envs WHERE id = ?1 LIMIT 1")
        .ok()?;
    let mut rows = stmt.query(rusqlite::params![env_id]).ok()?;
    let row = rows.next().ok()??;
    let init: Option<i64> = row.get(0).ok().flatten();
    let max: Option<i64> = row.get(1).ok().flatten();
    Some((init.unwrap_or(1000) as u64, max.unwrap_or(30_000) as u64))
}
