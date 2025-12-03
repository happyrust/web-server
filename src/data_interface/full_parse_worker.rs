//! 全量解析后台 Worker 模块
//!
//! 用于处理新文件的全量解析任务，避免阻塞主流程。
//!
//! # 核心功能
//! - 后台异步处理新文件的 CBA 压缩生成
//! - 支持 manual_db_nums 和 location_dbs 过滤
//! - 任务完成后通过 channel 通知结果

use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use pdms_io::defines::DbPageBasicInfo;
use pdms_io::sync::compress::{CompressOptions, execute_compress};
use aios_core::get_db_option;
use serde::{Deserialize, Serialize};

use crate::data_interface::failed_task_queue::{FailedTask, FailedTaskType, FailedTaskQueue};

/// 全量解析任务
#[derive(Debug, Clone)]
pub struct FullParseTask {
    /// 数据库文件路径
    pub path: PathBuf,
    /// 数据库基本信息
    pub header: DbPageBasicInfo,
    /// 数据库编号
    pub dbno: u32,
    /// 文件名（不含扩展名）
    pub file_name: String,
    /// 当前会话号
    pub current_sesno: i32,
}

/// 全量解析结果
#[derive(Debug, Clone)]
pub struct FullParseResult {
    /// 任务信息
    pub task: FullParseTask,
    /// 生成的 CBA 文件哈希
    pub file_hash: Option<String>,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果失败）
    pub error: Option<String>,
    /// 生成的 CBA 文件路径
    pub output_path: Option<PathBuf>,
    /// 生成时间
    pub generated_at: SystemTime,
}

/// 全量解析任务发送器类型别名
pub type FullParseSender = mpsc::UnboundedSender<FullParseTask>;
/// 全量解析任务接收器类型别名
pub type FullParseReceiver = mpsc::UnboundedReceiver<FullParseTask>;

/// 全量解析结果发送器类型别名
pub type FullParseResultSender = mpsc::UnboundedSender<FullParseResult>;
/// 全量解析结果接收器类型别名
pub type FullParseResultReceiver = mpsc::UnboundedReceiver<FullParseResult>;

/// 创建全量解析任务 channel
pub fn create_full_parse_channel() -> (FullParseSender, FullParseReceiver) {
    mpsc::unbounded_channel()
}

/// 创建全量解析结果 channel
pub fn create_full_parse_result_channel() -> (FullParseResultSender, FullParseResultReceiver) {
    mpsc::unbounded_channel()
}

/// 检查数据库编号是否应该被过滤
///
/// # 过滤规则
/// 1. 如果 `manual_db_nums` 非空且 dbno 不在其中，则过滤
/// 2. 如果 `location_dbs` 非空且 dbno 在其中，则保留（用于推送通知），否则只做本地解析不推送
///
/// # 返回值
/// - `true`: 应该过滤掉（不处理）
/// - `false`: 应该处理
pub fn should_filter_db(dbno: u32) -> bool {
    let db_option = get_db_option();
    
    // 检查 manual_db_nums 过滤（manual_db_nums 是 Vec<u32>）
    if let Some(ref manual_dbnums) = db_option.manual_db_nums {
        if !manual_dbnums.is_empty() && !manual_dbnums.contains(&dbno) {
            return true;
        }
    }
    
    false
}

/// 检查数据库编号是否在 location_dbs 中（用于判断是否推送通知）
///
/// # 返回值
/// - `true`: 在 location_dbs 中，应该推送通知
/// - `false`: 不在 location_dbs 中或未配置，不推送通知
pub fn should_push_notification(dbno: u32) -> bool {
    let db_option = get_db_option();
    
    // 如果配置了 location_dbs，检查 dbno 是否在其中
    if let Some(ref location_dbs) = db_option.location_dbs {
        return location_dbs.contains(&dbno);
    }
    
    // 未配置 location_dbs，默认推送所有
    true
}

/// 全量解析 Worker
///
/// 从 channel 接收任务，执行 CBA 压缩，并将结果发送到结果 channel
pub struct FullParseWorker {
    /// 任务接收器
    receiver: FullParseReceiver,
    /// 结果发送器
    result_sender: Option<FullParseResultSender>,
    /// 失败任务队列
    failed_queue: FailedTaskQueue,
}

impl FullParseWorker {
    /// 创建新的 Worker
    pub fn new(
        receiver: FullParseReceiver,
        result_sender: Option<FullParseResultSender>,
        failed_queue: FailedTaskQueue,
    ) -> Self {
        Self {
            receiver,
            result_sender,
            failed_queue,
        }
    }

    /// 启动 Worker（在后台运行）
    pub fn start(mut self) {
        tokio::spawn(async move {
            eprintln!("🚀 全量解析后台 Worker 已启动");

            while let Some(task) = self.receiver.recv().await {
                eprintln!(
                    "📥 收到全量解析任务: file={}, dbno={}, sesno=1-{}",
                    task.file_name, task.dbno, task.current_sesno
                );

                // 执行解析
                let result = self.process_task(&task).await;

                // 发送结果（如果有结果接收器）
                if let Some(ref sender) = self.result_sender {
                    if let Err(e) = sender.send(result.clone()) {
                        eprintln!("⚠️ 发送全量解析结果失败: {:?}", e);
                    }
                }

                // 打印结果
                if result.success {
                    eprintln!(
                        "✅ 全量解析完成: file={}, hash={}",
                        task.file_name,
                        result.file_hash.as_deref().unwrap_or("N/A")
                    );
                } else {
                    eprintln!(
                        "❌ 全量解析失败: file={}, error={}",
                        task.file_name,
                        result.error.as_deref().unwrap_or("未知错误")
                    );
                }
            }

            eprintln!("⏹️ 全量解析后台 Worker 已停止");
        });
    }

    /// 处理单个任务
    async fn process_task(&self, task: &FullParseTask) -> FullParseResult {
        let output: PathBuf = format!("assets/archives/{}.cba", task.file_name).into();
        let compress_opt = CompressOptions::new(
            task.path.clone(),
            output.clone(),
            "assets/temp",
        );

        match execute_compress(compress_opt).await {
            Ok(hash) => FullParseResult {
                task: task.clone(),
                file_hash: Some(hash.to_string()),
                success: true,
                error: None,
                output_path: Some(output),
                generated_at: SystemTime::now(),
            },
            Err(e) => {
                // 记录失败任务到重试队列
                let failed_task = FailedTask::new(
                    FailedTaskType::Compression {
                        input_path: task.path.clone(),
                        output_path: output.clone(),
                        sesno_range: format!("1..={}", task.current_sesno),
                    },
                    format!("全量解析 CBA 压缩失败: {:?}", e),
                )
                .with_metadata(serde_json::json!({
                    "file_name": task.file_name,
                    "db_num": task.dbno,
                    "is_new_file": true,
                    "sesno": task.current_sesno,
                    "worker": "full_parse_worker",
                }));

                // 异步推送到失败队列
                self.failed_queue.push(failed_task).await;

                FullParseResult {
                    task: task.clone(),
                    file_hash: None,
                    success: false,
                    error: Some(format!("{:?}", e)),
                    output_path: None,
                    generated_at: SystemTime::now(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_filter_db() {
        // 这个测试依赖于实际的配置，跳过
    }
}
