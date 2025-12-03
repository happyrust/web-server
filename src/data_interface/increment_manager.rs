use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use aios_core::pdms_types::*;
use aios_core::pe::SPdmsElement;
use aios_core::tool::db_tool::db1_dehash;
use aios_core::version::{backup_data, backup_owner_relate};
use aios_core::{RefU64Vec, get_db_option};
use aios_core::{SUL_DB, clear_all_caches};
use anyhow::{Context, anyhow};
use futures::StreamExt;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use notify::{RecursiveMode, Watcher};
use parse_pdms_db::parse::{parse_db_basic_info, parse_file};
use pdms_io::defines::DbPageBasicInfo;
use pdms_io::io::{EleOperationData, EleOperationDetail, PdmsIO};
use pdms_io::sync::compress::{CompressOptions, execute_compress};
// use pdms_io::sync::compress::{execute_compress, CompressOptions};
use pdms_io::watch::PdmsWatcher;
use petgraph::visit::Walker;
use rumqttc::{AsyncClient, QoS};
use serde::{Deserialize, Serialize};
use tokio::fs::create_dir_all;
use toml;
use walkdir::WalkDir;

use crate::data_interface::failed_task_queue::{FailedTask, FailedTaskType};
use crate::data_interface::increment_record::IncrGeoUpdateLog;
use crate::data_interface::interface::PdmsDataInterface;
use crate::data_interface::tidb_manager::AiosDBManager;
use crate::fast_model::*;
use crate::mqtt_service::SyncE3dFileMsg;
#[cfg(feature = "web_server")]
use crate::shared::{ProgressHub, ProgressMessage, TaskStatus};
#[cfg(feature = "web_server")]
use crate::web_server::{
    remote_runtime::REMOTE_RUNTIME,
    remote_sync_handlers,
    sync_control_center::{NewSyncTaskParams, SYNC_CONTROL_CENTER},
};
use parse_pdms_db::parse::DbBasicInfo;

#[cfg(feature = "web_server")]
const PENDING_SYNC_QUEUE_PATH: &str = "assets/pending_sync_queue.json";

/// 增量更新信息结构体
///
/// 用于存储和跟踪数据库中元素的增量变化信息
#[derive(Debug, Default, Clone)]
pub struct IncrementInfo {
    /// 元素的引用编号
    pub refno: RefU64,
    /// 数据库编号
    pub db_no: i32,
    /// 元素的属性映射
    pub attr: NamedAttrMap,
    /// 子元素的引用编号列表
    pub children: RefU64Vec,
    /// 元素的操作类型(增加/修改/删除)
    pub operation: EleOperation,
}

impl IncrementInfo {
    /// 检查元素是否被修改
    ///
    /// # 返回值
    ///
    /// * `bool` - 如果元素被修改返回true，否则返回false
    #[inline]
    pub fn is_modified(&self) -> bool {
        matches!(self.operation, EleOperation::Modified)
    }

    /// 检查元素是否被删除
    ///
    /// # 返回值
    ///
    /// * `bool` - 如果元素被删除返回true，否则返回false
    #[inline]
    pub fn is_deleted(&self) -> bool {
        matches!(self.operation, EleOperation::Deleted)
    }

    /// 检查元素是否为新增
    ///
    /// # 返回值
    ///
    /// * `bool` - 如果元素是新增的返回true，否则返回false
    #[inline]
    pub fn is_added(&self) -> bool {
        matches!(self.operation, EleOperation::Add)
    }
}

/// 增量更新统计信息
#[cfg(feature = "web_server")]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IncrementUpdateStats {
    pub total_added: u32,
    pub total_modified: u32,
    pub total_deleted: u32,
}

#[cfg(feature = "web_server")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeneratedSyncArtifact {
    path: PathBuf,
    file_name: String,
    file_size: u64,
    file_hash: Option<String>,
    record_count: Option<u64>,
    db_num: Option<u32>,
    db_path: Option<String>,
    old_sesno: Option<i32>,
    new_sesno: Option<i32>,
    session_range: Option<String>,
    generated_at: Option<SystemTime>,
    is_full_sync: bool,
    // 新增统计字段
    total_added: Option<u32>,
    total_modified: Option<u32>,
    total_deleted: Option<u32>,
}

/// P1重构: 已存在文件处理结果
struct ExistingFileResult {
    /// 增量参数（path -> (header, sesno_range)）
    pub increment_params: Option<(PathBuf, DbPageBasicInfo, RangeInclusive<i32>)>,
}

/// P1重构: 新文件处理结果
#[cfg(any(feature = "mqtt", feature = "web_server"))]
struct NewFileResult {
    /// 增量参数（用于全量导入）
    pub increment_params: Option<(PathBuf, DbPageBasicInfo, RangeInclusive<i32>)>,
    /// 生成的CBA文件哈希
    pub file_hash: Option<String>,
    /// 文件名
    pub file_name: Option<String>,
    /// 生成的同步产物
    #[cfg(feature = "web_server")]
    pub artifact: Option<GeneratedSyncArtifact>,
}

/// P1重构: 归档生成结果
#[cfg(any(feature = "mqtt", feature = "web_server"))]
struct ArchiveResult {
    /// 文件哈希
    pub file_hash: Option<String>,
    /// 文件名
    pub file_name: String,
    /// 是否应发送通知
    pub should_notify: bool,
    /// 生成的同步产物
    #[cfg(feature = "web_server")]
    pub artifact: Option<GeneratedSyncArtifact>,
}

#[cfg(feature = "web_server")]
fn load_pending_sync_artifacts() -> Vec<GeneratedSyncArtifact> {
    match fs::read_to_string(PENDING_SYNC_QUEUE_PATH) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(err) => {
            if err.kind() != ErrorKind::NotFound {
                eprintln!(
                    "读取待发送的同步任务失败 ({}): {:?}",
                    PENDING_SYNC_QUEUE_PATH, err
                );
            }
            Vec::new()
        }
    }
}

#[cfg(feature = "web_server")]
fn persist_pending_sync_artifacts(artifacts: &[GeneratedSyncArtifact]) {
    if artifacts.is_empty() {
        if let Err(err) = fs::remove_file(PENDING_SYNC_QUEUE_PATH) {
            if err.kind() != ErrorKind::NotFound {
                eprintln!(
                    "清理待发送的同步任务缓存失败 ({}): {:?}",
                    PENDING_SYNC_QUEUE_PATH, err
                );
            }
        }
        return;
    }

    if let Some(parent) = Path::new(PENDING_SYNC_QUEUE_PATH).parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("创建同步任务缓存目录失败 ({}): {:?}", parent.display(), err);
            return;
        }
    }

    match serde_json::to_string_pretty(artifacts) {
        Ok(serialized) => {
            if let Err(err) = fs::write(PENDING_SYNC_QUEUE_PATH, serialized) {
                eprintln!(
                    "写入待发送的同步任务失败 ({}): {:?}",
                    PENDING_SYNC_QUEUE_PATH, err
                );
            }
        }
        Err(err) => {
            eprintln!("序列化同步任务失败: {:?}", err);
        }
    }
}

#[cfg(feature = "web_server")]
async fn try_enqueue_sync_tasks(artifacts: &[GeneratedSyncArtifact]) -> anyhow::Result<()> {
    if artifacts.is_empty() {
        return Ok(());
    }

    let location = get_db_option().location.clone();
    let location_for_query = location.clone();

    let query_result = tokio::task::spawn_blocking(
        move || -> anyhow::Result<Vec<(String, Option<String>)>> {
            let conn = remote_sync_handlers::open_sqlite()
                .map_err(|e| anyhow::anyhow!("Failed to open SQLite: {}", e))?;

            let mut stmt_sites =
                conn.prepare("SELECT id, name FROM remote_sync_sites WHERE location = ?1")?;
            let site_iter = stmt_sites.query_map([location_for_query.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })?;
            let mut sites = Vec::new();
            for item in site_iter {
                sites.push(item?);
            }

            Ok(sites)
        },
    )
    .await?
    .map_err(|err| anyhow::anyhow!("查询远程同步站点失败: {}", err))?;

    let site_entries = query_result;
    let source_env = location.clone();

    // 详细日志：显示查询到的站点信息
    if site_entries.is_empty() {
        println!(
            "⚠️ 未找到目标站点 (location: {})，将创建无目标站点的同步任务",
            location
        );
    } else {
        let site_names: Vec<String> = site_entries
            .iter()
            .map(|(id, name)| {
                if let Some(n) = name {
                    format!("{} ({})", n, id)
                } else {
                    format!("站点 {}", id)
                }
            })
            .collect();
        println!(
            "📋 找到 {} 个目标站点 (location: {}): [{}]",
            site_entries.len(),
            location,
            site_names.join(", ")
        );
    }

    let targets: Vec<(Option<String>, Option<String>)> = if site_entries.is_empty() {
        vec![(None, None)]
    } else {
        site_entries
            .into_iter()
            .map(|(id, name)| (Some(id), name))
            .collect()
    };

    let mut center = SYNC_CONTROL_CENTER.write().await;
    let mut task_count = 0;
    for artifact in artifacts {
        let Some(path_str) = artifact.path.to_str().map(|s| s.to_string()) else {
            continue;
        };
        for (site_id_opt, site_name_opt) in &targets {
            let mut notes_parts = vec![];

            // 标记全量或增量同步类型
            if artifact.is_full_sync {
                notes_parts.push("全量同步".to_string());
            } else {
                notes_parts.push("增量同步".to_string());
            }

            if let Some(db_num) = artifact.db_num {
                notes_parts.push(format!("DB#{}", db_num));
            }
            if let Some(range) = &artifact.session_range {
                notes_parts.push(format!("会话: {}", range));
            } else if let (Some(old_ses), Some(new_ses)) = (artifact.old_sesno, artifact.new_sesno)
            {
                notes_parts.push(format!("会话: {} -> {}", old_ses, new_ses));
            }
            notes_parts.push(format!("文件: {}", artifact.file_name));
            if let Some(site_name) = site_name_opt.clone() {
                notes_parts.push(format!("目标: {}", site_name));
            }

            let notes = Some(notes_parts.join(" | "));

            let target_site_info = if let Some(site_id) = site_id_opt {
                if let Some(site_name) = site_name_opt {
                    format!("{} ({})", site_name, site_id)
                } else {
                    format!("站点 {}", site_id)
                }
            } else {
                "未指定".to_string()
            };

            println!(
                "📝 创建同步任务: 文件={}, 目标站点={}, 类型={}, 会话范围={:?}",
                artifact.file_name,
                target_site_info,
                if artifact.is_full_sync {
                    "全量同步"
                } else {
                    "增量同步"
                },
                artifact.session_range
            );

            center.add_task(NewSyncTaskParams {
                file_path: path_str.clone(),
                file_size: artifact.file_size,
                priority: 5,
                record_count: artifact.record_count,
                file_name: Some(artifact.file_name.clone()),
                file_hash: artifact.file_hash.clone(),
                env_id: None, // 已统一使用 location，不再使用 env_id
                source_env: Some(source_env.clone()),
                target_site: site_id_opt.clone(),
                direction: Some("UPLOAD".to_string()),
                notes,
            });
            task_count += 1;
        }
    }

    println!(
        "✅ 成功创建 {} 个同步任务 (共 {} 个文件, {} 个目标站点)",
        task_count,
        artifacts.len(),
        targets.len()
    );

    Ok(())
}

#[cfg(feature = "web_server")]
async fn enqueue_generated_sync_tasks(artifacts: Vec<GeneratedSyncArtifact>) {
    let mut pending = load_pending_sync_artifacts();
    pending.extend(artifacts);
    if pending.is_empty() {
        return;
    }

    match try_enqueue_sync_tasks(&pending).await {
        Ok(()) => {
            persist_pending_sync_artifacts(&[]);
        }
        Err(err) => {
            eprintln!("同步任务入队失败，已写入本地缓存: {:?}", err);
            persist_pending_sync_artifacts(&pending);
        }
    }
}

#[cfg(feature = "mqtt")]
async fn publish_sync_payload_with_retry(
    client: std::sync::Arc<AsyncClient>,
    payload: SyncE3dFileMsg,
) -> anyhow::Result<()> {
    use aios_core::get_db_option;

    const MAX_RETRIES: usize = 3;
    for attempt in 1..=MAX_RETRIES {
        match client
            .publish("Sync/E3d", QoS::ExactlyOnce, true, payload.clone())
            .await
        {
            Ok(_) => {
                // 记录消息发送（仅在 web_server 特性开启时）
                #[cfg(feature = "web_server")]
                {
                    use crate::web_server::mqtt_monitor_handlers;
                    use chrono::Utc;

                    let db_option = get_db_option();
                    let message_id = format!("{:?}_{}", payload.timestamp, payload.location);

                    // 从远程站点配置获取预期接收者列表
                    let expected_receivers = {
                        #[cfg(feature = "web_server")]
                        {
                            use crate::web_server::remote_sync_handlers;
                            // 获取所有站点的location列表（排除当前站点）
                            match remote_sync_handlers::list_all_sites().await {
                                Ok(sites) => {
                                    let receivers: Vec<String> = sites
                                        .iter()
                                        .filter_map(|site| site.location.clone())
                                        .filter(|loc| loc != &payload.location) // 排除当前站点
                                        .collect();

                                    // 详细日志：显示推送到哪些节点
                                    if receivers.is_empty() {
                                        println!(
                                            "📤 MQTT 推送成功 (主题: Sync/E3d, 文件数: {}, 会话范围: {:?})，但未找到目标节点（当前站点: {}）",
                                            payload.file_names.len(),
                                            payload.session_range,
                                            payload.location
                                        );
                                    } else {
                                        println!(
                                            "📤 MQTT 推送成功 (主题: Sync/E3d, 文件数: {}, 会话范围: {:?})，推送到 {} 个目标节点: [{}] (来源节点: {})",
                                            payload.file_names.len(),
                                            payload.session_range,
                                            receivers.len(),
                                            receivers.join(", "),
                                            payload.location
                                        );
                                    }

                                    receivers
                                }
                                Err(e) => {
                                    eprintln!("⚠️ 获取远程站点列表失败: {:?}", e);
                                    println!(
                                        "📤 MQTT 推送成功 (主题: Sync/E3d, 文件数: {}, 会话范围: {:?})，但无法确定目标节点列表",
                                        payload.file_names.len(),
                                        payload.session_range
                                    );
                                    vec![] // 如果获取失败，返回空列表
                                }
                            }
                        }
                        #[cfg(not(feature = "web_server"))]
                        {
                            vec![] // 非web_server模式下返回空列表
                        }
                    };

                    mqtt_monitor_handlers::record_message_sent(
                        message_id,
                        payload.location.clone(),
                        payload.session_range.clone(),
                        payload.file_names.len(),
                        expected_receivers.clone(),
                    )
                    .await;
                }
                #[cfg(not(feature = "web_server"))]
                {
                    println!(
                        "📤 MQTT 推送成功 (主题: Sync/E3d, 文件数: {}, 会话范围: {:?})",
                        payload.file_names.len(),
                        payload.session_range
                    );
                }

                return Ok(());
            }
            Err(err) => {
                if attempt == MAX_RETRIES {
                    return Err(anyhow::anyhow!(
                        "MQTT 发布失败 (重试 {} 次后仍未成功): {:?}",
                        attempt,
                        err
                    ));
                } else {
                    println!("MQTT 发布失败 (第 {} 次)，准备重试: {:?}", attempt, err);
                    tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                }
            }
        }
    }
    Ok(())
}

const FILE_EVENT_DEBOUNCE_MS: u64 = 500;

const JSON_CHUNK_COUNT: usize = 200;

pub const CHECK_DB_TYPES: [&'static str; 6] = ["CATA", "DESI", "DICT", "SYST", "GLB", "GLOB"];

impl AiosDBManager {
    /// 执行增量更新
    /// 执行增量更新操作
    ///
    /// 该函数处理多个数据库文件的增量更新。
    ///
    /// # 参数
    ///
    /// * `increment_ranges_map` - 包含路径和对应的数据库页面基本信息及会话号范围的映射
    ///   键为数据库文件路径，值为元组，包含数据库页面基本信息和需要更新的会话号范围
    ///
    /// # 返回值
    ///
    /// * `anyhow::Result<bool>` - 成功返回Ok(true)，失败返回错误
    ///
    /// # 错误
    ///
    /// 当数据库操作失败时会返回错误
    /// 查询会话范围的变更统计
    #[cfg(feature = "web_server")]
    async fn query_sesno_range_stats(
        start_sesno: i32,
        end_sesno: i32,
    ) -> anyhow::Result<IncrementUpdateStats> {
        // 查询element_changes表获取增删改统计
        // 使用简单的 COUNT 查询替代复杂的 array::filter_index
        let sql = format!(
            r#"
            SELECT
                (SELECT count() FROM element_changes WHERE sesno >= {} AND sesno <= {} AND operation = 'Add')[0] AS added,
                (SELECT count() FROM element_changes WHERE sesno >= {} AND sesno <= {} AND operation = 'Modify')[0] AS modified,
                (SELECT count() FROM element_changes WHERE sesno >= {} AND sesno <= {} AND operation = 'Delete')[0] AS deleted
            "#,
            start_sesno, end_sesno, start_sesno, end_sesno, start_sesno, end_sesno
        );

        let mut response = SUL_DB.query(&sql).await?;
        let result: Option<serde_json::Value> = response.take(0)?;

        if let Some(stats) = result {
            Ok(IncrementUpdateStats {
                total_added: stats["added"].as_u64().unwrap_or(0) as u32,
                total_modified: stats["modified"].as_u64().unwrap_or(0) as u32,
                total_deleted: stats["deleted"].as_u64().unwrap_or(0) as u32,
            })
        } else {
            Ok(IncrementUpdateStats::default())
        }
    }

    /// P1重构: 处理已存在文件的增量检测
    ///
    /// 检查文件是否有增量更新，如果有则返回增量参数
    ///
    /// # 返回值
    /// - Some((path, header, range)) - 有增量更新，返回参数
    /// - None - 无增量或发生错误（错误已入队）
    async fn process_existing_file(
        &self,
        path: &PathBuf,
        new_header: &DbPageBasicInfo,
        old_sesno: i32,
    ) -> Option<(PathBuf, DbPageBasicInfo, RangeInclusive<i32>)> {
        let new_sesno = new_header.latest_ses_data.sesno;
        let db_num = new_header.pdms_header.db_num;

        println!(
            "处理已存在文件: {:?}, old_sesno={}, new_sesno={}",
            path, old_sesno, new_sesno
        );

        // P0修复: 使用缓存查询（5秒TTL），减少数据库压力
        let db_latest_sesno = match self
            .sesno_cache
            .get_or_query(db_num as u32, |dbnum| async move {
                Self::query_latest_sesno_by_dbnum(dbnum).await
            })
            .await
        {
            Ok(sesno) => sesno,
            Err(e) => {
                // P0修复: 记录失败任务并加入重试队列
                eprintln!(
                    "❌ 查询数据库最新sesno失败: file={:?}, db_num={}, error={:?}",
                    path, db_num, e
                );

                // 创建失败任务并加入队列
                let failed_task = FailedTask::new(
                    FailedTaskType::DatabaseQuery {
                        dbnum: db_num as u32,
                        operation: "query_latest_sesno".to_string(),
                    },
                    format!("数据库查询失败: {:?}", e),
                )
                .with_metadata(serde_json::json!({
                    "file_path": path.to_string_lossy(),
                    "old_sesno": old_sesno,
                    "new_sesno": new_sesno,
                }));

                self.failed_queue.push(failed_task).await;
                return None;
            }
        };

        // 未发生修改，直接跳过
        if db_latest_sesno as i32 == new_sesno {
            println!(
                "文件 {:?} 无增量更新 (db_sesno={}, file_sesno={})",
                path, db_latest_sesno, new_sesno
            );
            return None;
        }

        // 构建增量参数
        let increment_range = (db_latest_sesno as i32 + 1)..=new_sesno;
        println!("检测到增量: {:?}, 范围={:?}", path, increment_range);

        Some((path.clone(), new_header.clone(), increment_range))
    }

    /// P1重构: 处理新文件的增量检测和归档生成
    ///
    /// # 过滤逻辑
    /// 1. 如果 dbno 在 manual_db_nums 中配置且不匹配，则跳过全量解析
    /// 2. 如果 dbno 在 location_dbs 中配置且不匹配，则跳过推送通知
    ///
    /// # 异步处理
    /// 全量解析任务通过 channel 发送到后台 worker 处理，不阻塞主流程
    #[cfg(any(feature = "mqtt", feature = "web_server"))]
    async fn process_new_file(
        &self,
        path: &PathBuf,
        new_header: &DbPageBasicInfo,
    ) -> Option<NewFileResult> {
        use crate::data_interface::full_parse_worker::{FullParseTask, should_filter_db, should_push_notification};

        // println!("watcher.headers: {:?}", self.watcher.headers);
        println!("在 watcher.headers 中找不到路径: {:?}", path);

        // 新增文件的处理逻辑：初始化 headers
        self.watcher
            .headers
            .insert(path.clone(), new_header.clone());

        let file_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(name) => name,
            None => {
                println!("无法从新文件路径中解析文件名: {:?}", path);
                return None;
            }
        };

        let dbno = new_header.pdms_header.db_num as u32;
        let current_sesno = new_header.latest_ses_data.sesno;

        // 检查 manual_db_nums 过滤：如果配置了且 dbno 不在其中，则跳过全量解析
        if should_filter_db(dbno) {
            println!(
                "⏭️ 跳过新文件全量解析: {} (DB#{} 不在 manual_db_nums 列表中)",
                file_name, dbno
            );
            // 仍然更新 headers，但不进行全量解析
            return Some(NewFileResult {
                increment_params: None,
                file_hash: None,
                file_name: None,
                #[cfg(feature = "web_server")]
                artifact: None,
            });
        }

        // 检查 location_dbs 过滤：如果 dbno 在 location_dbs 中，才进行推送通知
        let should_notify = should_push_notification(dbno);
        if !should_notify {
            println!(
                "⏭️ 新文件 {} (DB#{}) 不在 location_dbs 列表中，将进行本地解析但不推送通知",
                file_name, dbno
            );
        }

        // 构建增量参数（用于后续的增量更新）
        let increment_params = if current_sesno > 0 {
            println!(
                "发现新增文件: {}, DB#{}, 准备全量导入 sesno 1-{}",
                file_name, dbno, current_sesno
            );
            Some((path.clone(), new_header.clone(), 1..=current_sesno))
        } else {
            None
        };

        // 如果不需要推送通知，直接返回（不生成 CBA）
        if !should_notify {
            return Some(NewFileResult {
                increment_params,
                file_hash: None,
                file_name: None,
                #[cfg(feature = "web_server")]
                artifact: None,
            });
        }

        // 构建全量解析任务
        let task = FullParseTask {
            path: path.clone(),
            header: new_header.clone(),
            dbno,
            file_name: file_name.to_string(),
            current_sesno,
        };

        // 尝试发送到后台 worker
        if let Some(ref sender) = self.full_parse_sender {
            match sender.send(task) {
                Ok(_) => {
                    println!(
                        "📤 已将全量解析任务发送到后台 Worker: file={}, DB#{}, sesno=1-{}",
                        file_name, dbno, current_sesno
                    );
                    // 返回结果，但不包含 file_hash 和 artifact（它们将由 worker 生成）
                    return Some(NewFileResult {
                        increment_params,
                        file_hash: None, // 由 worker 异步生成
                        file_name: Some(file_name.to_owned()),
                        #[cfg(feature = "web_server")]
                        artifact: None, // 由 worker 异步生成
                    });
                }
                Err(e) => {
                    eprintln!(
                        "⚠️ 发送全量解析任务到 Worker 失败，回退到同步处理: {:?}",
                        e
                    );
                    // 回退到同步处理
                }
            }
        }

        // 回退：同步执行全量解析（当 worker 未启动或发送失败时）
        let output: PathBuf = format!("assets/archives/{}.cba", file_name).into();
        let compress_opt = CompressOptions::new(path.clone(), output.clone(), "assets/temp");

        let file_hash = match execute_compress(compress_opt.clone()).await {
            Ok(h) => h.to_string(),
            Err(e) => {
                // P0修复: 记录失败任务并加入重试队列
                eprintln!(
                    "❌ 新文件压缩生成 CBA 失败: file={}, error={:?}",
                    file_name, e
                );

                let failed_task = FailedTask::new(
                    FailedTaskType::Compression {
                        input_path: path.clone(),
                        output_path: output.clone(),
                        sesno_range: format!("1..={}", current_sesno),
                    },
                    format!("CBA压缩失败: {:?}", e),
                )
                .with_metadata(serde_json::json!({
                    "file_name": file_name,
                    "db_num": dbno,
                    "is_new_file": true,
                    "sesno": current_sesno,
                }));

                self.failed_queue.push(failed_task).await;
                return None;
            }
        };

        #[cfg(feature = "web_server")]
        let artifact = {
            let archive_size = std::fs::metadata(&output)
                .map(|m: std::fs::Metadata| m.len())
                .unwrap_or(0);
            Some(GeneratedSyncArtifact {
                path: output.clone(),
                file_name: format!("{}.cba", file_name),
                file_size: archive_size,
                file_hash: Some(file_hash.clone()),
                record_count: None,
                db_num: Some(dbno),
                db_path: path.to_str().map(|s| s.to_string()),
                old_sesno: Some(0), // 新文件从 0 开始
                new_sesno: Some(current_sesno),
                session_range: Some(format!("1-{}", current_sesno)),
                generated_at: Some(SystemTime::now()),
                is_full_sync: true, // 标记为全量同步
                total_added: None,
                total_modified: None,
                total_deleted: None,
            })
        };

        Some(NewFileResult {
            increment_params,
            file_hash: Some(file_hash),
            file_name: Some(file_name.to_owned()),
            #[cfg(feature = "web_server")]
            artifact,
        })
    }

    pub async fn execute_incr_update(
        &self,
        increment_ranges_map: IndexMap<PathBuf, (DbPageBasicInfo, RangeInclusive<i32>)>,
    ) -> anyhow::Result<bool> {
        for (path, (basic_info, sesno_range)) in increment_ranges_map {
            let start_sesno = *sesno_range.start();
            let end_sesno = *sesno_range.end();
            
            println!("Path: {:?}, Sesno Range: {:?}", path, &sesno_range);
            
            // 判断是全量解析还是增量解析
            // 如果 sesno 从 1 开始，说明是新文件，使用全量解析
            if start_sesno == 1 {
                println!("📦 检测到新文件，使用全量解析方式");
                self.execute_full_parse(path.clone(), &basic_info, end_sesno).await?;
                continue;
            }
            
            // 增量解析：只解析变化的 sesno 范围
            println!("📝 使用增量解析方式 (sesno: {}..={})", start_sesno, end_sesno);
            let mut io = PdmsIO::new("", path.clone(), true);
            io.open()
                .map_err(|e| anyhow::anyhow!("Failed to open PdmsIO: {}", e))?;
            let range_update_eles = io.collect_increment_eles(Some(sesno_range))?;
            io.update_elements_to_database(&range_update_eles, true)
                .await?;

            //执行逻辑

            //更新 sesno 到 db_file_info 中
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            // dbg!(&file_name);

            // P0修复: 使用 UPSERT 确保 SESNO 正确更新
            // SurrealDB 的 UPSERT 自动处理记录存在/不存在的情况
            // 同时更新 dbnum、db_type、file_name 字段，以便后续查询和管理
            let dbno = basic_info.pdms_header.db_num as u32;
            let db_basic_info = parse_db_basic_info(path.clone())
                .with_context(|| format!("解析数据库基本信息失败: {:?}", path))?;
            let db_type = db_basic_info.db_type;

            // 使用 record_ses_info 函数统一更新逻辑
            Self::record_ses_info(file_name, dbno, &db_type, end_sesno as u32)
                .await
                .with_context(|| format!("更新 db_file_info:{} SESNO 失败", file_name))?;

            eprintln!(
                "✅ UPSERT db_file_info:{} SESNO={}, dbnum={}, db_type={}",
                file_name, end_sesno, dbno, db_type
            );

            // 验证更新是否成功
            let verify_sesno = Self::query_latest_sesno_by_file_name(file_name)
                .await
                .with_context(|| "验证 SESNO 更新失败")?;
            if verify_sesno != end_sesno as u32 {
                return Err(anyhow!(
                    "SESNO 更新验证失败: db_file_info:{} 期望={}, 实际={}",
                    file_name,
                    end_sesno,
                    verify_sesno
                ));
            }

            // P0修复: 数据库更新成功后，使sesno缓存失效
            self.sesno_cache.invalidate(dbno);

            // 简化逻辑：只使用 db_file_info 表，不再更新 dbnum_info_table
            // db_file_info 表已经在上面更新了，这里只需要使缓存失效即可
        }

        Ok(true)
    }

    /// 执行全量解析（用于新文件）
    /// 
    /// 使用 parse_pdms_db 一次性解析整个文件，比逐 sesno 遍历更高效
    /// 
    /// # 参数
    /// * `path` - 数据库文件路径
    /// * `basic_info` - 数据库基本信息
    /// * `end_sesno` - 当前会话号
    pub async fn execute_full_parse(
        &self,
        path: PathBuf,
        basic_info: &DbPageBasicInfo,
        end_sesno: i32,
    ) -> anyhow::Result<bool> {
        let file_name = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("无法获取文件名: {:?}", path))?;
        let project = get_db_option().project_code.clone();
        
        println!("🚀 开始全量解析: {} (sesno: 1-{})", file_name, end_sesno);
        let start_time = Instant::now();
        
        // 使用 parse_pdms_db 全量解析文件
        let db_data = parse_file(&path, &None, file_name, &project)
            .await
            .with_context(|| format!("全量解析文件失败: {:?}", path))?;
        
        let parse_elapsed = start_time.elapsed();
        println!("✅ 文件解析完成: {} 个元素, 耗时: {:.2}s", 
            db_data.total_attr_map.len(), parse_elapsed.as_secs_f64());
        
        // 将解析结果保存到 SurrealDB
        let save_start = Instant::now();
        let dbno = basic_info.pdms_header.db_num as u32;
        let db_type = &db_data.db_type;
        
        // 批量插入元素到 pe 表
        let mut insert_count = 0;
        let batch_size = 500;
        let mut batch = Vec::with_capacity(batch_size);
        
        for entry in db_data.total_attr_map.iter() {
            let att = entry.value();
            if let Some(json) = att.gen_sur_json() {
                batch.push(json);
                if batch.len() >= batch_size {
                    // 执行批量插入
                    let sql = format!(
                        "INSERT INTO pe [{}] ON DUPLICATE KEY UPDATE sesno = $input.sesno, deleted = $input.deleted;",
                        batch.join(",")
                    );
                    SUL_DB.query(&sql).await
                        .with_context(|| format!("批量插入 pe 失败: batch_size={}", batch.len()))?;
                    insert_count += batch.len();
                    batch.clear();
                }
            }
        }
        
        // 处理剩余的元素
        if !batch.is_empty() {
            let sql = format!(
                "INSERT INTO pe [{}] ON DUPLICATE KEY UPDATE sesno = $input.sesno, deleted = $input.deleted;",
                batch.join(",")
            );
            SUL_DB.query(&sql).await
                .with_context(|| "批量插入剩余 pe 失败")?;
            insert_count += batch.len();
        }
        
        let save_elapsed = save_start.elapsed();
        println!("✅ 数据保存完成: {} 个元素, 耗时: {:.2}s", insert_count, save_elapsed.as_secs_f64());
        
        // 更新 db_file_info 记录
        Self::record_ses_info(file_name, dbno, db_type, end_sesno as u32)
            .await
            .with_context(|| format!("更新 db_file_info:{} 失败", file_name))?;
        
        // 使缓存失效
        self.sesno_cache.invalidate(dbno);
        
        let total_elapsed = start_time.elapsed();
        println!("🎉 全量解析完成: file={}, elements={}, total_time={:.2}s", 
            file_name, insert_count, total_elapsed.as_secs_f64());
        
        Ok(true)
    }

    /// 通过文件名查询数据库中最新的会话号
    ///
    /// # 参数
    ///
    /// * `file_name` - 要查询的数据库文件名
    ///
    /// # 返回值
    ///
    /// * `anyhow::Result<u32>` - 成功则返回最新会话号,失败返回错误
    ///
    /// # 错误
    ///
    /// 当数据库查询失败时会返回错误
    async fn query_latest_sesno_by_file_name(file_name: &str) -> anyhow::Result<u32> {
        let mut response = SUL_DB
            .query(format!(
                r#"
                select value sesno from only db_file_info:{} limit 1;
                "#,
                file_name
            ))
            .await?;
        let sesno: Option<u32> = response.take(0)?;
        Ok(sesno.unwrap_or_default())
    }

    /// 获取允许同步推送的数据库类型列表
    ///
    /// # 返回值
    ///
    /// * `Vec<String>` - 允许推送的数据库类型列表，空列表表示允许所有类型
    fn get_sync_push_db_types() -> Vec<String> {
        // 从配置文件读取 sync_push_db_types 配置项
        let config_name =
            std::env::var("DB_OPTION_FILE").unwrap_or_else(|_| "DbOption".to_string());
        let config_file = format!("{}.toml", config_name);

        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(toml_value) = toml::from_str::<toml::Value>(&content) {
                if let Some(array) = toml_value
                    .get("sync_push_db_types")
                    .and_then(|v| v.as_array())
                {
                    return array
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
        }

        // 默认值：只允许 DESI 类型
        vec!["DESI".to_string()]
    }

    /// 通过数据库编号查询数据库中最新的会话号
    ///
    /// # 参数
    ///
    /// * `dbnum` - 要查询的数据库编号
    ///
    /// # 返回值
    ///
    /// * `anyhow::Result<u32>` - 成功则返回最新会话号,失败返回错误
    ///
    /// # 错误
    ///
    /// 当数据库查询失败时会返回错误
    async fn query_latest_sesno_by_dbnum(dbnum: u32) -> anyhow::Result<u32> {
        // 从 db_file_info 表中查询对应 dbnum 的最大 sesno
        // 简化逻辑：只使用 db_file_info 表，不再依赖 dbnum_info_table
        let mut response = SUL_DB
            .query(format!(
                r#"
                math::max(array::flatten([
                    SELECT VALUE sesno FROM db_file_info WHERE dbnum = {}
                ]));
                "#,
                dbnum
            ))
            .await?;
        let sesno: Option<u32> = response.take(0)?;
        Ok(sesno.unwrap_or_default())
    }

    /// 检查是否需要生成或更新CBA文件
    ///
    /// # 参数
    /// * `cba_path` - CBA文件路径
    /// * `file_sesno` - 源文件的最新sesno
    /// * `db_sesno` - 数据库中记录的最新sesno
    ///
    /// # 返回值
    /// * `true` - 需要生成或更新CBA文件
    /// * `false` - CBA文件已存在且无需更新
    fn should_generate_cba(cba_path: &Path, file_sesno: i32, db_sesno: i32) -> bool {
        // 如果CBA文件不存在，需要生成
        if !cba_path.exists() {
            return true;
        }

        // 如果文件存在，比较sesno判断是否需要更新
        // 如果文件的sesno大于数据库中的sesno，说明文件有更新，需要重新生成CBA
        file_sesno > db_sesno
    }

    /// 记录数据库文件的 ses info 到表中（不需要解析整个 DB）
    ///
    /// # 参数
    /// * `file_name` - 数据库文件名
    /// * `db_no` - 数据库编号
    /// * `db_type` - 数据库类型（如 DESI, CATA, DICT 等）
    /// * `file_sesno` - 文件的最新 sesno
    ///
    /// # 返回值
    /// * `anyhow::Result<()>` - 成功返回 Ok(())，失败返回错误
    async fn record_ses_info(
        file_name: &str,
        db_no: u32,
        db_type: &str,
        file_sesno: u32,
    ) -> anyhow::Result<()> {
        // 更新 db_file_info 表，存储完整的文件信息
        // 字段包括：file_name, dbnum, db_type, sesno, updated_at
        let upsert_file_sql = format!(
            r#"
            UPSERT db_file_info:{} SET 
                file_name = {},
                dbnum = {},
                db_type = {},
                sesno = {},
                updated_at = time::now();
            "#,
            file_name,
            format!("'{}'", file_name), // file_name 字段（字符串）
            db_no,                      // dbnum 字段（数字）
            format!("'{}'", db_type),   // db_type 字段（字符串）
            file_sesno                  // sesno 字段（数字）
        );
        
        // 添加重试机制，防止连接在操作过程中被关闭
        let mut retry_count = 0;
        let max_retries = 3;
        loop {
            match SUL_DB.query(&upsert_file_sql).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    // 如果是通道关闭错误，尝试重试
                    if error_msg.contains("closed channel") || error_msg.contains("Failed to send command") {
                        retry_count += 1;
                        if retry_count > max_retries {
                            return Err(anyhow::anyhow!("记录 db_file_info:{} SESNO 失败（已重试 {} 次）: {}", file_name, max_retries, error_msg))
                                .with_context(|| format!("记录 db_file_info:{} SESNO 失败", file_name));
                        }
                        // 等待后重试（指数退避）
                        tokio::time::sleep(tokio::time::Duration::from_millis(200 * retry_count as u64)).await;
                        continue;
                    }
                    // 其他错误直接返回
                    return Err(e).with_context(|| format!("记录 db_file_info:{} SESNO 失败", file_name));
                }
            }
        }
    }

    /// 读取 rescan_all_dbs_on_startup 配置项
    ///
    /// # 返回值
    /// * `bool` - 如果配置为 true，返回 true；否则返回 false（默认值）
    fn read_rescan_all_dbs_config() -> bool {
        let config_name =
            std::env::var("DB_OPTION_FILE").unwrap_or_else(|_| "DbOption".to_string());
        let config_file = format!("{}.toml", config_name);

        if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(toml_value) = toml::from_str::<toml::Value>(&content) {
                if let Some(rescan) = toml_value.get("rescan_all_dbs_on_startup") {
                    if let Some(b) = rescan.as_bool() {
                        return b;
                    }
                }
            }
        }

        false
    }

    ///初始化监测
    /// 启动时监测数据文件夹里的文件变化
    pub async fn init_watcher(&self) -> anyhow::Result<()> {
        let mut params = IndexMap::new();
        fs::create_dir_all("assets/archives")?;
        let mut time = Instant::now();
        dbg!(&self.watcher.watch_dirs);
        let db_option = get_db_option();
        let manual_dbnums = db_option.manual_db_nums.clone().unwrap_or_default();
        let exclude_dbnums = db_option.exclude_db_nums.clone().unwrap_or_default();

        // 读取 rescan_all_dbs_on_startup 配置项
        // 如果为 true，则扫描全部数据库（忽略 manual_db_nums 限制）
        let rescan_all_dbs = Self::read_rescan_all_dbs_config();

        for watch_dir in &self.watcher.watch_dirs {
            for entry in WalkDir::new(watch_dir).sort_by(|a, b| {
                let a_len = a.path().metadata().map(|m| m.len()).unwrap_or_default();
                let b_len = b.path().metadata().map(|m| m.len()).unwrap_or_default();
                b_len.cmp(&a_len)
            }) {
                let dir_entry =
                    entry.map_err(|e| anyhow::anyhow!("Failed to get directory entry: {}", e))?;
                let path = dir_entry.path();
                let file_name = path
                    .file_stem()
                    .ok_or_else(|| {
                        anyhow::anyhow!("Failed to get file stem from path: {}", path.display())
                    })?
                    .to_str()
                    .ok_or_else(|| {
                        anyhow::anyhow!("Failed to convert file stem to string: {}", path.display())
                    })?;
                if path.is_dir() {
                    continue;
                }

                // 安全解析数据库基本信息，跳过无法解析的文件
                let DbBasicInfo {
                    db_type,
                    ses_pgno,
                    db_no,
                } = match parse_db_basic_info(path.to_path_buf()) {
                    Ok(info) => info,
                    Err(e) => {
                        eprintln!(
                            "⚠️  跳过文件 {}: 无法解析数据库基本信息: {}",
                            path.display(),
                            e
                        );
                        continue;
                    }
                };

                // 如果启用了全量扫描，忽略 manual_db_nums 限制
                // 否则按原有逻辑过滤
                if !rescan_all_dbs {
                    //是否调试里有筛选
                    if !manual_dbnums.is_empty() && !manual_dbnums.contains(&db_no) {
                        continue;
                    }
                }
                //过滤掉排除的数据库编号
                if !exclude_dbnums.is_empty() && exclude_dbnums.contains(&db_no) {
                    continue;
                }
                let project = get_db_option().project_name.clone();

                // 检查文件是否存在且可读，避免读取损坏或不完整的文件导致 panic
                let file_latest_sesno = if path.exists() {
                    // 检查文件大小，空文件或过小的文件可能不完整
                    if let Ok(metadata) = std::fs::metadata(path) {
                        if metadata.len() < 1024 {
                            eprintln!(
                                "⚠️  跳过文件 {} (db_no={}): 文件大小过小 ({} bytes)，可能不完整",
                                file_name,
                                db_no,
                                metadata.len()
                            );
                            0
                        } else {
                            // 尝试读取 sesno，捕获所有可能的错误（包括内部 panic 的源头）
                            match PdmsIO::new(&project, path.to_path_buf(), true).get_latest_sesno()
                            {
                                Ok(sesno) => sesno,
                                Err(e) => {
                                    eprintln!(
                                        "⚠️  读取文件 {} (db_no={}) 的 sesno 失败: {}，跳过该文件",
                                        file_name, db_no, e
                                    );
                                    eprintln!(
                                        "   文件路径: {:?}，文件大小: {} bytes",
                                        path,
                                        metadata.len()
                                    );
                                    // 返回 0，跳过该文件，避免程序崩溃
                                    0
                                }
                            }
                        }
                    } else {
                        eprintln!(
                            "⚠️  无法获取文件 {} (db_no={}) 的元数据，跳过",
                            file_name, db_no
                        );
                        0
                    }
                } else {
                    eprintln!("⚠️  文件 {} (db_no={}) 不存在，跳过", file_name, db_no);
                    0
                };

                // dbg!((db_no, file_latest_sesno));

                if !CHECK_DB_TYPES.contains(&db_type.as_str()) {
                    continue;
                }

                // 无论 CBA 文件是否存在，都记录 ses info 到表中
                // 这样即使 CBA 文件已存在，也能保持数据库中的 ses info 是最新的
                if let Err(e) =
                    Self::record_ses_info(file_name, db_no, &db_type, file_latest_sesno).await
                {
                    eprintln!(
                        "⚠️  记录 ses info 失败: file={}, db_no={}, db_type={}, sesno={}, error={:?}",
                        file_name, db_no, db_type, file_latest_sesno, e
                    );
                } else {
                    // println!(
                    //     "📝 已记录 ses info: file={}, db_no={}, db_type={}, sesno={}",
                    //     file_name, db_no, db_type, file_latest_sesno
                    // );
                }

                // 查询数据库中的sesno（如果查询失败或为0，仍然可以生成初始CBA文件）
                let db_latest_sesno = Self::query_latest_sesno_by_dbnum(db_no).await.unwrap_or(0);
                // dbg!((db_no, file_latest_sesno, db_latest_sesno));

                // 将文件添加到映射中（无论数据库是否有记录，使用 UPPERCASE key）
                self.watcher.insert_db_path(file_name, path.to_path_buf());

                // 检查是否需要生成CBA文件（仅在sync_live启用时）
                // 启动时只生成缺失的CBA文件，不基于增量检测生成
                // 有增量更新时会在增量更新完成后自动更新CBA文件
                if db_option.sync_live.unwrap_or(false) {
                    let cba_path: PathBuf = format!("assets/archives/{}.cba", file_name).into();
                    let file_sesno = file_latest_sesno as i32;
                    let db_sesno = db_latest_sesno as i32;

                    // 启动时只在CBA文件不存在时生成（首次初始化）
                    // 如果文件已存在，即使有增量更新，也不在启动时生成，而是等到增量更新完成后再更新
                    let should_generate = !cba_path.exists();

                    if should_generate {
                        let input = path.to_path_buf();
                        let output = cba_path.clone();

                        // 判断是生成新文件还是更新已有文件
                        let is_update = output.exists();
                        let action = if is_update { "更新" } else { "生成" };

                        println!("🔄 开始{}CBA文件: {}", action, file_name);

                        let compress_opt =
                            CompressOptions::new(input, output.clone(), "assets/temp");
                        match execute_compress(compress_opt).await {
                            Ok(hash) => {
                                let file_size =
                                    std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
                                println!(
                                    "✅ CBA文件{}成功: {} (大小: {} bytes, 哈希: {})",
                                    action, file_name, file_size, hash
                                );
                            }
                            Err(e) => {
                                eprintln!("❌ CBA文件{}失败: {}, 错误: {:?}", action, file_name, e);

                                // 记录失败任务到重试队列
                                let failed_task = FailedTask::new(
                                    FailedTaskType::Compression {
                                        input_path: path.to_path_buf(),
                                        output_path: output,
                                        sesno_range: format!("1..={}", file_sesno),
                                    },
                                    format!("CBA{}失败: {:?}", action, e),
                                )
                                .with_metadata(serde_json::json!({
                                    "file_name": file_name,
                                    "db_num": db_no,
                                    "is_new_file": !is_update,
                                    "file_sesno": file_sesno,
                                    "db_sesno": db_sesno,
                                }));

                                self.failed_queue.push(failed_task).await;
                            }
                        }
                    } else {
                        // CBA文件已存在，启动时不生成
                        // 如果有增量更新（file_sesno > db_sesno），会在增量更新完成后自动更新CBA
                        if file_sesno > db_sesno {
                            // println!(
                            //     "⏭️  跳过CBA文件生成: {} (文件已存在, 将在增量更新后自动更新, file_sesno={}, db_sesno={})",
                            //     file_name, file_sesno, db_sesno
                            // );
                        } else {
                            // println!(
                            //     "⏭️  跳过CBA文件生成: {} (文件已存在且无增量更新, file_sesno={}, db_sesno={})",
                            //     file_name, file_sesno, db_sesno
                            // );
                        }
                    }
                } else {
                    // println!("⏭️  跳过CBA文件生成: {} (sync_live未启用)", file_name);
                }

                //每个path 都要检查一遍
                // if db_latest_sesno != 0
                {
                    // #[cfg(feature = "debug_parse")]
                    // dbg!((db_no, db_latest_sesno));
                    //暂时先跳过更新比较大的
                    //
                    {
                        let mut io = PdmsIO::new(&project, path, true);
                        io.open()?;
                        if let Ok(basic_info) = io.get_page_basic_info() {
                            if file_latest_sesno > db_latest_sesno {
                                // 即使 rescan_all_dbs 为 true（允许扫描所有文件记录 ses info），
                                // 在执行增量解析时仍然应该尊重 manual_dbnums 的限制
                                // rescan_all_dbs 只影响扫描和记录 ses info，不影响增量解析的执行
                                let should_process_increment = manual_dbnums.is_empty() || manual_dbnums.contains(&db_no);

                                if should_process_increment {
                                    println!(
                                        "发现需要增量更新的文件: {:?}, 当前数据库属性最大sesno: {db_latest_sesno},\
                                            文件属性对应sesno: {file_latest_sesno}",
                                        &file_name
                                    );
                                    let nearest_sesno = io
                                        .get_nearest_large_sesno(db_latest_sesno as i32 + 1)
                                        .unwrap_or_default();
                                    params.insert(
                                        path.to_path_buf(),
                                        (
                                            basic_info.clone(),
                                            //warning : db_latest_sesno as i32 + 1 不一定存在，需要找离他最近的sesno
                                            nearest_sesno..=file_latest_sesno as i32,
                                        ),
                                    );
                                } else {
                                    // 跳过增量解析，但已记录 ses info
                                    // println!(
                                    //     "⏭️  跳过增量解析: {} (不在 manual_db_nums 列表中, db_no={})",
                                    //     file_name, db_no
                                    // );
                                }
                            }
                            // 初始化监听的headers
                            self.watcher.headers.insert(path.to_path_buf(), basic_info);
                        }
                    }
                }
            }
        }

        //等所有的文件都检查同步完毕，才执行更新
        //按每个单独的 sesno
        if !params.is_empty() {
            dbg!(params.len());
        }

        // 记录需要更新CBA的文件列表
        let mut files_to_update_cba = Vec::new();
        if !params.is_empty() {
            for (path, (_basic_info, sesno_range)) in &params {
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                files_to_update_cba.push((path.clone(), file_name.to_string(), *sesno_range.end()));
            }
        }

        match self.execute_incr_update(params).await {
            Ok(true) => {
                println!("执行启动后的自动增量完成。");

                // 增量更新成功后，更新对应的CBA文件
                if db_option.sync_live.unwrap_or(false) && !files_to_update_cba.is_empty() {
                    println!("🔄 开始更新增量更新后的CBA文件...");
                    for (path, file_name, end_sesno) in files_to_update_cba {
                        let output: PathBuf = format!("assets/archives/{}.cba", file_name).into();

                        println!("🔄 更新CBA文件: {}", file_name);
                        let compress_opt =
                            CompressOptions::new(path.clone(), output.clone(), "assets/temp");

                        match execute_compress(compress_opt).await {
                            Ok(hash) => {
                                let file_size =
                                    std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
                                println!(
                                    "✅ CBA文件更新成功: {} (大小: {} bytes, 哈希: {}, sesno: {})",
                                    file_name, file_size, hash, end_sesno
                                );
                            }
                            Err(e) => {
                                eprintln!("❌ CBA文件更新失败: {}, 错误: {:?}", file_name, e);

                                // 记录失败任务到重试队列
                                let failed_task = FailedTask::new(
                                    FailedTaskType::Compression {
                                        input_path: path.clone(),
                                        output_path: output,
                                        sesno_range: format!("1..={}", end_sesno),
                                    },
                                    format!("CBA更新失败: {:?}", e),
                                )
                                .with_metadata(serde_json::json!({
                                    "file_name": file_name,
                                    "end_sesno": end_sesno,
                                    "is_incremental_update": true,
                                }));

                                self.failed_queue.push(failed_task).await;
                            }
                        }
                    }
                    println!("✅ CBA文件更新完成");
                }
            }
            Ok(false) => {
                println!("没有发生增量更新。")
            }
            Err(e) => {
                println!("Execute increment update error: {:?}", e);
            }
        }

        println!("初始化增量更新耗时: {} s", time.elapsed().as_secs_f32());

        anyhow::Ok(())
    }

    //开始监测数据文件夹
    pub async fn async_watch(&self) -> notify::Result<()> {
        let (mut watcher, mut rx) = PdmsWatcher::async_watcher()?;
        dbg!(&self.watcher.watch_dirs);
        self.watcher.watch_dirs.iter().for_each(|x| {
            watcher
                .watch(x.as_path(), RecursiveMode::NonRecursive)
                .expect("watch files failed");
        });

        create_dir_all("assets/archives")
            .await
            .map_err(|e| notify::Error::io(e))?;
        create_dir_all("assets/temp")
            .await
            .map_err(|e| notify::Error::io(e))?;
        while let Some(res) = rx.next().await {
            match res {
                Ok(event) => {
                    // dbg!(&event);
                    //跳过只是meta data变动的情况
                    let data_changed = matches!(
                        event.kind,
                        notify::EventKind::Modify(notify::event::ModifyKind::Data(_))
                            | notify::EventKind::Modify(notify::event::ModifyKind::Any)
                            | notify::EventKind::Create(notify::event::CreateKind::File)
                            | notify::EventKind::Remove(notify::event::RemoveKind::File)
                    );
                    if !data_changed {
                        continue;
                    }
                    //后面用派发任务的方式,不要放在这里阻塞
                    println!("changed: {:?}", &event);
                    // 添加调试信息
                    println!("开始扫描数据库头部信息，路径: {:?}", &event.paths);
                    // dbg!(&self.watcher.headers);
                    if let Ok(new_headers) = PdmsWatcher::scan_db_headers(&event.paths) {
                        println!("成功扫描到 {} 个数据库头部", new_headers.len());
                        #[cfg(feature = "web_server")]
                        let mut generated_artifacts: Vec<
                            GeneratedSyncArtifact,
                        > = Vec::new();
                        // 收集本次事件中需要通过 MQTT 推送的文件名和哈希
                        let mut notify_file_names = vec![];
                        let mut notify_file_hashes = vec![];
                        let mut params = IndexMap::new();
                        for (path, new_header) in &new_headers {
                            println!("正在处理路径: {:?}", path);

                            // ===== P0修复: 并发安全优化 =====
                            // 先快速读取旧值并立即释放锁，避免长时间持有锁
                            let old_sesno_opt = self
                                .watcher
                                .headers
                                .get(path)
                                .map(|entry| entry.latest_ses_data.sesno);

                            if let Some(old_sesno) = old_sesno_opt {
                                // P1重构: 使用抽取的函数处理已存在文件
                                if let Some((p, h, r)) = self
                                    .process_existing_file(path, new_header, old_sesno)
                                    .await
                                {
                                    params.insert(p, (h, r));
                                }
                                // 已存在文件处理完毕，跳过新文件逻辑
                                continue;
                            }

                            // P1重构: 使用抽取的函数处理新文件
                            #[cfg(any(feature = "mqtt", feature = "web_server"))]
                            {
                                if let Some(result) = self.process_new_file(path, new_header).await
                                {
                                    // 将增量参数加入 params
                                    if let Some((p, h, r)) = result.increment_params {
                                        params.insert(p, (h, r));
                                    }

                                    // 将 artifact 加入 generated_artifacts
                                    #[cfg(feature = "web_server")]
                                    if let Some(artifact) = result.artifact {
                                        generated_artifacts.push(artifact);
                                    }

                                    // 处理 MQTT 通知
                                    #[cfg(feature = "mqtt")]
                                    if let (Some(file_hash), Some(file_name)) =
                                        (result.file_hash, result.file_name)
                                    {
                                        // 避免对已经同步过相同文件 hash 的记录重复发送
                                        // 修复：检查是否有其他节点已经收到过这个消息，而不是仅检查当前节点是否已发送
                                        // 这样可以确保即使当前节点已经发送过，如果其他节点还没有收到，也会继续推送
                                        let current_location = get_db_option().location.as_str();
                                        
                                        // 获取所有目标节点列表（排除当前节点）
                                        #[cfg(feature = "web_server")]
                                        let target_locations = {
                                            use crate::web_server::remote_sync_handlers;
                                            match remote_sync_handlers::list_all_sites().await {
                                                Ok(sites) => {
                                                    sites.iter()
                                                        .filter_map(|site| site.location.clone())
                                                        .filter(|loc| loc != current_location)
                                                        .collect::<Vec<String>>()
                                                }
                                                Err(e) => {
                                                    eprintln!("⚠️ 获取目标节点列表失败: {:?}，将跳过去重检查", e);
                                                    Vec::new()
                                                }
                                            }
                                        };
                                        #[cfg(not(feature = "web_server"))]
                                        let target_locations: Vec<String> = Vec::new();

                                        // 检查是否有其他节点已经收到过这个文件
                                        let should_skip = if target_locations.is_empty() {
                                            // 如果没有配置目标节点，检查是否有任何其他节点（除了当前节点）已经收到过这个消息
                                            // 如果没有其他节点收到过，就应该发送 MQTT 消息（因为 MQTT 是广播的）
                                            let sql = format!(
                                                "select value <string> id from (select * from e3d_sync where location != '{}' and '{}' in file_names and '{}' in file_hashes order by timestamp desc LIMIT 1) ",
                                                current_location,
                                                file_name,
                                                &file_hash
                                            );
                                            let id = match SUL_DB.query(&sql).await {
                                                Ok(mut resp) => {
                                                    resp.take::<Vec<String>>(0).unwrap_or_default()
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "❌ 新文件去重查询失败: file={}, error={:?}",
                                                        file_name, e
                                                    );
                                                    Vec::new() // 查询失败时，不跳过推送
                                                }
                                            };
                                            // 如果有其他节点已经收到过，才跳过推送
                                            !id.is_empty()
                                        } else {
                                            // 检查所有目标节点是否都已经收到过这个文件
                                            let mut all_received = true;
                                            for target_loc in &target_locations {
                                                let sql = format!(
                                                    "select value <string> id from (select * from e3d_sync where location = '{}' and '{}' in file_names and '{}' in file_hashes order by timestamp desc LIMIT 1) ",
                                                    target_loc,
                                                    file_name,
                                                    &file_hash
                                                );
                                                let mut response = match SUL_DB.query(&sql).await {
                                                    Ok(resp) => resp,
                                                    Err(e) => {
                                                        eprintln!(
                                                            "❌ 新文件去重查询失败: file={}, target={}, error={:?}",
                                                            file_name, target_loc, e
                                                        );
                                                        all_received = false; // 查询失败时，认为未收到
                                                        break;
                                                    }
                                                };
                                                let id = response.take::<Vec<String>>(0).unwrap_or_default();
                                                if id.is_empty() {
                                                    all_received = false;
                                                    break;
                                                }
                                            }
                                            all_received
                                        };

                                        if !should_skip {
                                            println!("发现新增 db 文件，推送：{} (文件hash: {}, 当前节点: {})", &file_name, &file_hash, current_location);
                                            notify_file_hashes.push(file_hash);
                                            notify_file_names.push(file_name);
                                        } else {
                                            println!("⏭️  跳过推送新文件: {} (所有目标节点已收到此文件，文件hash: {}, 当前节点: {})", &file_name, &file_hash, current_location);
                                        }
                                    }
                                }
                            }

                            // 非 mqtt/web_server 特性时的简化处理
                            #[cfg(not(any(feature = "mqtt", feature = "web_server")))]
                            {
                                self.watcher
                                    .headers
                                    .insert(path.clone(), new_header.clone());
                                let current_sesno = new_header.latest_ses_data.sesno;
                                if current_sesno > 0 {
                                    params.insert(
                                        path.clone(),
                                        (new_header.clone(), 1..=current_sesno),
                                    );
                                }
                            }
                        }
                        // dbg!(&params);
                        if params.is_empty() {
                            continue;
                        }

                        //如果数据没有发生变化，则不需要推出变化，不需要执行增量
                        match self.execute_incr_update(params).await {
                            Ok(true) => {
                                //执行没问题了，再更新当前的版本记录，headers直接存本地json
                                for (path, new_header) in new_headers {
                                    let file_name = path.file_stem().unwrap().to_str().unwrap();
                                    let dbno = new_header.pdms_header.db_num as u32;
                                    if path.is_dir() {
                                        continue;
                                    }

                                    // ===== P0修复: 并发安全优化 =====
                                    // 先快速读取旧值
                                    let prev_sesno_opt = self
                                        .watcher
                                        .headers
                                        .get(&path)
                                        .map(|entry| entry.latest_ses_data.sesno);

                                    let Some(prev_sesno) = prev_sesno_opt else {
                                        // 文件不在 headers 中，可能是新文件，跳过此处理
                                        continue;
                                    };

                                    let new_sesno = new_header.latest_ses_data.sesno;

                                    // 未发生修改，直接跳过
                                    if prev_sesno >= new_sesno {
                                        continue;
                                    }

                                    // 更新 headers（快速操作）
                                    self.watcher.headers.insert(path.clone(), new_header);

                                    // P0修复: 使缓存失效，确保下次查询获取最新sesno
                                    self.sesno_cache.invalidate(dbno);

                                    // 发生修改的文件，重新生成archive
                                    let output: PathBuf =
                                        format!("assets/archives/{}.cba", file_name).into();

                                    let compress_opt = CompressOptions::new(
                                        path.clone(),
                                        output.clone(),
                                        "assets/temp",
                                    );

                                    // P0修复: 压缩失败时记录错误并加入重试队列
                                    let file_hash =
                                        match execute_compress(compress_opt.clone()).await {
                                            Ok(hash) => hash.to_string(),
                                            Err(e) => {
                                                eprintln!(
                                                    "❌ 压缩失败: file={}, error={:?}",
                                                    file_name, e
                                                );

                                                // 创建失败任务并加入队列
                                                let failed_task = FailedTask::new(
                                                    FailedTaskType::Compression {
                                                        input_path: path.clone(),
                                                        output_path: output.clone(),
                                                        sesno_range: format!(
                                                            "{}..={}",
                                                            prev_sesno + 1,
                                                            new_sesno
                                                        ),
                                                    },
                                                    format!("CBA压缩失败: {:?}", e),
                                                )
                                                .with_metadata(serde_json::json!({
                                                    "file_name": file_name,
                                                    "db_num": dbno,
                                                    "is_new_file": false,
                                                    "prev_sesno": prev_sesno,
                                                    "new_sesno": new_sesno,
                                                }));

                                                self.failed_queue.push(failed_task).await;
                                                continue;
                                            }
                                        };

                                    #[cfg(feature = "web_server")]
                                    {
                                        let archive_size = std::fs::metadata(&output)
                                            .map(|m: std::fs::Metadata| m.len())
                                            .unwrap_or(0);
                                        let delta = new_sesno.saturating_sub(prev_sesno) as u64;

                                        // 查询增删改统计
                                        let stats = Self::query_sesno_range_stats(
                                            prev_sesno + 1,
                                            new_sesno,
                                        )
                                        .await
                                        .unwrap_or_default();

                                        println!(
                                            "📊 会话范围 {}-{}: 新增 {}, 修改 {}, 删除 {}",
                                            prev_sesno + 1,
                                            new_sesno,
                                            stats.total_added,
                                            stats.total_modified,
                                            stats.total_deleted
                                        );

                                        generated_artifacts.push(GeneratedSyncArtifact {
                                            path: output.clone(),
                                            file_name: format!("{file_name}.cba"),
                                            file_size: archive_size,
                                            file_hash: Some(file_hash.clone()),
                                            record_count: if delta > 0 {
                                                Some(delta)
                                            } else {
                                                None
                                            },
                                            db_num: Some(dbno),
                                            db_path: path.to_str().map(|s| s.to_string()),
                                            old_sesno: Some(prev_sesno),
                                            new_sesno: Some(new_sesno),
                                            session_range: Some(format!(
                                                "{}-{}",
                                                prev_sesno + 1,
                                                new_sesno
                                            )),
                                            generated_at: Some(SystemTime::now()),
                                            is_full_sync: false, // 标记为增量同步
                                            total_added: Some(stats.total_added),
                                            total_modified: Some(stats.total_modified),
                                            total_deleted: Some(stats.total_deleted),
                                        });
                                    }

                                    // 如果location_dbs为空，则不进行筛选
                                    // 说明是所有地区都推送，跳过检查
                                    // 必须要是地区对应的dbnos才能推送
                                    if let Some(location_dbs) = &get_db_option().location_dbs {
                                        if !location_dbs.contains(&dbno) {
                                            continue;
                                        }
                                    }

                                    // 检查数据库类型是否在允许同步推送的类型列表中
                                    // 从文件路径解析数据库类型
                                    let db_basic_info = match parse_db_basic_info(path.clone()) {
                                        Ok(info) => info,
                                        Err(e) => {
                                            eprintln!(
                                                "⚠️  跳过推送文件 {}: 无法解析数据库基本信息: {}",
                                                file_name, e
                                            );
                                            continue;
                                        }
                                    };
                                    let db_type = db_basic_info.db_type;

                                    // 从配置读取允许推送的数据库类型列表
                                    let allowed_db_types = Self::get_sync_push_db_types();
                                    if !allowed_db_types.is_empty()
                                        && !allowed_db_types.contains(&db_type)
                                    {
                                        println!(
                                            "⏭️  跳过推送: {} (数据库类型 {} 不在允许的推送类型列表中)",
                                            file_name, db_type
                                        );
                                        continue;
                                    }

                                    // 数据库里不存在这个file hash的记录，才需要发送
                                    // 是自己创建的，在记录里还没有的，才能发送消息出去
                                    // 如果是别的创建的，就应该跳过
                                    // 修复：检查是否有其他节点已经收到过这个消息，而不是仅检查当前节点是否已发送
                                    // 这样可以确保即使当前节点已经发送过，如果其他节点还没有收到，也会继续推送
                                    let current_location = get_db_option().location.as_str();
                                    
                                    // 获取所有目标节点列表（排除当前节点）
                                    #[cfg(feature = "web_server")]
                                    let target_locations = {
                                        use crate::web_server::remote_sync_handlers;
                                        match remote_sync_handlers::list_all_sites().await {
                                            Ok(sites) => {
                                                sites.iter()
                                                    .filter_map(|site| site.location.clone())
                                                    .filter(|loc| loc != current_location)
                                                    .collect::<Vec<String>>()
                                            }
                                            Err(e) => {
                                                eprintln!("⚠️ 获取目标节点列表失败: {:?}，将跳过去重检查", e);
                                                Vec::new()
                                            }
                                        }
                                    };
                                    #[cfg(not(feature = "web_server"))]
                                    let target_locations: Vec<String> = Vec::new();

                                    // 检查是否有其他节点已经收到过这个文件
                                    let should_skip = if target_locations.is_empty() {
                                        // 如果没有配置目标节点，检查是否有任何其他节点（除了当前节点）已经收到过这个消息
                                        // 如果没有其他节点收到过，就应该发送 MQTT 消息（因为 MQTT 是广播的）
                                        let sql = format!(
                                            "select value <string> id from (select * from e3d_sync where location != '{}' and '{}' in file_names and '{}' in file_hashes order by timestamp desc LIMIT 1) ",
                                            current_location,
                                            file_name,
                                            &file_hash
                                        );
                                        let id = match SUL_DB.query(&sql).await {
                                            Ok(mut resp) => {
                                                resp.take::<Vec<String>>(0).unwrap_or_default()
                                            }
                                            Err(e) => {
                                                eprintln!(
                                                    "❌ 去重查询失败: file={}, error={:?}",
                                                    file_name, e
                                                );
                                                Vec::new() // 查询失败时，不跳过推送
                                            }
                                        };
                                        // 如果有其他节点已经收到过，才跳过推送
                                        !id.is_empty()
                                    } else {
                                        // 检查所有目标节点是否都已经收到过这个文件
                                        let mut all_received = true;
                                        for target_loc in &target_locations {
                                            let sql = format!(
                                                "select value <string> id from (select * from e3d_sync where location = '{}' and '{}' in file_names and '{}' in file_hashes order by timestamp desc LIMIT 1) ",
                                                target_loc,
                                                file_name,
                                                &file_hash
                                            );
                                            let mut response = match SUL_DB.query(&sql).await {
                                                Ok(resp) => resp,
                                                Err(e) => {
                                                    eprintln!(
                                                        "❌ 去重查询失败: file={}, target={}, error={:?}",
                                                        file_name, target_loc, e
                                                    );
                                                    all_received = false; // 查询失败时，认为未收到
                                                    break;
                                                }
                                            };
                                            let id = response.take::<Vec<String>>(0).unwrap_or_default();
                                            if id.is_empty() {
                                                all_received = false;
                                                break;
                                            }
                                        }
                                        all_received
                                    };

                                    if !should_skip {
                                        println!("发生了增量更新，推送：{} (文件hash: {}, 当前节点: {})", &file_name, &file_hash, current_location);
                                        notify_file_hashes.push(file_hash);
                                        notify_file_names.push(file_name.to_owned());
                                    } else {
                                        println!("⏭️  跳过推送: {} (所有目标节点已收到此文件，文件hash: {}, 当前节点: {})", &file_name, &file_hash, current_location);
                                    }
                                }
                                // now save the watch.json
                                // self.watcher.save(None).expect("save watch.json failed");
                            }
                            Ok(false) => {
                                println!("{:?} 文件发生修改，但是没有发生增量更新。", &event.paths);
                                continue;
                            }
                            Err(e) => {
                                println!("Execute increment update error: {:?}", e);
                            }
                        }
                        //publish notify db file updates
                        dbg!(&notify_file_names);
                        #[cfg(feature = "mqtt")]
                        if !notify_file_names.is_empty() {
                            let payload =
                                SyncE3dFileMsg::new(notify_file_names.clone(), notify_file_hashes.clone());
                            //自己本地也要保存
                            // todo 后续还是要配置哪些dbs，哪个地方能修改，哪个地方是不能改的
                            SUL_DB
                                .query(format!(
                                    "INSERT IGNORE INTO e3d_sync {} ",
                                    serde_json::to_string(&payload).unwrap()
                                ))
                                .await
                                .unwrap();
                            //todo 检查是否只是发生了claim page的变化，如果只是claim修改，是需要每次都同步？
                            //会导致出现循环
                            println!(
                                "📤 准备通过 MQTT 推送增量更新 (文件数: {}, 会话范围: {:?}, 来源节点: {})",
                                payload.file_names.len(),
                                payload.session_range,
                                payload.location
                            );

                            // 使用全局 MQTT Publisher Client（由 start_mqtt_publisher 初始化）
                            let mqtt_client_opt = {
                                use crate::data_interface::db_model::MQTT_PUBLISHER_CLIENT;
                                MQTT_PUBLISHER_CLIENT.read().await.clone()
                            };
                            
                            if let Some(mqtt_client) = mqtt_client_opt {
                                match publish_sync_payload_with_retry(
                                    mqtt_client,
                                    payload.clone(),
                                )
                                .await
                                {
                                    Ok(()) => {
                                        // 成功日志已在 publish_sync_payload_with_retry 内部输出
                                    }
                                    Err(e) => {
                                        eprintln!("❌ MQTT 发布失败（已重试3次）: {:?}", e);
                                        eprintln!(
                                            "   文件数: {}, 会话范围: {:?}, 来源节点: {}",
                                            payload.file_names.len(),
                                            payload.session_range,
                                            payload.location
                                        );
                                    }
                                }
                            } else {
                                println!("⚠️ MQTT Publisher 未初始化，跳过推送（请先启动 MQTT 订阅）");
                            }
                        }
                        #[cfg(feature = "web_server")]
                        {
                            // 🔔 检测到增量变化，立即通知前端（Running 状态）
                            if !notify_file_names.is_empty() {
                                let mut center =
                                    crate::web_server::sync_control_center::SYNC_CONTROL_CENTER
                                        .write()
                                        .await;
                                if let Some(ref progress_hub) = center.progress_hub {
                                    let task_id = "increment-updates".to_string();

                                    // 注册任务（如果尚未注册）
                                    if !progress_hub.has_task(&task_id) {
                                        progress_hub.register(task_id.clone());
                                    }

                                    // 发送 Running 状态消息
                                    let running_msg = ProgressMessage {
                                        task_id: task_id.clone(),
                                        status: TaskStatus::Running,
                                        percentage: 50.0,
                                        current_step: format!(
                                            "正在处理增量更新: {} 个文件",
                                            notify_file_names.len()
                                        ),
                                        current_step_number: 1,
                                        total_steps: 2,
                                        processed_items: 0,
                                        total_items: notify_file_names.len() as u64,
                                        message: notify_file_names.join(", "),
                                        timestamp: chrono::Utc::now(),
                                        details: None,
                                    };

                                    match progress_hub.publish(running_msg) {
                                        Ok(count) => {
                                            println!(
                                                "🔔 已通知前端开始增量更新 (推送给 {} 个客户端)",
                                                count
                                            );
                                        }
                                        Err(e) => {
                                            println!("⚠️  通知前端失败: {}", e);
                                        }
                                    }
                                }
                            }

                            // 先保存同步历史记录到 e3d_sync 表
                            if !generated_artifacts.is_empty() {
                                // 收集所有已生成归档的文件名和哈希，以及统计信息
                                let mut all_file_names = vec![];
                                let mut all_file_hashes = vec![];
                                let mut session_ranges = vec![];
                                let mut db_nums = vec![];
                                let mut is_full_sync = false;
                                let mut total_added = 0u32;
                                let mut total_modified = 0u32;
                                let mut total_deleted = 0u32;

                                for artifact in &generated_artifacts {
                                    // 从 artifact.file_name 中提取实际文件名（去掉.cba后缀）
                                    let file_name = artifact
                                        .file_name
                                        .strip_suffix(".cba")
                                        .unwrap_or(&artifact.file_name);
                                    all_file_names.push(file_name.to_string());
                                    if let Some(hash) = &artifact.file_hash {
                                        all_file_hashes.push(hash.clone());
                                    }
                                    if let Some(range) = &artifact.session_range {
                                        session_ranges.push(range.clone());
                                    }
                                    if let Some(db_num) = artifact.db_num {
                                        db_nums.push(db_num);
                                    }
                                    if artifact.is_full_sync {
                                        is_full_sync = true;
                                    }
                                    // 聚合统计信息
                                    if let Some(added) = artifact.total_added {
                                        total_added += added;
                                    }
                                    if let Some(modified) = artifact.total_modified {
                                        total_modified += modified;
                                    }
                                    if let Some(deleted) = artifact.total_deleted {
                                        total_deleted += deleted;
                                    }
                                }

                                let mut payload = SyncE3dFileMsg::new(
                                    all_file_names.clone(),
                                    all_file_hashes.clone(),
                                );

                                // 填充扩展统计信息
                                if !session_ranges.is_empty() {
                                    payload.session_range = Some(session_ranges.join(", "));
                                }
                                if !db_nums.is_empty() {
                                    payload.db_num = Some(db_nums[0]); // 取第一个 db_num
                                }
                                payload.is_full_sync = Some(is_full_sync);
                                // 保存增删改统计
                                if total_added > 0 || total_modified > 0 || total_deleted > 0 {
                                    payload.total_added = Some(total_added);
                                    payload.total_modified = Some(total_modified);
                                    payload.total_deleted = Some(total_deleted);
                                }

                                if let Err(e) = SUL_DB
                                    .query(format!(
                                        "INSERT IGNORE INTO e3d_sync {} ",
                                        serde_json::to_string(&payload).unwrap()
                                    ))
                                    .await
                                {
                                    println!("❌ 记录 e3d_sync 历史失败: {:?}", e);
                                } else {
                                    let sync_type = if is_full_sync {
                                        "完全同步"
                                    } else {
                                        "增量同步"
                                    };
                                    let range_info = if !session_ranges.is_empty() {
                                        format!(" (会话范围: {})", session_ranges.join(", "))
                                    } else {
                                        String::new()
                                    };
                                    let stats_info = if total_added > 0
                                        || total_modified > 0
                                        || total_deleted > 0
                                    {
                                        format!(
                                            " [新增:{} 修改:{} 删除:{}]",
                                            total_added, total_modified, total_deleted
                                        )
                                    } else {
                                        String::new()
                                    };
                                    println!(
                                        "✅ 同步历史已保存到 e3d_sync 表: {} 个文件 [{}]{}{}",
                                        all_file_names.len(),
                                        sync_type,
                                        range_info,
                                        stats_info
                                    );

                                    let mut center =
                                        crate::web_server::sync_control_center::SYNC_CONTROL_CENTER
                                            .write()
                                            .await;
                                    center.add_log(
                                        "INFO",
                                        format!(
                                            "同步记录已保存: {} 个文件 [{}]{}{}",
                                            all_file_names.len(),
                                            sync_type,
                                            range_info,
                                            stats_info
                                        ),
                                    );

                                    // 🎯 新增: 通过 ProgressHub 广播增量更新事件到 WebSocket 客户端
                                    if let Some(ref progress_hub) = center.progress_hub {
                                        // 使用固定的 task_id，方便前端订阅
                                        let task_id = "increment-updates".to_string();
                                        let step_message = format!(
                                            "检测到增量更新: {} 个文件{}{}",
                                            all_file_names.len(),
                                            range_info,
                                            stats_info
                                        );

                                        // 如果任务不存在则注册，否则直接发布
                                        if !progress_hub.has_task(&task_id) {
                                            progress_hub.register(task_id.clone());
                                        }

                                        let progress_msg = ProgressMessage {
                                            task_id: task_id.clone(),
                                            status: TaskStatus::Completed,
                                            percentage: 100.0,
                                            current_step: step_message.clone(),
                                            current_step_number: 1,
                                            total_steps: 1,
                                            processed_items: all_file_names.len() as u64,
                                            total_items: all_file_names.len() as u64,
                                            message: all_file_names.join(", "),
                                            timestamp: chrono::Utc::now(),
                                            details: None,
                                        };

                                        match progress_hub.publish(progress_msg) {
                                            Ok(count) => {
                                                println!(
                                                    "📡 已通过 WebSocket 推送增量更新通知给 {} 个客户端: {}",
                                                    count, &step_message
                                                );
                                            }
                                            Err(e) => {
                                                println!("⚠️ WebSocket 推送失败: {}", e);
                                            }
                                        }

                                        // 注意：不取消注册，保持任务一直存在以便前端订阅
                                    }

                                    // 🔔 可选: 发送 SSE 事件到本地前端（用于Web UI实时刷新）
                                    // 注意: 这是本地前端功能，跨站点通知应通过 MQTT
                                    use crate::web_server::sync_control_center::{
                                        SYNC_EVENT_TX, SyncEvent,
                                    };
                                    let sse_event = SyncEvent::SyncCompleted {
                                        task_id: "increment-updates".to_string(),
                                        file_path: all_file_names.join(", "),
                                        duration_ms: 0, // TODO: 计算实际耗时
                                        timestamp: chrono::Utc::now().to_rfc3339(),
                                    };
                                    match SYNC_EVENT_TX.send(sse_event) {
                                        Ok(count) => {
                                            if count > 0 {
                                                println!(
                                                    "📡 已通过 SSE 推送增量更新通知给 {} 个本地前端客户端 (文件: {})",
                                                    count,
                                                    all_file_names.join(", ")
                                                );
                                            }
                                            // 如果没有客户端连接，静默处理（这是可选的本地功能）
                                        }
                                        Err(_) => {
                                            // SSE 推送失败是正常的（如果没有前端连接），不输出错误日志
                                            // 跨站点通知应通过 MQTT，SSE 仅用于本地前端实时刷新
                                        }
                                    }
                                }
                            }

                            // 然后将任务加入队列
                            enqueue_generated_sync_tasks(generated_artifacts).await;
                        }
                    } else {
                        println!("扫描数据库头部失败，错误路径: {:?}", &event.paths);
                    }
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }

        Ok(())
    }

    /// 重试失败任务
    ///
    /// 根据任务类型执行相应的重试逻辑
    /// 解析 sesno_range 字符串 (格式: "start..=end")
    fn parse_sesno_range(range_str: &str) -> anyhow::Result<RangeInclusive<i32>> {
        let parts: Vec<&str> = range_str.split("..=").collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("无效的range格式: {}", range_str));
        }
        let start: i32 = parts[0]
            .parse()
            .with_context(|| format!("无法解析起始值: {}", parts[0]))?;
        let end: i32 = parts[1]
            .parse()
            .with_context(|| format!("无法解析结束值: {}", parts[1]))?;
        Ok(start..=end)
    }

    async fn retry_failed_task(&self, task: &FailedTask) -> anyhow::Result<()> {
        match &task.task_type {
            FailedTaskType::DatabaseQuery { dbnum, operation } => {
                eprintln!(
                    "🔄 重试数据库查询: dbnum={}, operation={}",
                    dbnum, operation
                );

                // 重试查询最新会话号
                if operation == "query_latest_sesno" {
                    Self::query_latest_sesno_by_dbnum(*dbnum).await?;
                    eprintln!("✅ 数据库查询重试成功: dbnum={}", dbnum);
                }

                Ok(())
            }

            FailedTaskType::Compression {
                input_path,
                output_path,
                sesno_range,
            } => {
                eprintln!(
                    "🔄 重试CBA压缩: input={}, output={}, range={}",
                    input_path.display(),
                    output_path.display(),
                    sesno_range
                );

                // 重新执行压缩
                let compress_opt =
                    CompressOptions::new(input_path.clone(), output_path.clone(), "assets/temp");

                execute_compress(compress_opt).await?;
                eprintln!("✅ CBA压缩重试成功: {}", output_path.display());

                Ok(())
            }

            FailedTaskType::IncrementUpdate {
                path,
                sesno_range,
                dbnum,
            } => {
                eprintln!(
                    "🔄 重试增量更新: file={}, range={}, dbnum={}",
                    path.display(),
                    sesno_range,
                    dbnum
                );

                // 解析 sesno_range 字符串 (格式: "start..=end")
                let range = Self::parse_sesno_range(sesno_range)?;

                // 重新读取文件头信息
                let mut io = PdmsIO::new("", path.clone(), true);
                io.open()
                    .map_err(|e| anyhow::anyhow!("打开文件失败: {}", e))?;
                let basic_info = io.get_page_basic_info()?;

                // 构造增量更新参数
                let mut increment_map = IndexMap::new();
                increment_map.insert(path.clone(), (basic_info, range));

                // 执行增量更新
                self.execute_incr_update(increment_map).await?;
                eprintln!("✅ 增量更新重试成功: file={}", path.display());

                Ok(())
            }

            FailedTaskType::MqttPublish {
                topic,
                payload_summary,
            } => {
                eprintln!(
                    "🔄 重试MQTT推送: topic={}, payload={}",
                    topic, payload_summary
                );

                // 注意: MQTT推送失败通常是由于网络问题导致的
                // 由于我们只保存了payload摘要而非完整payload,无法直接重试发送
                // 实际的解决方案是在文件监控循环中重新检测增量并发送
                // 这里返回错误,让任务保留在队列中,等待下次文件变化时自然触发

                eprintln!("ℹ️ MQTT推送重试需要依赖文件变化重新触发");
                Err(anyhow::anyhow!(
                    "MQTT推送无法直接重试(缺少完整payload),需等待文件变化触发"
                ))
            }
        }
    }

    /// 启动后台重试线程
    ///
    /// 每60秒扫描一次失败任务队列，对符合重试条件的任务进行重试
    pub async fn start_retry_worker(self: Arc<Self>) {
        let queue = self.failed_queue.clone();
        let mgr = self.clone();

        tokio::spawn(async move {
            eprintln!("🚀 失败任务重试worker已启动");

            loop {
                // 每60秒扫描一次
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;

                // 获取待重试的任务
                let pending = queue.get_pending_tasks().await;

                if pending.is_empty() {
                    continue;
                }

                eprintln!("📋 发现 {} 个待重试任务", pending.len());

                // 处理每个待重试任务
                for mut task in pending {
                    eprintln!(
                        "🔄 开始重试任务: {} (重试次数: {}/{})",
                        task.description(),
                        task.retry_count,
                        task.max_retries
                    );

                    // 执行重试
                    let result = mgr.retry_failed_task(&task).await;

                    match result {
                        Ok(()) => {
                            // 重试成功，移除任务
                            eprintln!("✅ 任务重试成功，已从队列移除: {}", task.id);
                            queue.remove(&task.id).await;
                        }
                        Err(e) => {
                            // 重试失败，更新任务状态
                            task.error = format!("重试失败: {:?}", e);
                            task.schedule_next_retry();

                            if task.is_exhausted() {
                                eprintln!(
                                    "❌ 任务已达到最大重试次数 ({}/{}): {}",
                                    task.retry_count,
                                    task.max_retries,
                                    task.description()
                                );
                            } else {
                                let next_retry_secs = task
                                    .next_retry_at
                                    .duration_since(std::time::SystemTime::now())
                                    .unwrap_or_default()
                                    .as_secs();

                                eprintln!(
                                    "⏰ 任务重试失败，将在 {} 秒后再次尝试 (重试次数: {}/{}): {}",
                                    next_retry_secs,
                                    task.retry_count,
                                    task.max_retries,
                                    task.description()
                                );
                            }

                            queue.update(task).await;
                        }
                    }
                }

                // 检查已耗尽重试次数的任务（用于告警）
                let exhausted = queue.get_exhausted_tasks().await;
                if !exhausted.is_empty() {
                    eprintln!("⚠️ 有 {} 个任务已达到最大重试次数:", exhausted.len());
                    for task in exhausted {
                        eprintln!("  - [{}] {}: {}", task.id, task.description(), task.error);
                    }
                }

                // 打印队列统计
                let stats = queue.get_stats().await;
                eprintln!(
                    "📊 失败任务队列统计: 总数={}, 待重试={}, 等待中={}, 已耗尽={}",
                    stats.total, stats.pending, stats.waiting, stats.exhausted
                );
            }
        });
    }

    /// 启动全量解析后台 Worker
    ///
    /// 创建 channel 并启动 worker，返回 sender 供后续使用
    ///
    /// # 返回值
    /// - `FullParseSender`: 用于发送全量解析任务的 sender
    #[cfg(any(feature = "mqtt", feature = "web_server"))]
    pub fn start_full_parse_worker(&mut self) -> crate::data_interface::full_parse_worker::FullParseSender {
        use crate::data_interface::full_parse_worker::{
            create_full_parse_channel, FullParseWorker,
        };

        let (sender, receiver) = create_full_parse_channel();
        
        // 创建 worker 并启动
        let worker = FullParseWorker::new(
            receiver,
            None, // 暂不需要结果回调
            self.failed_queue.clone(),
        );
        worker.start();

        // 保存 sender 到 self
        self.full_parse_sender = Some(sender.clone());

        eprintln!("🚀 全量解析后台 Worker 已初始化");
        sender
    }
}
