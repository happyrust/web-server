//! 失败任务队列模块
//!
//! 用于处理增量更新过程中的失败任务，提供自动重试和持久化能力。
//!
//! # 核心功能
//! - 失败任务记录和持久化
//! - 指数退避重试策略（1→2→4→8→16分钟）
//! - 断电恢复（JSON持久化）
//! - 最大重试次数限制（默认5次）
//!
//! # 使用示例
//! ```rust
//! let queue = FailedTaskQueue::new(PathBuf::from("assets/failed_tasks.json"));
//!
//! // 添加失败任务
//! let task = FailedTask::new(
//!     FailedTaskType::DatabaseQuery {
//!         dbnum: 1001,
//!         operation: "query_sesno".to_string(),
//!     },
//!     "Database connection timeout",
//! );
//! queue.push(task).await;
//!
//! // 后台重试
//! queue.start_retry_worker(retry_handler).await;
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// 失败任务类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FailedTaskType {
    /// 数据库查询失败
    DatabaseQuery {
        /// 数据库编号
        dbnum: u32,
        /// 操作类型 (如 "query_sesno", "update_elements")
        operation: String,
    },

    /// CBA压缩失败
    Compression {
        /// 输入文件路径
        input_path: PathBuf,
        /// 输出文件路径
        output_path: PathBuf,
        /// 会话号范围 (如 "12341..=12350")
        sesno_range: String,
    },

    /// 增量更新失败
    IncrementUpdate {
        /// 文件路径
        path: PathBuf,
        /// 会话号范围
        sesno_range: String,
        /// 数据库编号
        dbnum: u32,
    },

    /// MQTT推送失败
    MqttPublish {
        /// MQTT主题
        topic: String,
        /// 负载摘要
        payload_summary: String,
    },
}

impl FailedTaskType {
    /// 获取任务类型的简短描述
    pub fn description(&self) -> String {
        match self {
            Self::DatabaseQuery { dbnum, operation } => {
                format!("DB查询失败 (dbnum={}, op={})", dbnum, operation)
            }
            Self::Compression {
                input_path,
                sesno_range,
                ..
            } => {
                format!(
                    "CBA压缩失败 (file={}, range={})",
                    input_path.display(),
                    sesno_range
                )
            }
            Self::IncrementUpdate {
                path,
                sesno_range,
                dbnum,
            } => {
                format!(
                    "增量更新失败 (file={}, range={}, dbnum={})",
                    path.display(),
                    sesno_range,
                    dbnum
                )
            }
            Self::MqttPublish { topic, .. } => {
                format!("MQTT推送失败 (topic={})", topic)
            }
        }
    }
}

/// 失败任务记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedTask {
    /// 任务ID (UUID)
    pub id: String,

    /// 任务类型
    pub task_type: FailedTaskType,

    /// 错误信息
    pub error: String,

    /// 错误堆栈 (如果有)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_trace: Option<String>,

    /// 重试次数
    pub retry_count: u32,

    /// 最大重试次数
    pub max_retries: u32,

    /// 首次失败时间 (Unix timestamp in seconds)
    #[serde(with = "timestamp_serde")]
    pub first_failed_at: SystemTime,

    /// 最后重试时间 (Unix timestamp in seconds)
    #[serde(
        default,
        with = "option_timestamp_serde",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_retry_at: Option<SystemTime>,

    /// 下次重试时间 (Unix timestamp in seconds)
    #[serde(with = "timestamp_serde")]
    pub next_retry_at: SystemTime,

    /// 优先级 (1-10, 10最高)
    pub priority: u8,

    /// 任务元数据 (JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl FailedTask {
    /// 创建新的失败任务
    pub fn new(task_type: FailedTaskType, error: impl ToString) -> Self {
        let now = SystemTime::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_type,
            error: error.to_string(),
            error_trace: None,
            retry_count: 0,
            max_retries: 5,
            first_failed_at: now,
            last_retry_at: None,
            next_retry_at: now + Duration::from_secs(60), // 1分钟后重试
            priority: 5,
            metadata: None,
        }
    }

    /// 设置错误堆栈
    pub fn with_trace(mut self, trace: impl ToString) -> Self {
        self.error_trace = Some(trace.to_string());
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(10);
        self
    }

    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// 计算下次重试时间 (指数退避)
    pub fn schedule_next_retry(&mut self) {
        self.retry_count += 1;
        self.last_retry_at = Some(SystemTime::now());

        // 指数退避: 1min, 2min, 4min, 8min, 16min
        let delay_secs = 60 * (2u64.pow(self.retry_count.min(4)));
        self.next_retry_at = SystemTime::now() + Duration::from_secs(delay_secs);
    }

    /// 判断是否应该重试
    pub fn should_retry(&self) -> bool {
        self.retry_count < self.max_retries && SystemTime::now() >= self.next_retry_at
    }

    /// 判断是否已达到最大重试次数
    pub fn is_exhausted(&self) -> bool {
        self.retry_count >= self.max_retries
    }

    /// 获取任务的简短描述
    pub fn description(&self) -> String {
        self.task_type.description()
    }
}

/// 失败任务队列管理器
#[derive(Clone)]
pub struct FailedTaskQueue {
    /// 内存队列
    tasks: Arc<RwLock<Vec<FailedTask>>>,

    /// 持久化文件路径
    persist_path: PathBuf,
}

impl FailedTaskQueue {
    /// 创建新的失败任务队列
    ///
    /// 如果持久化文件存在，会自动加载之前的失败任务
    pub fn new(persist_path: PathBuf) -> Self {
        let tasks = Self::load_from_disk(&persist_path).unwrap_or_default();

        if !tasks.is_empty() {
            eprintln!("✅ 从磁盘加载了 {} 个失败任务", tasks.len());
        }

        Self {
            tasks: Arc::new(RwLock::new(tasks)),
            persist_path,
        }
    }

    /// 添加失败任务
    pub async fn push(&self, task: FailedTask) {
        let mut tasks = self.tasks.write().await;
        eprintln!("📝 添加失败任务: {} (ID: {})", task.description(), task.id);
        tasks.push(task);
        drop(tasks);

        // 异步持久化
        if let Err(e) = self.persist().await {
            eprintln!("⚠️ 持久化失败任务失败: {:?}", e);
        }
    }

    /// 批量添加失败任务
    pub async fn push_batch(&self, new_tasks: Vec<FailedTask>) {
        if new_tasks.is_empty() {
            return;
        }

        let mut tasks = self.tasks.write().await;
        let count = new_tasks.len();
        tasks.extend(new_tasks);
        drop(tasks);

        eprintln!("📝 批量添加 {} 个失败任务", count);

        if let Err(e) = self.persist().await {
            eprintln!("⚠️ 持久化失败任务失败: {:?}", e);
        }
    }

    /// 获取待重试的任务
    pub async fn get_pending_tasks(&self) -> Vec<FailedTask> {
        let tasks = self.tasks.read().await;
        tasks.iter().filter(|t| t.should_retry()).cloned().collect()
    }

    /// 获取所有任务（用于监控）
    pub async fn get_all_tasks(&self) -> Vec<FailedTask> {
        let tasks = self.tasks.read().await;
        tasks.clone()
    }

    /// 获取任务统计
    pub async fn get_stats(&self) -> TaskQueueStats {
        let tasks = self.tasks.read().await;

        let total = tasks.len();
        let pending = tasks.iter().filter(|t| t.should_retry()).count();
        let exhausted = tasks.iter().filter(|t| t.is_exhausted()).count();
        let waiting = total - pending - exhausted;

        TaskQueueStats {
            total,
            pending,
            waiting,
            exhausted,
        }
    }

    /// 移除任务 (重试成功后)
    pub async fn remove(&self, task_id: &str) {
        let mut tasks = self.tasks.write().await;
        let before = tasks.len();
        tasks.retain(|t| t.id != task_id);
        let after = tasks.len();

        if before > after {
            eprintln!("✅ 移除成功任务: {}", task_id);
        }

        drop(tasks);

        if let Err(e) = self.persist().await {
            eprintln!("⚠️ 持久化失败任务失败: {:?}", e);
        }
    }

    /// 更新任务 (重试失败后)
    pub async fn update(&self, task: FailedTask) {
        let mut tasks = self.tasks.write().await;

        if let Some(existing) = tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task;
        }

        drop(tasks);

        if let Err(e) = self.persist().await {
            eprintln!("⚠️ 持久化失败任务失败: {:?}", e);
        }
    }

    /// 获取已达到最大重试次数的任务 (用于告警)
    pub async fn get_exhausted_tasks(&self) -> Vec<FailedTask> {
        let tasks = self.tasks.read().await;
        tasks.iter().filter(|t| t.is_exhausted()).cloned().collect()
    }

    /// 清理已耗尽的任务（可选，手动触发）
    pub async fn cleanup_exhausted(&self) -> usize {
        let mut tasks = self.tasks.write().await;
        let before = tasks.len();
        tasks.retain(|t| !t.is_exhausted());
        let after = tasks.len();
        let removed = before - after;

        if removed > 0 {
            eprintln!("🧹 清理了 {} 个已耗尽的任务", removed);
        }

        drop(tasks);

        if let Err(e) = self.persist().await {
            eprintln!("⚠️ 持久化失败任务失败: {:?}", e);
        }

        removed
    }

    /// 持久化到磁盘
    async fn persist(&self) -> Result<()> {
        let tasks = self.tasks.read().await;
        let json = serde_json::to_string_pretty(&*tasks).context("序列化失败任务失败")?;

        // 确保目录存在
        if let Some(parent) = self.persist_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("创建失败任务目录失败")?;
        }

        // 原子写入（先写临时文件，再重命名）
        let temp_path = self.persist_path.with_extension("tmp");
        tokio::fs::write(&temp_path, json)
            .await
            .context("写入临时文件失败")?;

        tokio::fs::rename(&temp_path, &self.persist_path)
            .await
            .context("重命名失败任务文件失败")?;

        Ok(())
    }

    /// 从磁盘加载
    fn load_from_disk(path: &Path) -> Option<Vec<FailedTask>> {
        let content = std::fs::read_to_string(path).ok()?;
        match serde_json::from_str(&content) {
            Ok(tasks) => Some(tasks),
            Err(e) => {
                eprintln!("⚠️ 解析失败任务文件失败: {:?}", e);
                None
            }
        }
    }
}

/// 任务队列统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueStats {
    /// 总任务数
    pub total: usize,
    /// 待重试任务数（已到重试时间）
    pub pending: usize,
    /// 等待中任务数（未到重试时间）
    pub waiting: usize,
    /// 已耗尽任务数（达到最大重试次数）
    pub exhausted: usize,
}

// ========== 时间序列化辅助模块 ==========

mod timestamp_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let secs = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        serializer.serialize_u64(secs)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

mod option_timestamp_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match time {
            Some(t) => {
                let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                serializer.serialize_some(&secs)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<u64> = Option::deserialize(deserializer)?;
        Ok(opt.map(|secs| UNIX_EPOCH + std::time::Duration::from_secs(secs)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failed_task_creation() {
        let task = FailedTask::new(
            FailedTaskType::DatabaseQuery {
                dbnum: 1001,
                operation: "query_sesno".to_string(),
            },
            "Database connection timeout",
        );

        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 5);
        assert!(!task.is_exhausted());
    }

    #[test]
    fn test_exponential_backoff() {
        let mut task = FailedTask::new(
            FailedTaskType::DatabaseQuery {
                dbnum: 1001,
                operation: "test".to_string(),
            },
            "Test error",
        );

        // 第1次重试: ~1分钟
        task.schedule_next_retry();
        assert_eq!(task.retry_count, 1);

        // 第2次重试: ~2分钟
        task.schedule_next_retry();
        assert_eq!(task.retry_count, 2);

        // 第3次重试: ~4分钟
        task.schedule_next_retry();
        assert_eq!(task.retry_count, 3);

        // 验证未耗尽
        assert!(!task.is_exhausted());
    }

    #[test]
    fn test_task_exhaustion() {
        let mut task = FailedTask::new(
            FailedTaskType::DatabaseQuery {
                dbnum: 1001,
                operation: "test".to_string(),
            },
            "Test error",
        )
        .with_max_retries(3);

        // 重试3次
        for _ in 0..3 {
            task.schedule_next_retry();
        }

        // 应该已耗尽
        assert!(task.is_exhausted());
        assert_eq!(task.retry_count, 3);
    }

    #[tokio::test]
    async fn test_queue_operations() {
        let temp_file = std::env::temp_dir().join("test_failed_tasks.json");
        let queue = FailedTaskQueue::new(temp_file.clone());

        // 添加任务
        let task = FailedTask::new(
            FailedTaskType::DatabaseQuery {
                dbnum: 1001,
                operation: "test".to_string(),
            },
            "Test error",
        );
        let task_id = task.id.clone();
        queue.push(task).await;

        // 验证统计
        let stats = queue.get_stats().await;
        assert_eq!(stats.total, 1);

        // 移除任务
        queue.remove(&task_id).await;
        let stats = queue.get_stats().await;
        assert_eq!(stats.total, 0);

        // 清理临时文件
        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_task_serialization() {
        let task = FailedTask::new(
            FailedTaskType::Compression {
                input_path: PathBuf::from("/test/input.db"),
                output_path: PathBuf::from("/test/output.cba"),
                sesno_range: "100..=110".to_string(),
            },
            "Compression failed",
        )
        .with_priority(8);

        // 序列化
        let json = serde_json::to_string(&task).unwrap();

        // 反序列化
        let deserialized: FailedTask = serde_json::from_str(&json).unwrap();

        assert_eq!(task.id, deserialized.id);
        assert_eq!(task.priority, deserialized.priority);
        assert_eq!(task.error, deserialized.error);
    }
}
