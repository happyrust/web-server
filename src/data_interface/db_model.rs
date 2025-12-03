use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use aios_core::SUL_DB;
use aios_core::accel_tree::acceleration_tree::{AccelerationTree, RStarBoundingBox};
use aios_core::file_helper::collect_db_dirs;
use aios_core::get_db_option;
use aios_core::options::DbOption;
use aios_core::pdms_types::*;
use dashmap::DashMap;
use futures::StreamExt;
use glam::Vec3;
use indexmap::IndexMap;
use itertools::Itertools;
use log::{error, info};
use once_cell::sync::Lazy;
use parry3d::bounding_volume::{Aabb, BoundingVolume};
use parry3d::math::Vector;
use parry3d::query::{Ray, RayCast};
use pdms_io::io::PdmsIO;
use pdms_io::sync::clone::{CloneOptions, execute_clone};
use pdms_io::watch::PdmsWatcher;
use rayon::prelude::*;
use reqwest::Client;
use rumqttc::Event::Incoming;
use rumqttc::{Packet, QoS};

#[cfg(feature = "sql")]
use sqlx::pool::PoolOptions;
#[cfg(feature = "sql")]
use sqlx::{Executor, MySql, MySqlPool, Pool, Row};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::consts::*;
use crate::data_interface::failed_task_queue::FailedTaskQueue;
use crate::data_interface::interface::PdmsDataInterface;
use crate::data_interface::sesno_cache::SesnoCache;
use crate::data_interface::tidb_manager::AiosDBManager;
use crate::defines::CACHED_MDB_SITE_MAP;
use crate::mqtt_service::{SyncE3dFileMsg, new_mqtt_inst, new_mqtt_inst_with_config};

pub const TUBI_TOL: f32 = 0.1f32;

// project + mdb + module
pub static GLOBAL_MDB_WORLD_MAP: Lazy<DashMap<String, PdmsElement>> = Lazy::new(DashMap::new);

static PDMS_GNERAL_TYPE_NAMES_MAP: Lazy<HashMap<&'static str, PdmsGenericType>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("EQUI", PdmsGenericType::EQUI);
    m.insert("PIPE", PdmsGenericType::PIPE);
    m.insert("ROOM", PdmsGenericType::ROOM);
    m.insert("STRU", PdmsGenericType::STRU);
    m.insert("PANE", PdmsGenericType::PANE);
    m.insert("HANG", PdmsGenericType::HANG);
    m.insert("WALL", PdmsGenericType::WALL);
    m.insert("GWALL", PdmsGenericType::WALL);
    m.insert("CWALL", PdmsGenericType::WALL);
    m.insert("STWALL", PdmsGenericType::WALL);
    m.insert("CFLOOR", PdmsGenericType::CFLOOR);
    m.insert("FLOOR", PdmsGenericType::FLOOR);
    m.insert("EXTR", PdmsGenericType::EXTR);
    m.insert("REVO", PdmsGenericType::REVO);
    m
});

//创建一个监控mqtt是否连接的全局变量,使用Mutex<bool>
pub static MQTT_CONNECT_STATUS: Lazy<Mutex<Option<bool>>> = Lazy::new(|| Mutex::new(None));

/// 全局 MQTT Publisher Client（由 start_mqtt_publisher 初始化）
/// 用于在 increment_manager 中发布消息
#[cfg(feature = "mqtt")]
pub static MQTT_PUBLISHER_CLIENT: Lazy<tokio::sync::RwLock<Option<Arc<rumqttc::AsyncClient>>>> =
    Lazy::new(|| tokio::sync::RwLock::new(None));

impl AiosDBManager {
    /// 从默认配置文件初始化
    pub async fn init_form_config() -> anyhow::Result<Self> {
        let db_option = get_db_option();
        let mut mgr = Self::init(&db_option).await?;
        Ok(mgr)
    }

    /// 启动 MQTT 发布器的 EventLoop
    /// 当需要通过 MQTT 发布消息时调用此方法
    /// 返回 JoinHandle 以便外部管理任务生命周期
    /// 
    /// 注意：此方法创建一个新的 MQTT 连接，客户端存储在 MQTT_PUBLISHER_CLIENT 全局变量中，
    /// increment_manager 应使用该全局变量而非 self.mqtt_client 进行发布
    #[cfg(feature = "mqtt")]
    pub fn start_mqtt_publisher(&self) -> tokio::task::JoinHandle<()> {
        let db_option = get_db_option();
        // 使用唯一的 client ID 避免与其他连接冲突
        let client_id = format!(
            "{}-{}-pub-active",
            db_option.location.as_str(),
            db_option.project_code
        );
        let mut mqtt_inst = new_mqtt_inst(&client_id);
        
        // 存储 client 到全局变量，供 increment_manager 使用
        let client_arc = Arc::new(mqtt_inst.client);
        let client_for_global = client_arc.clone();
        
        tokio::task::spawn(async move {
            // 将 client 存入全局变量
            {
                let mut guard = MQTT_PUBLISHER_CLIENT.write().await;
                *guard = Some(client_for_global);
                info!("[Publisher] MQTT client stored in global variable");
            }
            
            loop {
                let event = mqtt_inst.el.poll().await;
                match event {
                    Ok(event) => match event {
                        rumqttc::Event::Incoming(Packet::ConnAck(_)) => {
                            let mut mqtt_connect_status = MQTT_CONNECT_STATUS.lock().await;
                            if mqtt_connect_status.is_none() {
                                *mqtt_connect_status = Some(true);
                                info!("[Publisher] Init connected to MQTT broker.");
                            } else if !(*mqtt_connect_status).unwrap() {
                                *mqtt_connect_status = Some(true);
                                info!("[Publisher] Connected to MQTT broker.");
                            }
                        }
                        _ => {}
                    },
                    Err(e) => {
                        let mut mqtt_connect_status = MQTT_CONNECT_STATUS.lock().await;
                        if mqtt_connect_status.is_none() {
                            *mqtt_connect_status = Some(false);
                            error!("[Publisher] Init MQTT Connection error: {}", e);
                        } else if (*mqtt_connect_status).unwrap() {
                            *mqtt_connect_status = Some(false);
                            error!("[Publisher] MQTT Connection error: {}", e);
                        }
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        })
    }

    //初始化watcher
    pub async fn exec_watcher(mgr: Arc<AiosDBManager>) -> anyhow::Result<()> {
        mgr.init_watcher().await.unwrap();
        mgr.async_watch().await.unwrap();
        Ok(())
    }

    //开启定时同步更新任务
    pub async fn run_e3d_clone_bg_task(mgr: Arc<AiosDBManager>) -> anyhow::Result<()> {
        dbg!("定时同步数据任务开启");
        let forever = tokio::spawn(async move {
            //10分钟强制刷一遍
            let mut interval = tokio::time::interval(Duration::from_secs(60 * 10));
            loop {
                interval.tick().await;
                //todo，需要配置各个db对应的映射, 不同区域对应不同的db
                // Self::exec_delta_clone_remotes(&mgr.watcher, &[]).await.unwrap();
            }
        });
        forever.await?
    }

    //增量从服务器里的数据clone到本地
    pub async fn exec_delta_clone_remotes(
        watcher: &PdmsWatcher,
        sync_msg: SyncE3dFileMsg,
    ) -> anyhow::Result<bool> {
        if sync_msg.file_names.is_empty() {
            return Ok(false);
        }
        let loc_dbs = &get_db_option().location_dbs;
        let remote_url = sync_msg.file_server_host.as_str();
        let client = Client::new();
        for file_name in sync_msg.file_names.iter() {
            let url = format!("{}/{}.cba", remote_url, file_name);
            dbg!(&file_name);
            //todo 如果没有需要新加数据
            let pb = if let Some(pb) = watcher.get_db_path(file_name) {
                pb
            } else {
                // 如果找不到文件名, 就跳过
                println!("File {} not found in db_path_map, skip", file_name);
                continue;
            };
            dbg!(&pb);

            // 检查 location_dbs 过滤：必须不是当前区域的 db 才能 clone
            // location_dbs 是当前站点专有的数据库，不允许外部修改
            let dbno = if let Some(dbno) = watcher.get_dbno(&pb) {
                dbno
            } else {
                // 如果 headers 中没有，直接从文件读取 dbnum
                match PdmsIO::new("", pb.clone(), true).get_page_basic_info() {
                    Ok(info) => info.pdms_header.db_num as u32,
                    Err(e) => {
                        println!("⚠️ 无法读取文件 {} 的 dbnum: {}, 跳过", file_name, e);
                        continue;
                    }
                }
            };
            dbg!(dbno);
            
            // 跳过当前区域的 dbnos (location_dbs 中的文件不允许被外部修改)
            if let Some(dbs) = loc_dbs {
                if dbs.contains(&dbno) {
                    println!("⏭️ 跳过 {} (DB#{})：在 location_dbs 中，不允许外部修改", file_name, dbno);
                    continue;
                }
            }

            println!(
                "Start delta clone db files num: {} from {}",
                sync_msg.file_names.len(),
                &url
            );
            let e3d_file: PathBuf = pb.clone();
            let tmp_dir = std::env::temp_dir().join("e3d_sync_cba");
            if let Err(e) = tokio::fs::create_dir_all(&tmp_dir).await {
                println!("Create temp dir for {} failed: {}", file_name, e);
                continue;
            }
            let tmp_file = tmp_dir.join(format!("{}.cba", file_name));
            if let Err(e) = download_cba(&client, &url, &tmp_file).await {
                println!("Download {} failed: {}", file_name, e);
                continue;
            }
            // 注意：不再手动校验哈希，dpcsync 的 execute_clone 会自动验证完整性

            let mut clone_time = Instant::now();
            let clone_opt = CloneOptions::new_local(
                tmp_file
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid temp path for {}", file_name))?,
                e3d_file,
            );
            match execute_clone(clone_opt).await {
                Ok(r) => {
                    if r {
                        //需要保存更新记录
                        println!(
                            "Clone {} cost: {:?}s",
                            file_name,
                            clone_time.elapsed().as_secs_f64()
                        );
                        //clone完了,再执行增量更新
                    } else {
                        println!("Clone {} returned false", file_name);
                    }
                }
                Err(e) => {
                    println!("Clone {} failed: {}", file_name, e);
                }
            }

            // 清理临时文件
            if let Err(e) = tokio::fs::remove_file(&tmp_file).await {
                eprintln!("⚠️ 清理临时文件失败: {}, 错误: {:?}", tmp_file.display(), e);
            }
        }

        Ok(true)
    }

    #[cfg(test)]
    #[tokio::test]
    async fn test_debug_clone() {
        use pdms_io::sync::clone::{CloneOptions, execute_clone};

        // 测试本地文件克隆
        let source_file =
            "/Volumes/DPC/work/e3d_models/test_sjz/AvevaMarineSample/ams000/test1112.cba";
        let target_file =
            "/Volumes/DPC/work/e3d_models/test_sjz/AvevaMarineSample/ams000/test1112_debug";

        println!(
            "Testing local clone from {} to {}",
            source_file, target_file
        );

        let clone_opt = CloneOptions::new_local(source_file, target_file);

        match execute_clone(clone_opt).await {
            Ok(success) => {
                if success {
                    println!("Clone successful!");
                } else {
                    println!("Clone returned false");
                }
            }
            Err(e) => {
                println!("Clone failed: {}", e);
                panic!("Clone failed: {}", e);
            }
        }
    }

    pub async fn spawn_exec_watcher(mgr: Arc<AiosDBManager>) -> anyhow::Result<()> {
        let f = tokio::spawn(async move {
            mgr.init_watcher().await.unwrap();
            mgr.async_watch().await.unwrap();
        });
        Ok(f.await?)
    }

    pub async fn demo_mqtt_requests() {
        let mut mqtt_inst = new_mqtt_inst("test-1");
        let client = mqtt_inst.client.clone();
        let f = tokio::spawn(async move {
            for i in 1..=10000 {
                let test_data = SyncE3dFileMsg {
                    file_names: vec![format!("Hello-{}", i)],
                    file_hashes: vec![],
                    file_server_host: "http://50c170h624.zicp.vip:56785/assets/archives"
                        .to_string(),
                    location: "bj".to_string(),
                    timestamp: Default::default(),
                    session_range: None,
                    total_added: None,
                    total_modified: None,
                    total_deleted: None,
                    is_full_sync: None,
                    db_num: None,
                };
                let _ = client
                    .publish("Sync/E3d", QoS::ExactlyOnce, false, test_data)
                    .await
                    .unwrap();

                dbg!(i);

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            // tokio::time::sleep(Duration::from_secs(120)).await;
        });

        loop {
            let event = mqtt_inst.el.poll().await;
        }

        f.await.expect("demo_mqtt_requests panic");
    }

    ///另外将里面可能有关联的db，也要同步检查后一下？？
    ///处理mqtt的消息, 通知需要处理的db 文件名，然后对应的归属地也需要发送
    pub async fn poll_sync_e3d_mqtt_events(watcher: Arc<PdmsWatcher>) {
        let db_option = get_db_option();
        let location = db_option.location.clone();
        let f = tokio::spawn(async move {
            //订阅消息处理更新
            let mut mqtt_inst = new_mqtt_inst(&format!(
                "{}-{}-sub",
                db_option.location.as_str(),
                db_option.project_code
            ));
            let mut heartbeat_handle: Option<tokio::task::JoinHandle<()>> = None;
            mqtt_inst
                .client
                .subscribe("Sync/E3d", QoS::ExactlyOnce)
                .await
                .unwrap();
            // 连接超时已在 MqttOptions 上配置，无需再从 EventLoop 访问网络选项
            loop {
                let event = mqtt_inst.el.poll().await;
                match &event {
                    Ok(v) => {
                        match v {
                            Incoming(Packet::Publish(p)) => {
                                let sync_e3d = SyncE3dFileMsg::from(p.payload.to_vec());

                                // 详细日志：显示接收到的消息信息
                                println!(
                                    "📥 [目标节点: {}] 收到 MQTT 消息 (来源节点: {}, 文件数: {}, 会话范围: {:?}, 文件列表: [{}])",
                                    location,
                                    sync_e3d.location,
                                    sync_e3d.file_names.len(),
                                    sync_e3d.session_range,
                                    sync_e3d.file_names.join(", ")
                                );

                                // 记录消息接收（用于 MQTT 监控）
                                #[cfg(feature = "web_server")]
                                {
                                    use crate::web_server::mqtt_monitor_handlers;
                                    let message_id =
                                        format!("{:?}_{}", sync_e3d.timestamp, sync_e3d.location);
                                    let message_id_clone = message_id.clone();
                                    mqtt_monitor_handlers::record_message_received(
                                        location.clone(),
                                        message_id,
                                    )
                                    .await;
                                    println!(
                                        "📊 [目标节点: {}] 已记录消息接收状态到监控系统 (消息ID: {})",
                                        location, message_id_clone
                                    );
                                }

                                //检查是否和本地的location一致，如果不一致，才发生更新
                                if sync_e3d.location != location {
                                    println!(
                                        "✅ [目标节点: {}] 消息来源不同，开始处理增量更新 (来源: {})",
                                        location, sync_e3d.location
                                    );

                                    //自己本地也要保存, todo 后续还是要配置哪些dbs，哪个地方能修改，哪个地方是不能改的
                                    SUL_DB
                                        .query(format!(
                                            "INSERT IGNORE INTO e3d_sync {} ",
                                            serde_json::to_string(&sync_e3d).unwrap()
                                        ))
                                        .await
                                        .unwrap();

                                    //执行指定文件的clone
                                    match Self::exec_delta_clone_remotes(&watcher, sync_e3d).await {
                                        Ok(_) => {
                                            println!(
                                                "✅ [目标节点: {}] 增量更新处理完成",
                                                location
                                            );
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "❌ [目标节点: {}] 增量更新处理失败: {:?}",
                                                location, e
                                            );
                                        }
                                    }
                                } else {
                                    println!(
                                        "⏭️  [目标节点: {}] 消息来源相同，跳过处理 (来源: {})",
                                        location, sync_e3d.location
                                    );
                                }
                            }
                            _ => {
                                // dbg!(v);
                            }
                        }
                    }
                    Err(e) => {
                        // println!("Error = {e:?}");
                        // return Ok(());
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                    _ => {}
                }
            }
        });
        f.await.expect("demo_mqtt_requests panic");
    }

    ///处理mqtt的消息，带重连退避（单位ms）
    pub async fn poll_sync_e3d_mqtt_events_with_backoff(
        watcher: Arc<PdmsWatcher>,
        initial_backoff_ms: u64,
        max_backoff_ms: u64,
    ) {
        let db_option = get_db_option();
        let location = db_option.location.clone();
        let mut backoff = initial_backoff_ms.max(100);
        let max_backoff = max_backoff_ms.max(backoff);

        loop {
            // 构造新的连接实例
            let mut mqtt_inst = new_mqtt_inst(&format!(
                "{}-{}-sub",
                db_option.location.as_str(),
                db_option.project_code
            ));
            let _ = mqtt_inst
                .client
                .subscribe("Sync/E3d", QoS::ExactlyOnce)
                .await;

            // 初始标记订阅未连接，等待 ConnAck 后再置为已连接并启动心跳
            #[cfg(feature = "web_server")]
            let mut heartbeat_handle: Option<tokio::task::JoinHandle<()>> = None;
            #[cfg(feature = "web_server")]
            {
                let location_clone = location.clone();
                let node_name = format!("{}-{}", db_option.location, db_option.project_code);
                use crate::web_server::mqtt_monitor_handlers;
                mqtt_monitor_handlers::update_subscription_status(
                    location_clone.clone(),
                    node_name.clone(),
                    false,
                )
                .await;
            }
            // 轮询事件，直到错误发生
            loop {
                let event = mqtt_inst.el.poll().await;
                match &event {
                    Ok(v) => match v {
                        Incoming(Packet::ConnAck(_)) => {
                            #[cfg(feature = "web_server")]
                            {
                                let location_clone = location.clone();
                                let node_name =
                                    format!("{}-{}", db_option.location, db_option.project_code);
                                use crate::web_server::mqtt_monitor_handlers;
                                mqtt_monitor_handlers::update_subscription_status(
                                    location_clone.clone(),
                                    node_name.clone(),
                                    true,
                                )
                                .await;

                                let location_for_heartbeat = location_clone.clone();
                                let node_name_for_heartbeat = node_name.clone();
                                if heartbeat_handle.is_none() {
                                    heartbeat_handle = Some(tokio::spawn(async move {
                                        let mut interval =
                                            tokio::time::interval(Duration::from_secs(10));
                                        loop {
                                            interval.tick().await;
                                            mqtt_monitor_handlers::update_node_heartbeat(
                                                location_for_heartbeat.clone(),
                                                node_name_for_heartbeat.clone(),
                                                vec!["Sync/E3d".to_string()],
                                            )
                                            .await;
                                        }
                                    }));
                                }
                            }
                        }
                        Incoming(Packet::Publish(p)) => {
                            let sync_e3d = SyncE3dFileMsg::from(p.payload.to_vec());

                            // 详细日志：显示接收到的消息信息
                            println!(
                                "📥 [目标节点: {}] 收到 MQTT 消息 (来源节点: {}, 文件数: {}, 会话范围: {:?}, 文件列表: [{}])",
                                location,
                                sync_e3d.location,
                                sync_e3d.file_names.len(),
                                sync_e3d.session_range,
                                sync_e3d.file_names.join(", ")
                            );

                            if sync_e3d.location != location {
                                println!(
                                    "✅ [目标节点: {}] 消息来源不同，开始处理增量更新 (来源: {})",
                                    location, sync_e3d.location
                                );

                                let _ = SUL_DB
                                    .query(format!(
                                        "INSERT IGNORE INTO e3d_sync {} ",
                                        serde_json::to_string(&sync_e3d).unwrap()
                                    ))
                                    .await;

                                // 记录消息接收到监控系统
                                #[cfg(feature = "web_server")]
                                {
                                    use crate::web_server::mqtt_monitor_handlers;
                                    let message_id =
                                        format!("{:?}_{}", sync_e3d.timestamp, sync_e3d.location);
                                    let message_id_clone = message_id.clone();
                                    mqtt_monitor_handlers::record_message_received(
                                        location.clone(),
                                        message_id,
                                    )
                                    .await;
                                    println!(
                                        "📊 [目标节点: {}] 已记录消息接收状态到监控系统 (消息ID: {})",
                                        location, message_id_clone
                                    );
                                }

                                match Self::exec_delta_clone_remotes(&watcher, sync_e3d).await {
                                    Ok(_) => {
                                        println!("✅ [目标节点: {}] 增量更新处理完成", location);
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "❌ [目标节点: {}] 增量更新处理失败: {:?}",
                                            location, e
                                        );
                                    }
                                }
                            } else {
                                println!(
                                    "⏭️  [目标节点: {}] 消息来源相同，跳过处理 (来源: {})",
                                    location, sync_e3d.location
                                );
                            }
                            // 收到消息，视为连接正常，重置退避
                            backoff = initial_backoff_ms.max(100);
                        }
                        _ => {}
                    },
                    Err(e) => {
                        {
                            let mut mqtt_connect_status = MQTT_CONNECT_STATUS.lock().await;
                            if mqtt_connect_status.is_none() {
                                *mqtt_connect_status = Some(false);
                                error!("Init MQTT Connection error encountered: {}", e);
                            } else if (*mqtt_connect_status).unwrap() {
                                *mqtt_connect_status = Some(false);
                                error!("MQTT Connection error encountered: {}", e);
                            }
                        }
                        // 发生错误，退出内层循环以重建连接
                        break;
                    }
                }
            }
            // 退避等待后重建连接
            tokio::time::sleep(Duration::from_millis(backoff)).await;
            backoff = (backoff.saturating_mul(2)).min(max_backoff);
        }
    }

    ///处理mqtt的消息，带重连退避（单位ms），支持动态MQTT配置（从节点模式）
    pub async fn poll_sync_e3d_mqtt_events_with_backoff_and_config(
        watcher: Arc<PdmsWatcher>,
        initial_backoff_ms: u64,
        max_backoff_ms: u64,
        master_mqtt_host: Option<String>,
        master_mqtt_port: Option<u16>,
    ) {
        let db_option = get_db_option();
        let location = db_option.location.clone();
        let mut backoff = initial_backoff_ms.max(100);
        let max_backoff = max_backoff_ms.max(backoff);

        loop {
            // 构造新的连接实例（使用主节点配置）
            let host = master_mqtt_host.clone().unwrap_or_else(|| db_option.mqtt_host.clone());
            let port = master_mqtt_port.unwrap_or(db_option.mqtt_port);
            
            log::info!(
                "🔄 [从节点: {}] 尝试连接到主节点 MQTT Broker: {}:{}",
                location, host, port
            );
            
            let mut mqtt_inst = new_mqtt_inst_with_config(
                &format!(
                    "{}-{}-sub",
                    db_option.location.as_str(),
                    db_option.project_code
                ),
                master_mqtt_host.clone(),
                master_mqtt_port,
            );
            let _ = mqtt_inst
                .client
                .subscribe("Sync/E3d", QoS::ExactlyOnce)
                .await;

            // 初始标记订阅未连接，等待 ConnAck 后再置为已连接并启动心跳
            #[cfg(feature = "web_server")]
            let mut heartbeat_handle: Option<tokio::task::JoinHandle<()>> = None;
            #[cfg(feature = "web_server")]
            {
                let location_clone = location.clone();
                let node_name = format!("{}-{}", db_option.location, db_option.project_code);
                use crate::web_server::mqtt_monitor_handlers;
                mqtt_monitor_handlers::update_subscription_status(
                    location_clone.clone(),
                    node_name.clone(),
                    false,
                )
                .await;
            }
            // 轮询事件，直到错误发生
            loop {
                let event = mqtt_inst.el.poll().await;
                match &event {
                    Ok(v) => match v {
                        Incoming(Packet::ConnAck(_)) => {
                            #[cfg(feature = "web_server")]
                            {
                                let location_clone = location.clone();
                                let node_name =
                                    format!("{}-{}", db_option.location, db_option.project_code);
                                use crate::web_server::mqtt_monitor_handlers;
                                mqtt_monitor_handlers::update_subscription_status(
                                    location_clone.clone(),
                                    node_name.clone(),
                                    true,
                                )
                                .await;

                                let location_for_heartbeat = location_clone.clone();
                                let node_name_for_heartbeat = node_name.clone();
                                if heartbeat_handle.is_none() {
                                    heartbeat_handle = Some(tokio::spawn(async move {
                                        let mut interval =
                                            tokio::time::interval(Duration::from_secs(10));
                                        loop {
                                            interval.tick().await;
                                            mqtt_monitor_handlers::update_node_heartbeat(
                                                location_for_heartbeat.clone(),
                                                node_name_for_heartbeat.clone(),
                                                vec!["Sync/E3d".to_string()],
                                            )
                                            .await;
                                        }
                                    }));
                                }
                            }
                        }
                        Incoming(Packet::Publish(p)) => {
                            let sync_e3d = SyncE3dFileMsg::from(p.payload.to_vec());

                            // 详细日志：显示接收到的消息信息
                            println!(
                                "📥 [目标节点: {}] 收到 MQTT 消息 (来源节点: {}, 文件数: {}, 会话范围: {:?}, 文件列表: [{}])",
                                location,
                                sync_e3d.location,
                                sync_e3d.file_names.len(),
                                sync_e3d.session_range,
                                sync_e3d.file_names.join(", ")
                            );

                            if sync_e3d.location != location {
                                println!(
                                    "✅ [目标节点: {}] 消息来源不同，开始处理增量更新 (来源: {})",
                                    location, sync_e3d.location
                                );

                                let _ = SUL_DB
                                    .query(format!(
                                        "INSERT IGNORE INTO e3d_sync {} ",
                                        serde_json::to_string(&sync_e3d).unwrap()
                                    ))
                                    .await;

                                // 记录消息接收到监控系统
                                #[cfg(feature = "web_server")]
                                {
                                    use crate::web_server::mqtt_monitor_handlers;
                                    let message_id =
                                        format!("{:?}_{}", sync_e3d.timestamp, sync_e3d.location);
                                    let message_id_clone = message_id.clone();
                                    mqtt_monitor_handlers::record_message_received(
                                        location.clone(),
                                        message_id,
                                    )
                                    .await;
                                    println!(
                                        "📊 [目标节点: {}] 已记录消息接收状态到监控系统 (消息ID: {})",
                                        location, message_id_clone
                                    );
                                }

                                match Self::exec_delta_clone_remotes(&watcher, sync_e3d).await {
                                    Ok(_) => {
                                        println!("✅ [目标节点: {}] 增量更新处理完成", location);
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "❌ [目标节点: {}] 增量更新处理失败: {:?}",
                                            location, e
                                        );
                                    }
                                }
                            } else {
                                println!(
                                    "⏭️  [目标节点: {}] 消息来源相同，跳过处理 (来源: {})",
                                    location, sync_e3d.location
                                );
                            }
                            // 收到消息，视为连接正常，重置退避
                            backoff = initial_backoff_ms.max(100);
                        }
                        _ => {}
                    },
                    Err(e) => {
                        {
                            let mut mqtt_connect_status = MQTT_CONNECT_STATUS.lock().await;
                            if mqtt_connect_status.is_none() {
                                *mqtt_connect_status = Some(false);
                                error!("Init MQTT Connection error encountered: {}", e);
                            } else if (*mqtt_connect_status).unwrap() {
                                *mqtt_connect_status = Some(false);
                                error!("MQTT Connection error encountered: {}", e);
                            }
                        }
                        // 发生错误，退出内层循环以重建连接
                        break;
                    }
                }
            }
            // 退避等待后重建连接
            tokio::time::sleep(Duration::from_millis(backoff)).await;
            backoff = (backoff.saturating_mul(2)).min(max_backoff);
        }
    }

    ///快速获得table名称
    // 已废弃: cache 模块已移除
    pub fn get_table_name(&self, refno: RefU64) -> String {
        "UNSET".to_string()
    }

    ///获得默认的连接字符串
    #[inline]
    pub fn get_default_conn_str(d: &DbOption) -> String {
        let user = d.user.as_str();
        let pwd = urlencoding::encode(d.password.as_str());
        let ip = d.ip.as_str();
        let port = d.port.as_str();
        format!("mysql://{user}:{pwd}@{ip}:{port}")
    }

    #[cfg(feature = "sql")]
    #[inline]
    pub async fn get_global_pool(&self) -> anyhow::Result<Pool<MySql>> {
        let connection_str = self.default_conn_str();
        let url = &format!("{connection_str}/{}", GLOBAL_DATABASE);
        PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(url)
            .await
            .map_err({ |x| anyhow::anyhow!(x.to_string()) })
    }

    ///获得默认的连接字符串
    #[inline]
    pub fn default_conn_str(&self) -> String {
        let d = &self.db_option;
        let user = d.user.as_str();
        let pwd = urlencoding::encode(&d.password);
        let ip = d.ip.as_str();
        let port = d.port.as_str();
        format!("mysql://{user}:{pwd}@{ip}:{port}")
    }
    /// 获得pool
    #[cfg(feature = "sql")]
    #[inline]
    pub async fn get_db_pool(connection_str: &str, project: &str) -> anyhow::Result<Pool<MySql>> {
        let url = &format!("{connection_str}/{}", project);
        PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(url)
            .await
            .map_err({ |x| anyhow::anyhow!(x.to_string()) })
    }

    #[inline]
    pub fn puhua_conn_str(&self) -> String {
        let d = &self.db_option;
        let user = d.puhua_database_user.as_str();
        let pwd = d.puhua_database_password.as_str();
        let ip = d.puhua_database_ip.as_str();
        format!("mysql://{user}:{pwd}@{ip}")
    }

    ///获取普华mysql数据库的连接pool
    #[cfg(feature = "sql")]
    #[inline]
    pub async fn get_puhua_pool(&self) -> anyhow::Result<Pool<MySql>> {
        let conn = self.puhua_conn_str();
        let url = &format!("{conn}/{}", PUHUA_MATERIAL_DATABASE);
        PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(url)
            .await
            .map_err({ |x| anyhow::anyhow!(x.to_string()) })
    }

    ///获取mysql数据库模糊查询的连接pool
    #[cfg(feature = "sql")]
    #[inline]
    pub async fn get_fuzzy_query_pool(&self) -> anyhow::Result<Pool<MySql>> {
        let connection_str = self.default_conn_str();
        let url = &format!("{connection_str}/{}", FUZZY_QUERT);
        PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(url)
            .await
            .map_err({ |x| anyhow::anyhow!(x.to_string()) })
    }

    ///获得默认的pool
    #[cfg(feature = "sql")]
    #[inline]
    pub async fn get_default_pool(conn_str: &str) -> anyhow::Result<Pool<MySql>> {
        MySqlPool::connect(conn_str)
            .await
            .map_err(|x| anyhow::anyhow!(x.to_string()))
    }

    /// 初始化mdb
    pub async fn init_mdb(&mut self, project: &str, mdb: &str, module: &str) -> anyhow::Result<()> {
        Ok(())
    }

    ///初始化db manager
    pub async fn init(db_option: &DbOption) -> anyhow::Result<Self> {
        let dir = db_option.project_path.to_string();
        #[cfg(feature = "sql")]
        let mut project_map = DashMap::new();
        let default_conn = AiosDBManager::get_default_conn_str(&db_option);
        let projects = db_option.get_project_dir_names().clone();

        let mut db_paths =
            collect_db_dirs(&db_option.project_path, projects.iter().map(|x| x.as_ref()))
                .unwrap_or_default();
        // 临时修复：如果 db_paths 为空，直接手动添加 project_path
        if db_paths.is_empty() {
            db_paths.push(db_option.project_path.clone().into());
        }
        dbg!(&db_paths); // 调试输出：看看收集到的目录路径
        let mut watcher = PdmsWatcher::new(db_paths);
        #[cfg(feature = "debug_watch")]
        {
            dbg!(&db_paths);
            dbg!(watcher.headers.len());
            dbg!(watcher.db_path_map.len());
        }
        // 🔧 MQTT 客户端延迟初始化：仅创建客户端，不自动启动事件循环
        // MQTT 连接应通过前端控制（/api/mqtt/subscription/start）手动启动
        let mqtt_inst = new_mqtt_inst(&format!(
            "{}-{}-pub",
            db_option.location.as_str(),
            db_option.project_code
        ));
        let mqtt_client = Arc::new(mqtt_inst.client);
        // 注意：mqtt_inst.el (EventLoop) 在此处被丢弃，连接不会自动建立
        // 如果需要 MQTT 发布功能，需要单独管理 EventLoop 的生命周期
        // 当前设计：MQTT 订阅由 remote_runtime::start_runtime() 或前端 API 控制
        // 初始化失败任务队列
        let failed_queue = FailedTaskQueue::new(PathBuf::from("assets/failed_tasks.json"));

        // 初始化会话号缓存（5秒TTL）
        let sesno_cache = Arc::new(SesnoCache::new(Duration::from_secs(5)));

        // 启动缓存清理worker
        sesno_cache.clone().start_cleanup_worker();

        let mgr = AiosDBManager {
            #[cfg(feature = "sql")]
            project_map,
            projects,
            needed_parse_files: None,
            project_path: dir,
            db_option: db_option.clone(),
            watcher: Arc::new(watcher),
            mqtt_client,
            rtree: None,
            failed_queue,
            sesno_cache,
            #[cfg(any(feature = "mqtt", feature = "web_server"))]
            full_parse_sender: None, // 稍后由 start_full_parse_worker 设置
        };
        // 注意：init_watcher() 不再在这里调用，而是在 web_server 的后台任务中调用
        // 这样可以确保数据库连接完全初始化后再执行，避免 "sending into a closed channel" 错误
        Ok(mgr)
    }

    /// 根据project获取连接池
    #[cfg(feature = "sql")]
    #[inline]
    pub fn get_project_pool(&self, project: &str) -> Option<Pool<MySql>> {
        self.project_map.get(project).map(|x| x.value().clone())
    }

    /// 根据project获取连接池
    #[cfg(feature = "sql")]
    #[inline]
    pub fn get_cur_project_pool(&self) -> Option<Pool<MySql>> {
        self.project_map
            .get(self.get_cur_project())
            .map(|x| x.value().clone())
    }

    ///获得project 的db
    #[cfg(feature = "sql")]
    #[inline]
    pub async fn get_project_pool_by_refno(&self, refno: RefU64) -> Option<(String, Pool<MySql>)> {
        // if let Some(projects) = self.ref0_projects.get(&refno.get_0()) {
        //     ///只有一个的时候
        //     if projects.len() == 1 {
        //         let project = projects.value().iter().next().as_ref().unwrap().clone();
        //         if let Some(project_pool) = self.project_map.get(project) {
        //             return Some((project.clone(), project_pool.value().clone()));
        //         }
        //     } else {
        //         for project in &self.db_option.included_projects {
        //             if let Some(pool) = self.get_project_pool(project) {
        //                 // if check_exist_refno(refno, &pool, &self.mdb_dbnums)
        //                 //     .await
        //                 //     .ok()?
        //                 // {
        //                     return Some((project.clone(), pool.clone()));
        //                 // }
        //             }
        //         }
        //     }
        // }
        None
    }

    fn match_stype(input: i32) -> String {
        match input {
            1 => "DESI".to_string(),
            2 => "CATA".to_string(),
            4 => "PROP".to_string(),
            6 => "ISOD".to_string(),
            7 => "PADD".to_string(),
            8 => "DICT".to_string(),
            9 => "ENGI".to_string(),
            14 => "SCHE".to_string(),
            _ => "".to_string(),
        }
    }

    ///获得当前mdb下的site参考号
    pub async fn get_site_refnos(&self) -> anyhow::Result<Vec<RefU64>> {
        // let world_refno = self.get_desi_world().await?.refno;
        // let r = self
        //     .get_cached_site_nodes(world_refno)
        //     .await?
        //     .unwrap_or_default()
        //     .iter()
        //     .map(|x| x.refno)
        //     .collect();
        Ok(vec![])
    }
}

/// 下载远端 CBA 文件到本地路径
async fn download_cba(client: &Client, url: &str, path: &Path) -> anyhow::Result<()> {
    let resp = client.get(url).send().await?.error_for_status()?;
    let bytes = resp.bytes().await?;

    let mut file = tokio::fs::File::create(path).await?;
    file.write_all(&bytes).await?;
    Ok(())
}

#[tokio::test]
async fn test_get_attr() -> anyhow::Result<()> {
    // let mut mgr = AiosDBManager::init_form_config().await?;
    // let refno: RefU64 = RefI32Tuple((23584, 8)).into();
    // let v = mgr.get_attr(refno).await?;
    // println!("v={:?}", v.to_string_hashmap());

    // mgr.cache_geos_data("Sample", "SAMPLE").await?;

    Ok(())
}

#[test]
fn test_compute_distance() {
    let x = Vec3::new(19373.929, -2923.338, 15286.0);
    let y = Vec3::new(19381.39, -2894.83, 15286.0);
    let arrive = x.distance(y);
    let z = Vec3::new(19381.39, -2865.362, 15286.0);
    let leave = z.distance(y);
    let inst_a = Vec3::new(28.508010864257812, 7.4603271484375, 0.0);
    let inst_b = Vec3::new(0.0, 0.0, 0.0);
    let inst_dis = inst_a.distance(inst_b);
    dbg!(&inst_dis);
    dbg!(&arrive);
    dbg!(&leave);
}
