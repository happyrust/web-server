use axum::{
    Router,
    extract::{Query, State},
    http::{Method, StatusCode, header},
    response::{Html, Json},
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, oneshot};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use uuid::Uuid;

pub mod handlers;
pub mod models;
pub mod ws; // WebSocket 模块
// pub mod templates; // 暂时禁用，有语法错误
pub mod auth_handlers;
pub mod batch_tasks_template;
pub mod config_reload_manager; // 配置热重载
pub mod dashboard_handlers; // 增量更新实时监控仪表盘 - API
pub mod dashboard_template; // 增量更新实时监控仪表盘 - 页面
pub mod database_diagnostics;
pub mod database_status_handlers;
pub mod db_connection;
pub mod db_startup_handlers;
pub mod db_startup_manager;
pub mod db_status_handlers;
pub mod db_status_template;
pub mod incremental_update_handlers;
pub mod layout;
pub mod litefs_handlers;
pub mod mqtt_monitor_handlers;
pub mod remote_runtime;
pub mod remote_sync_handlers;
pub mod remote_sync_template;
pub mod room_api;
pub mod room_page;
pub mod simple_templates;
pub mod site_config_handlers; // 站点配置管理
pub mod site_metadata;
pub mod sse_handlers; // SSE 事件流处理器
pub mod sync_control_center;
pub mod sync_control_handlers;
pub mod task_creation_handlers;
pub mod topology_handlers; // 拓扑配置处理器
pub mod wizard_handlers;
pub mod wizard_template;

use crate::web_api::{
    NounHierarchyApiState, SpatialQueryApiState, create_noun_hierarchy_routes,
    create_spatial_query_routes,
};
use handlers::*;
use models::*;

/// Web UI应用状态
#[derive(Clone)]
pub struct AppState {
    /// 任务管理器
    pub task_manager: Arc<Mutex<TaskManager>>,
    /// 配置管理器（前端模板用途，非 DbOption）
    pub config_manager: Arc<RwLock<ConfigManager>>,
    /// 配置热重载管理器（DbOption 热更新）
    pub config_reload: Arc<crate::web_server::config_reload_manager::ConfigReloadManager>,
    /// 进度广播中心（用于 WebSocket 和 gRPC）
    pub progress_hub: Arc<crate::shared::ProgressHub>,
    /// 服务器关闭信号发送器（用于优雅关闭和重启）
    pub shutdown_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

/// 任务管理器
#[derive(Default)]
pub struct TaskManager {
    /// 活跃任务列表
    pub active_tasks: HashMap<String, TaskInfo>,
    /// 任务历史记录
    pub task_history: Vec<TaskInfo>,
}

/// 配置管理器
#[derive(Default)]
pub struct ConfigManager {
    /// 当前配置
    pub current_config: DatabaseConfig,
    /// 配置模板
    pub config_templates: HashMap<String, DatabaseConfig>,
}

impl AppState {
    pub fn new() -> Self {
        let mut config_manager = ConfigManager::default();

        // 添加一些预设配置模板
        config_manager.add_template(
            "default",
            DatabaseConfig {
                name: "默认配置".to_string(),
                manual_db_nums: vec![],
                gen_model: true,
                gen_mesh: true,
                gen_spatial_tree: true,
                apply_boolean_operation: true,
                mesh_tol_ratio: 3.0,
                room_keyword: "-RM".to_string(),
                project_name: "AvevaMarineSample".to_string(),
                project_path: "/Users/dongpengcheng/Documents/models/e3d_models".to_string(),
                project_code: 1516,
                ..Default::default()
            },
        );

        config_manager.add_template(
            "db_7999",
            DatabaseConfig {
                name: "数据库7999配置".to_string(),
                manual_db_nums: vec![7999],
                gen_model: true,
                gen_mesh: true,
                gen_spatial_tree: true,
                apply_boolean_operation: true,
                mesh_tol_ratio: 3.0,
                room_keyword: "-RM".to_string(),
                project_name: "AvevaMarineSample".to_string(),
                project_path: "/Users/dongpengcheng/Documents/models/e3d_models".to_string(),
                project_code: 1516,
                ..Default::default()
            },
        );

        // 创建任务管理器并恢复之前保存的任务
        let mut task_manager = TaskManager::default();

        // 从SQLite恢复任务
        let restored_tasks = wizard_handlers::restore_tasks_from_sqlite();
        for task in restored_tasks {
            task_manager.active_tasks.insert(task.id.clone(), task);
        }

        let config_reload = config_reload_manager::ConfigReloadManager::new()
            .expect("Failed to initialize ConfigReloadManager");

        Self {
            task_manager: Arc::new(Mutex::new(task_manager)),
            config_manager: Arc::new(RwLock::new(config_manager)),
            config_reload: Arc::new(config_reload),
            progress_hub: Arc::new(crate::shared::ProgressHub::default()),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }
}

impl ConfigManager {
    pub fn add_template(&mut self, name: &str, config: DatabaseConfig) {
        self.config_templates.insert(name.to_string(), config);
    }
}

/// 解析模板文件路径
/// 优先检查可执行文件同目录下的 templates，然后向上查找项目根目录
pub fn resolve_template_path(relative_path: &str) -> PathBuf {
    // 提取文件名（如 "incremental_update_vue.html"）
    let path_buf = PathBuf::from(relative_path);
    let file_name = path_buf.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // 1. 优先检查可执行文件同目录下的 templates（便携式部署，最高优先级）
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_template_path = exe_dir.join("templates").join(file_name);
            if exe_template_path.exists() {
                return exe_template_path;
            }
        }
    }

    // 2. 尝试当前工作目录（开发环境或从项目根目录启动）
    let current_dir_path = PathBuf::from(relative_path);
    if current_dir_path.exists() {
        return current_dir_path;
    }

    // 3. 尝试相对于可执行文件的路径（站点部署环境，向上查找项目根目录）
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // 从 site-main/bin/web_server.exe 向上找到项目根目录
            let mut path = exe_dir.to_path_buf();
            // 向上查找，直到找到包含 templates 的目录
            for _ in 0..5 {
                let template_path = path.join(relative_path);
                if template_path.exists() {
                    return template_path;
                }
                if let Some(parent) = path.parent() {
                    path = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }
    }

    // 如果都找不到，返回相对路径（会在运行时失败，但至少不会编译错误）
    PathBuf::from(relative_path)
}

/// 通过UdpSocket获取本机IP地址
fn get_local_ip_via_udp() -> Result<String, std::io::Error> {
    // 连接到一个外部地址（不需要实际连接成功）
    // 这个方法会返回用于发送数据包的网络接口的IP地址
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    let local_addr = socket.local_addr()?;

    if let IpAddr::V4(ipv4) = local_addr.ip() {
        Ok(ipv4.to_string())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "无法获取IPv4地址",
        ))
    }
}

/// 解析静态文件目录路径
/// 尝试多个可能的路径，确保无论工作目录在哪里都能找到静态文件
/// 优先检查可执行文件同目录下的 static（便携式部署）
fn resolve_static_dir() -> PathBuf {
    // 1. 优先检查可执行文件同目录下的 static（便携式部署，最高优先级）
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_static_path = exe_dir.join("static");
            if exe_static_path.exists() && exe_static_path.is_dir() {
                println!(
                    "📂 使用静态文件目录（可执行文件同目录）: {}",
                    exe_static_path.display()
                );
                return exe_static_path;
            }
        }
    }

    // 2. 尝试当前工作目录（开发环境或从项目根目录启动）
    let current_dir_path = PathBuf::from("src/web_server/static");
    if current_dir_path.exists() && current_dir_path.is_dir() {
        println!(
            "📂 使用静态文件目录（当前目录）: {}",
            current_dir_path.display()
        );
        return current_dir_path;
    }

    // 3. 尝试站点目录下的 static（备用）
    let site_static_path = PathBuf::from("static");
    if site_static_path.exists() && site_static_path.is_dir() {
        println!(
            "📂 使用静态文件目录（站点目录）: {}",
            site_static_path.display()
        );
        return site_static_path;
    }

    // 如果都找不到，返回默认路径（会在运行时失败，但至少不会编译错误）
    println!("⚠️  警告: 未找到静态文件目录，使用默认路径: src/web_server/static");
    PathBuf::from("src/web_server/static")
}

/// 解析归档文件目录路径
/// 尝试多个可能的路径，确保无论工作目录在哪里都能找到 assets/archives 目录
pub fn resolve_archives_dir() -> PathBuf {
    // 1. 优先检查可执行文件同目录下的 assets/archives（便携式部署，最高优先级）
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_archives_path = exe_dir.join("assets/archives");
            if exe_archives_path.exists() && exe_archives_path.is_dir() {
                println!(
                    "📂 使用归档文件目录（可执行文件同目录）: {}",
                    exe_archives_path.display()
                );
                return exe_archives_path;
            }
        }
    }

    // 2. 尝试当前工作目录（开发环境或从项目根目录启动）
    let current_dir_path = PathBuf::from("assets/archives");
    if current_dir_path.exists() && current_dir_path.is_dir() {
        println!(
            "📂 使用归档文件目录（当前目录）: {}",
            current_dir_path.display()
        );
        return current_dir_path;
    }

    // 3. 尝试从可执行文件向上查找项目根目录
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut path = exe_dir.to_path_buf();
            // 向上查找，直到找到包含 assets/archives 的目录
            for _ in 0..5 {
                let archives_path = path.join("assets/archives");
                if archives_path.exists() && archives_path.is_dir() {
                    println!(
                        "📂 使用归档文件目录（向上查找）: {}",
                        archives_path.display()
                    );
                    return archives_path;
                }
                if let Some(parent) = path.parent() {
                    path = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }
    }

    // 如果都找不到，返回默认路径（会在运行时失败，但至少不会编译错误）
    println!("⚠️  警告: 未找到归档文件目录，使用默认路径: assets/archives");
    PathBuf::from("assets/archives")
}

/// 启动Web UI服务器
pub async fn start_web_server(port: u16) -> anyhow::Result<()> {
    start_web_server_with_config(port, None, None).await
}

pub async fn start_web_server_with_config(
    port: u16,
    config_file: Option<&str>,
    host: Option<String>,
) -> anyhow::Result<()> {
    let app_state = AppState::new();

    // 创建 shutdown channel
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    {
        let mut tx_guard = app_state.shutdown_tx.lock().await;
        *tx_guard = Some(shutdown_tx);
    }

    // 如果指定了配置文件，设置环境变量
    if let Some(config_path) = config_file {
        unsafe {
            std::env::set_var("DB_OPTION_FILE", config_path);
        }
        println!("⚙️  使用配置文件: {}.toml", config_path);
    }

    // 📂 优先加载静态文件目录（启动时立即检查）
    println!("🔍 正在查找静态文件目录...");
    let static_dir = resolve_static_dir();
    println!("✅ 静态文件目录已确定: {}", static_dir.display());

    // 🔧 修复：初始化数据库连接
    println!("🔄 正在初始化数据库连接...");
    println!("📂 当前工作目录: {:?}", std::env::current_dir()?);

    let config_name = std::env::var("DB_OPTION_FILE").unwrap_or_else(|_| "DbOption".to_string());
    println!("📄 尝试读取 {}.toml 配置文件...", config_name);

    match aios_core::init_surreal().await {
        Ok(_) => {
            println!("✅ 数据库连接初始化成功");
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("Already connected") {
                println!("⚠️ 数据库已经连接，跳过重复初始化");
            } else {
                eprintln!("❌ 数据库初始化失败: {}", error_msg);
                eprintln!("💡 请确保:");
                eprintln!("   1. DbOption.toml 文件在当前目录");
                eprintln!("   2. SurrealDB 服务运行在配置的端口 (默认 8020)");
                eprintln!("   3. 配置文件中的连接信息正确");
                return Err(anyhow::anyhow!("数据库连接初始化失败: {}", error_msg));
            }
        }
    }

    // 初始化 SurrealDB 中的 projects 表（若已存在忽略错误）
    crate::web_server::handlers::ensure_projects_schema().await;
    // 初始化 SurrealDB 中的 deployment_sites 表
    crate::web_server::handlers::ensure_deployment_sites_schema().await;

    // 初始化空间查询API
    let db_manager = crate::AiosDBManager::init_form_config().await?;
    let spatial_query_state = SpatialQueryApiState {
        db_manager: Arc::new(db_manager.clone()),
    };
    let spatial_query_routes = create_spatial_query_routes(spatial_query_state);

    // 初始化名词层级查询API
    let noun_hierarchy_state = NounHierarchyApiState {
        db_manager: Arc::new(db_manager.clone()),
    };
    let noun_hierarchy_routes = create_noun_hierarchy_routes(noun_hierarchy_state);

    // 🔥 启动增量监测后台任务（如果启用）
    let db_option = aios_core::get_db_option();
    if db_option.sync_live.unwrap_or(false) {
        // 检查是否启用 web_server 自动监测（需要在 rs-core 中添加该字段支持）
        // 暂时使用 sync_live 字段作为开关
        println!("🔍 启动增量监测后台任务...");

        // 🎯 设置 ProgressHub 到 SYNC_CONTROL_CENTER，用于实时 WebSocket 推送
        {
            let mut center = sync_control_center::SYNC_CONTROL_CENTER.write().await;
            center.progress_hub = Some(app_state.progress_hub.clone());
            println!("✅ ProgressHub 已注入到 SyncControlCenter");
        }

        let mgr_for_watch = db_manager.clone();
        
        // 🔧 注意：MQTT Publisher 不在此自动启动
        // MQTT 功能（Publisher + Subscriber）完全由前端控制：
        // - 用户点击 "启动订阅" 按钮 → POST /api/mqtt/subscription/start
        // - 后端调用 start_runtime() → 启动 Publisher + Subscriber
        // 这样可以避免 MQTT Broker 未运行时的连接错误刷屏
        #[cfg(feature = "mqtt")]
        println!("ℹ️  MQTT 功能由前端控制，请通过 MQTT 监控页面启动订阅");
        
        tokio::spawn(async move {
            // 🔥 重要：先初始化监听器，扫描并缓存所有数据库文件的header信息
            // 注意：数据库连接已在第310行初始化完成，这里可以安全地使用 SUL_DB
            println!("📋 正在初始化数据库文件监听器...");
            if let Err(e) = mgr_for_watch.init_watcher().await {
                eprintln!("❌ 初始化监听器失败: {:?}", e);
                return;
            }
            println!("✅ 监听器初始化完成");

            println!("📡 增量监测任务已启动，正在监听文件变化...");
            if let Err(e) = mgr_for_watch.async_watch().await {
                eprintln!("❌ 增量监测任务异常退出: {:?}", e);
            }
        });
        println!("✅ 增量监测后台任务已启动");
    } else {
        println!("ℹ️  增量监测未启用 (sync_live = false)");
    }

    // 初始化房间 API
    let room_api_state = room_api::RoomApiState {
        task_manager: Arc::new(tokio::sync::RwLock::new(
            room_api::RoomTaskManager::default(),
        )),
        progress_hub: app_state.progress_hub.clone(),
    };
    let room_routes = room_api::create_room_api_routes().with_state(room_api_state);

    let app = Router::new()
        // API路由
        .route("/api/auth/token", post(auth_handlers::generate_token))
        .route("/api/tasks", get(get_tasks).post(create_task))
        .route("/api/tasks/{id}", get(get_task).delete(delete_task))
        .route("/api/tasks/{id}/start", post(start_task))
        .route("/api/tasks/{id}/stop", post(stop_task))
        .route("/api/tasks/{id}/restart", post(restart_task))
        .route("/api/tasks/{id}/error", get(get_task_error_details))
        .route("/api/tasks/{id}/logs", get(get_task_logs))
        .route("/api/tasks/batch", post(create_batch_tasks))
        .route("/api/tasks/next-number", get(get_next_task_number))
        .route("/api/templates", get(get_task_templates))
        .route("/api/config", get(get_config).post(update_config))
        .route("/api/config/templates", get(get_config_templates))
        .route("/api/databases", get(get_available_databases))
        .route("/api/status", get(get_system_status))
        // 基于 Refno 的模型生成 API
        .route(
            "/api/model/generate-by-refno",
            post(handlers::api_generate_by_refno),
        )
        // SurrealDB 控制 (暂时注释掉有编译问题的路由)
        // .route("/api/surreal/start", post(handlers::start_surreal_server))
        // .route("/api/surreal/stop", post(handlers::stop_surreal_server))
        // .route("/api/surreal/restart", post(handlers::restart_surreal_server))
        .route("/api/surreal/status", get(handlers::get_surreal_status))
        .route("/api/surreal/test", post(handlers::test_surreal_connection))
        .route("/api/surreal/check-port", get(handlers::check_port_status))
        .route(
            "/api/surreal/kill-port",
            post(handlers::kill_port_processes_api),
        )
        // 数据库连接监控API
        .route(
            "/api/database/connection/check",
            get(handlers::check_database_connection),
        )
        .route(
            "/api/database/diagnostics",
            get(handlers::run_database_diagnostics_api),
        )
        .route(
            "/api/database/startup-scripts",
            get(handlers::get_startup_scripts),
        )
        .route(
            "/api/database/start-instance",
            post(handlers::start_database_instance),
        )
        // 数据库启动管理API
        .route(
            "/api/database/startup/start",
            post(db_startup_handlers::start_database_api),
        )
        .route(
            "/api/database/startup/status",
            get(db_startup_handlers::get_startup_status),
        )
        .route(
            "/api/database/startup/instances",
            get(db_startup_handlers::get_all_instances),
        )
        .route(
            "/api/database/startup/stop",
            post(db_startup_handlers::stop_database_api),
        )
        .route(
            "/api/database/startup/logs",
            get(db_startup_handlers::get_startup_logs),
        )
        // Dashboard API - 增量更新实时监控
        .route(
            "/api/dashboard/summary",
            get(dashboard_handlers::get_dashboard_summary),
        )
        .route(
            "/api/dashboard/active-tasks",
            get(dashboard_handlers::get_active_tasks),
        )
        .route(
            "/api/dashboard/failed-tasks",
            get(dashboard_handlers::get_failed_tasks),
        )
        .route(
            "/api/dashboard/task-details/{id}",
            get(dashboard_handlers::get_task_details),
        )
        .route(
            "/api/dashboard/stats/timeline",
            get(dashboard_handlers::get_timeline_stats),
        )
        .route(
            "/api/dashboard/retry-task/{id}",
            post(dashboard_handlers::retry_failed_task),
        )
        .route(
            "/api/dashboard/cleanup-exhausted",
            post(dashboard_handlers::cleanup_exhausted_tasks),
        )
        .route(
            "/api/dashboard/increment-elements/{sync_id}",
            get(dashboard_handlers::get_increment_elements),
        )
        // 增量更新检测API
        .route(
            "/api/incremental/status",
            get(incremental_update_handlers::get_all_incremental_status),
        )
        .route(
            "/api/incremental/site/{site_id}",
            get(incremental_update_handlers::get_site_incremental_details),
        )
        .route(
            "/api/incremental/detect/{site_id}",
            post(incremental_update_handlers::start_incremental_detection),
        )
        .route(
            "/api/incremental/sync/{site_id}",
            post(incremental_update_handlers::start_incremental_sync),
        )
        .route(
            "/api/incremental/task/{task_id}",
            get(incremental_update_handlers::get_detection_task_status),
        )
        .route(
            "/api/incremental/task/{task_id}/cancel",
            post(incremental_update_handlers::cancel_task),
        )
        .route(
            "/api/incremental/config",
            get(incremental_update_handlers::get_incremental_config),
        )
        .route(
            "/api/incremental/config",
            post(incremental_update_handlers::update_incremental_config),
        )
        .route(
            "/api/incremental/logs",
            get(incremental_update_handlers::get_increment_logs),
        )
        .route(
            "/api/incremental/history",
            get(incremental_update_handlers::get_sync_history_paged),
        )
        .route(
            "/api/incremental/archives",
            get(incremental_update_handlers::list_cba_files),
        )
        .route(
            "/api/incremental/stats",
            get(incremental_update_handlers::get_incremental_stats),
        )
        // MQTT 节点监控 API
        .route(
            "/api/mqtt/nodes",
            get(mqtt_monitor_handlers::get_mqtt_nodes_status),
        )
        .route(
            "/api/mqtt/nodes/{location}",
            delete(mqtt_monitor_handlers::remove_mqtt_node),
        )
        .route(
            "/api/mqtt/nodes/client-unsubscribed",
            post(mqtt_monitor_handlers::client_unsubscribed),
        )
        .route(
            "/api/mqtt/messages",
            get(mqtt_monitor_handlers::get_message_delivery_status),
        )
        .route(
            "/api/mqtt/messages/{message_id}",
            get(mqtt_monitor_handlers::get_message_delivery_detail),
        )
        // 增量更新页面
        .route("/incremental", get(serve_incremental_update_page))
        .route("/incremental-vue", get(serve_incremental_update_vue_page))
        // 同步控制中心
        .route(
            "/sync-control",
            get(sync_control_handlers::sync_control_page),
        )
        .route(
            "/api/sync/start",
            post(sync_control_handlers::start_sync_service),
        )
        .route(
            "/api/sync/stop",
            post(sync_control_handlers::stop_sync_service),
        )
        .route(
            "/api/sync/restart",
            post(sync_control_handlers::restart_sync_service),
        )
        .route(
            "/api/sync/pause",
            post(sync_control_handlers::pause_sync_service),
        )
        .route(
            "/api/sync/resume",
            post(sync_control_handlers::resume_sync_service),
        )
        .route(
            "/api/sync/status",
            get(sync_control_handlers::get_sync_status),
        )
        .route(
            "/api/sync/events",
            get(sync_control_handlers::sync_events_stream),
        )
        .route(
            "/api/sync/metrics",
            get(sync_control_handlers::get_sync_metrics),
        )
        .route(
            "/api/sync/metrics/history",
            get(sync_control_handlers::get_sync_metrics_history),
        )
        .route(
            "/api/sync/queue",
            get(sync_control_handlers::get_sync_queue),
        )
        .route(
            "/api/sync/queue/clear",
            post(sync_control_handlers::clear_sync_queue),
        )
        .route(
            "/api/sync/config",
            get(sync_control_handlers::get_sync_config),
        )
        .route(
            "/api/sync/config",
            put(sync_control_handlers::update_sync_config),
        )
        .route(
            "/api/sync/test",
            post(sync_control_handlers::test_sync_connection),
        )
        .route("/api/sync/task", post(sync_control_handlers::add_sync_task))
        .route(
            "/api/sync/trigger-download",
            post(sync_control_handlers::trigger_file_download),
        )
        .route(
            "/api/sync/task/{id}/cancel",
            post(sync_control_handlers::cancel_sync_task),
        )
        .route(
            "/api/sync/history",
            get(sync_control_handlers::get_sync_history),
        )
        // SSE 事件流（使用独立路径避免与轮询接口冲突）
        .route(
            "/api/sync/events/stream",
            get(sse_handlers::sync_events_handler),
        )
        .route("/api/sync/events/test", get(sse_handlers::test_sse_handler))
        .route(
            "/api/sync/mqtt/start",
            post(sync_control_handlers::start_mqtt_server_api),
        )
        .route(
            "/api/sync/mqtt/stop",
            post(sync_control_handlers::stop_mqtt_server_api),
        )
        .route(
            "/api/sync/mqtt/status",
            get(sync_control_handlers::get_mqtt_server_status),
        )
        .route(
            "/api/mqtt/broker/logs",
            get(sync_control_handlers::get_mqtt_broker_logs_api),
        )
        // MQTT 订阅客户端控制
        .route(
            "/api/mqtt/subscription/start",
            post(sync_control_handlers::start_mqtt_subscription_api),
        )
        .route(
            "/api/mqtt/subscription/stop",
            post(sync_control_handlers::stop_mqtt_subscription_api),
        )
        .route(
            "/api/mqtt/subscription/clear-master-config",
            post(sync_control_handlers::clear_master_config_api),
        )
        .route(
            "/api/mqtt/subscription/status",
            get(sync_control_handlers::get_mqtt_subscription_status),
        )
        .route(
            "/api/mqtt/subscription/status/stream",
            get(sse_handlers::mqtt_subscription_status_stream_handler),
        )
        // 节点角色管理
        .route(
            "/api/mqtt/node/set-master",
            post(sync_control_handlers::set_as_master_node),
        )
        .route(
            "/api/mqtt/node/set-client",
            post(sync_control_handlers::set_as_client_node),
        )
        // 站点配置管理
        .route(
            "/api/site-config",
            get(site_config_handlers::get_site_config),
        )
        .route(
            "/api/site/info",
            get(site_config_handlers::get_site_info),
        )
        .route(
            "/api/site-config/save",
            post(site_config_handlers::save_site_config),
        )
        .route(
            "/api/site-config/validate",
            post(site_config_handlers::validate_site_config),
        )
        .route(
            "/api/site-config/reload",
            post(site_config_handlers::reload_site_config),
        )
        .route(
            "/api/site-config/restart",
            post(site_config_handlers::restart_server),
        )
        .route(
            "/api/site-config/server-ip",
            get(site_config_handlers::get_server_ip),
        )
        // 异地增量环境配置页面 + API
        .route("/remote-sync", get(remote_sync_handlers::remote_sync_page))
        .route(
            "/api/remote-sync/envs",
            get(remote_sync_handlers::list_envs).post(remote_sync_handlers::create_env),
        )
        .route(
            "/api/remote-sync/envs/{id}",
            get(remote_sync_handlers::get_env)
                .put(remote_sync_handlers::update_env)
                .delete(remote_sync_handlers::delete_env),
        )
        .route(
            "/api/remote-sync/envs/{id}/apply",
            post(remote_sync_handlers::apply_env),
        )
        .route(
            "/api/remote-sync/envs/{id}/activate",
            post(remote_sync_handlers::activate_env),
        )
        .route(
            "/api/remote-sync/runtime/stop",
            post(remote_sync_handlers::stop_runtime),
        )
        // .route("/api/remote-sync/envs/{id}/test-mqtt", post(remote_sync_handlers::test_mqtt_env))
        // .route("/api/remote-sync/envs/{id}/test-http", post(remote_sync_handlers::test_http_env))
        // .route("/api/remote-sync/sites/{id}/test-http", post(remote_sync_handlers::test_http_site))
        .route(
            "/api/remote-sync/runtime/status",
            get(remote_sync_handlers::runtime_status),
        )
        .route(
            "/api/remote-sync/runtime/config",
            get(remote_sync_handlers::runtime_config),
        )
        .route(
            "/api/remote-sync/envs/import-from-dboption",
            post(remote_sync_handlers::import_env_from_dboption),
        )
        .route(
            "/api/remote-sync/batch-import",
            post(remote_sync_handlers::batch_import),
        )
        // 站点配置导出（供其他站点通过 URL 导入）
        .route(
            "/api/site-config/export",
            get(remote_sync_handlers::export_site_config),
        )
        // 从远程 URL 导入站点配置
        .route(
            "/api/remote-sync/import-from-url",
            post(remote_sync_handlers::import_site_from_url),
        )
        .route(
            "/api/remote-sync/logs",
            get(remote_sync_handlers::list_logs),
        )
        .route(
            "/api/remote-sync/stats/daily",
            get(remote_sync_handlers::daily_stats),
        )
        .route(
            "/api/remote-sync/stats/flows",
            get(remote_sync_handlers::flow_stats),
        )
        .route(
            "/api/remote-sync/envs/{id}/sites",
            get(remote_sync_handlers::list_sites).post(remote_sync_handlers::create_site),
        )
        .route(
            "/api/remote-sync/sites/{id}",
            put(remote_sync_handlers::update_site).delete(remote_sync_handlers::delete_site),
        )
        .route(
            "/api/remote-sync/sites/{id}/metadata",
            get(remote_sync_handlers::get_site_metadata),
        )
        .route(
            "/api/remote-sync/sync-site",
            post(remote_sync_handlers::sync_site_api),
        )
        .route(
            "/api/remote-sync/update-master-config",
            post(remote_sync_handlers::update_master_config_api),
        )
        .route(
            "/api/remote-sync/sites/{id}/files/{*path}",
            get(remote_sync_handlers::serve_site_files),
        )
        // 拓扑配置 API
        .route(
            "/api/remote-sync/topology",
            get(topology_handlers::get_topology)
                .post(topology_handlers::save_topology)
                .delete(topology_handlers::delete_topology),
        )
        .route(
            "/api/remote-sync/sites/{id}/files",
            get(remote_sync_handlers::serve_site_files_root),
        )
        // 健康检查 API
        .route("/api/health", get(litefs_handlers::health_check))
        // 数据库状态管理API
        .route(
            "/api/database/status",
            get(database_status_handlers::get_all_database_status),
        )
        .route(
            "/api/database/{db_num}/details",
            get(database_status_handlers::get_database_details),
        )
        .route(
            "/api/database/{db_num}/parse",
            post(database_status_handlers::reparse_database),
        )
        .route(
            "/api/database/{db_num}/generate",
            post(database_status_handlers::regenerate_model),
        )
        .route(
            "/api/database/{db_num}/update",
            post(database_status_handlers::trigger_database_update),
        )
        .route(
            "/api/database/{db_num}/clear-cache",
            post(database_status_handlers::clear_database_cache),
        )
        .route(
            "/api/database/batch",
            post(database_status_handlers::execute_batch_operation),
        )
        .route(
            "/api/database/modules",
            get(database_status_handlers::get_module_list),
        )
        // 数据库状态页面
        .route("/database-status", get(serve_database_status_page))
        // 数据库状态管理API
        .route(
            "/api/db-status",
            get(db_status_handlers::get_db_status_list),
        )
        .route(
            "/api/db-status/{dbnum}",
            get(db_status_handlers::get_db_status_detail),
        )
        .route(
            "/api/db-status/update",
            post(db_status_handlers::execute_incremental_update),
        )
        .route(
            "/api/db-status/check-versions",
            get(db_status_handlers::check_file_versions),
        )
        .route(
            "/api/db-status/{dbnum}/auto-update-type",
            post(db_status_handlers::set_auto_update_type),
        )
        .route(
            "/api/db-status/{dbnum}/auto-update",
            post(db_status_handlers::set_auto_update),
        )
        // 本地扫描与同步
        .route(
            "/api/db-sync/scan",
            get(db_status_handlers::scan_local_files),
        )
        .route(
            "/api/db-sync/sync",
            post(db_status_handlers::sync_file_metadata),
        )
        .route(
            "/api/db-sync/rescan",
            post(db_status_handlers::rescan_and_cache),
        )
        // 项目管理 API（最小集：列表 + 创建）
        .route(
            "/api/projects",
            get(handlers::api_get_projects).post(handlers::api_create_project),
        )
        .route(
            "/api/projects/{id}",
            get(handlers::api_get_project)
                .put(handlers::api_update_project)
                .delete(handlers::api_delete_project),
        )
        .route("/api/projects/demo", post(handlers::api_projects_demo))
        .route(
            "/api/projects/{id}/healthcheck",
            post(handlers::api_healthcheck_project),
        )
        // 部署站点管理 API
        .route(
            "/api/deployment-sites/import-dboption",
            post(handlers::api_import_deployment_site_from_dboption),
        )
        .route(
            "/api/deployment-sites",
            get(handlers::api_get_deployment_sites).post(handlers::api_create_deployment_site),
        )
        .route(
            "/api/deployment-sites/{id}",
            get(handlers::api_get_deployment_site)
                .put(handlers::api_update_deployment_site)
                .delete(handlers::api_delete_deployment_site),
        )
        // .route(
        //     "/api/deployment-sites/{id}/browse-directory",
        //     get(handlers::api_browse_deployment_site_directory),
        // )
        .route(
            "/api/deployment-sites/{id}/tasks",
            post(handlers::api_create_deployment_site_task),
        )
        // .route(
        //     "/api/deployment-sites/{id}/healthcheck",
        //     post(handlers::api_healthcheck_deployment_site_post),
        // )
        .route(
            "/api/deployment-sites/{id}/export-config",
            get(handlers::api_export_deployment_site_config),
        )
        // 部署站点管理页面
        .route("/deployment-sites", get(handlers::deployment_sites_page))
        // 数据解析向导API
        .route(
            "/api/wizard/scan-directory",
            get(wizard_handlers::scan_directory),
        )
        .route(
            "/api/wizard/scan-database-files",
            get(wizard_handlers::scan_database_files),
        )
        .route(
            "/api/wizard/list-projects",
            get(wizard_handlers::list_projects),
        )
        .route(
            "/api/wizard/create-task",
            post(wizard_handlers::create_wizard_task),
        )
        .route(
            "/api/wizard/templates",
            get(wizard_handlers::get_wizard_templates),
        )
        .route(
            "/api/wizard/browse-directory",
            get(wizard_handlers::browse_directory),
        )
        // 任务创建API - 使用不同的路径避免冲突
        .route(
            "/api/task-creation",
            post(task_creation_handlers::create_task),
        )
        .route(
            "/api/task-templates",
            get(task_creation_handlers::get_task_templates),
        )
        .route(
            "/api/task-creation/validate-name",
            get(task_creation_handlers::validate_task_name),
        )
        .route(
            "/api/task-creation/preview",
            post(task_creation_handlers::preview_task_config),
        )
        // SQLite 空间索引 API
        .route(
            "/api/sqlite-spatial/rebuild",
            post(handlers::api_sqlite_spatial_rebuild),
        )
        .route(
            "/api/sqlite-spatial/query",
            get(handlers::api_sqlite_spatial_query),
        )
        // 空间查询页面
        .route("/spatial-query", get(handlers::spatial_query_page))
        // 空间计算 API
        .route(
            "/api/space/suppo-trays",
            post(handlers::api_space_suppo_trays),
        )
        .route("/api/space/fitting", post(handlers::api_space_fitting))
        .route(
            "/api/space/wall-distance",
            post(handlers::api_space_wall_distance),
        )
        .route(
            "/api/space/fitting-offset",
            post(handlers::api_space_fitting_offset),
        )
        .route(
            "/api/space/steel-relative",
            post(handlers::api_space_steel_relative),
        )
        .route("/api/space/tray-span", post(handlers::api_space_tray_span))
        // 模型导出 API
        .route("/api/export/gltf", post(handlers::create_export_task))
        .route("/api/export/glb", post(handlers::create_export_task))
        .route(
            "/api/export/status/{task_id}",
            get(handlers::get_export_status),
        )
        .route(
            "/api/export/download/{task_id}",
            get(handlers::download_export),
        )
        .route("/api/export/tasks", get(handlers::list_export_tasks))
        .route("/api/export/cleanup", post(handlers::cleanup_export_tasks))
        // 静态文件服务 - 使用智能路径解析
        .nest_service("/static", ServeDir::new(resolve_static_dir()))
        .nest_service("/files/output", ServeDir::new("output"))
        // CBA 文件分发服务 - 用于远程站点下载增量数据包（使用智能路径解析）
        // 使用 nest 嵌套路由：根路径显示目录列表，子路径返回文件内容
        .nest("/assets/archives", axum::Router::new()
            .route("/", get(incremental_update_handlers::serve_archives_root))
            .route("/{*path}", get(incremental_update_handlers::serve_archives_file))
        )
        // 主页面
        .route("/", get(index_page))
        .route("/admin", get(handlers::admin_page))
        .route("/dashboard", get(dashboard_page))
        .route("/config", get(config_page))
        .route("/tasks", get(tasks_page))
        .route("/tasks/{id}", get(task_detail_page))
        .route("/tasks/{id}/logs", get(task_logs_page))
        .route("/batch-tasks", get(batch_tasks_page))
        .route("/db-status", get(db_status_page))
        .route("/wizard", get(wizard_page))
        .route("/space-tools", get(space_tools_page))
        .route("/sqlite-spatial", get(handlers::sqlite_spatial_page))
        .route(
            "/database-connection",
            get(handlers::database_connection_page),
        )
        // 桥架支撑检测页面 + API
        .route("/tray-supports", get(handlers::tray_supports_page))
        .route(
            "/api/sqlite-tray-supports/detect",
            post(handlers::api_sqlite_tray_supports_detect),
        )
        // SCTN 测试流程（后台任务 + 进度 + 结果）
        .route("/sctn-test", get(handlers::sctn_test_page))
        .route("/api/sctn-test/run", post(handlers::api_sctn_test_run))
        .route(
            "/api/sctn-test/result/{id}",
            get(handlers::api_sctn_test_result),
        )
        // 空间查询可视化页面
        .route(
            "/spatial-visualization",
            get(handlers::spatial_visualization_page),
        )
        // 房间计算管理页面
        .route("/room-management", get(room_page::room_management_page))
        // WebSocket 路由
        .route("/ws/progress/{task_id}", get(ws::ws_progress_handler))
        .route("/ws/tasks", get(ws::ws_tasks_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(Any),
        )
        .with_state(app_state.clone())
        .merge(spatial_query_routes)
        .merge(noun_hierarchy_routes)
        .merge(room_routes);

    // 监听指定地址，默认 0.0.0.0（所有网络接口）
    let bind_host = host.as_deref().unwrap_or("0.0.0.0");
    let bind_addr = format!("{}:{}", bind_host, port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    // 根据绑定地址选择显示的IP
    let display_ip = if bind_host == "0.0.0.0" {
        // 绑定所有接口时，显示本机实际IP
        get_local_ip_via_udp().unwrap_or_else(|_| "127.0.0.1".to_string())
    } else {
        // 指定了具体host时，显示该host
        bind_host.to_string()
    };

    println!("🚀 Web UI服务器启动成功！");
    println!("📱 访问地址: http://{}:{}", display_ip, port);
    println!("🎯 功能包括:");
    println!("   - 数据库生成任务管理");
    println!("   - 实时进度监控");
    println!("   - 配置管理");
    println!("   - 任务历史记录");
    // 后台自动更新扫描任务（基于 auto_update + sesno 比较）
    // 注释掉自动调度器，因为数据库服务由配置管理
    // 先确保 SurrealDB 的表结构字段齐备（在生产环境中便于统一管理）
    // crate::web_server::db_status_handlers::ensure_dbnum_info_schema().await;
    // tokio::spawn(auto_update_scheduler(app_state.clone()));

    // 周期性项目健康检查（可通过 WEBUI_HEALTH_SCHED=0 关闭）
    // 也注释掉，避免启动时查询数据库
    // tokio::spawn(crate::web_server::handlers::projects_health_scheduler());

    // 使用 graceful shutdown
    let shutdown = async {
        shutdown_rx.await.ok();
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

async fn auto_update_scheduler(state: AppState) {
    use crate::web_server::models::{IncrementalUpdateRequest, UpdateType};
    use aios_core::SUL_DB;
    use axum::{Json, extract::State as AxumState};
    use std::time::Duration;

    loop {
        // 每60秒扫描一次
        tokio::time::sleep(Duration::from_secs(60)).await;

        // 读取 auto_update 的记录
        let sql = "SELECT dbnum, file_name, sesno, project, auto_update, updating FROM dbnum_info_table WHERE auto_update = true";
        let rows = match SUL_DB.query(sql).await {
            Ok(mut resp) => resp.take::<Vec<serde_json::Value>>(0).unwrap_or_default(),
            Err(_) => continue,
        };

        for row in rows {
            let dbnum = row["dbnum"].as_u64().unwrap_or(0) as u32;
            let project = row["project"].as_str().unwrap_or("");
            let updating = row["updating"].as_bool().unwrap_or(false);

            // 计算是否需要更新
            let cached_sesno = crate::fast_model::session::SESSION_STORE
                .get_max_sesno_for_dbnum(dbnum)
                .unwrap_or(0);
            let latest_file_sesno = {
                // TODO: Implement proper PDMS sesno extraction
                // This requires creating PdmsIO from project directory
                0
            };
            let needs_update = cached_sesno < latest_file_sesno;

            if needs_update && !updating {
                // 读取更新类型
                let typ = row["auto_update_type"].as_str().unwrap_or("ParseAndModel");
                let update_type = match typ {
                    "ParseOnly" => UpdateType::ParseOnly,
                    "Full" => UpdateType::Full,
                    _ => UpdateType::ParseAndModel,
                };
                // 构造并发起增量更新（解析+建模）
                let req = IncrementalUpdateRequest {
                    dbnums: vec![dbnum],
                    force_update: false,
                    update_type,
                    target_sesno: None,
                };
                let _ = crate::web_server::handlers::execute_incremental_update(
                    AxumState(state.clone()),
                    Json(req),
                )
                .await;
            }
        }
    }
}

/// 查询参数
#[derive(Deserialize)]
pub struct TaskQuery {
    pub status: Option<String>,
    pub limit: Option<usize>,
}

/// 创建任务请求
#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub task_type: TaskType,
    pub config: DatabaseConfig,
}

/// 更新配置请求
#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub config: DatabaseConfig,
}
