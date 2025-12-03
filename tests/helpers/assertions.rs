// 测试断言工具

use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::time::{sleep, Instant};

use aios_database::web_server::sync_control_center::{SyncControlCenter, SyncTaskStatus};

/// 断言任务已完成
pub async fn assert_task_completed(
    center: &SyncControlCenter,
    task_id: &str,
    timeout: Duration,
) {
    let start = Instant::now();

    loop {
        if start.elapsed() > timeout {
            panic!("任务 {} 在 {:?} 内未完成", task_id, timeout);
        }

        if center.history.iter().any(|t| {
            t.id == task_id && t.status == SyncTaskStatus::Completed
        }) {
            return;
        }

        sleep(Duration::from_millis(100)).await;
    }
}

/// 断言文件存在
pub async fn assert_file_exists(path: &Path) {
    assert!(
        path.exists(),
        "文件不存在: {}",
        path.display()
    );
}

/// 断言文件大小
pub async fn assert_file_size(path: &Path, expected_size: u64) {
    let metadata = fs::metadata(path)
        .await
        .expect(&format!("无法获取文件元数据: {}", path.display()));

    assert_eq!(
        metadata.len(),
        expected_size,
        "文件大小不匹配: 期望 {}, 实际 {}",
        expected_size,
        metadata.len()
    );
}

/// 断言 SQLite 日志存在
pub async fn assert_log_entry_exists(
    sqlite_path: &Path,
    task_id: &str,
    status: &str,
) {
    use rusqlite::Connection;

    let conn = Connection::open(sqlite_path)
        .expect(&format!("无法打开数据库: {}", sqlite_path.display()));

    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM remote_sync_logs WHERE task_id = ?1 AND status = ?2")
        .expect("SQL 准备失败");

    let count: i64 = stmt
        .query_row([task_id, status], |row| row.get(0))
        .expect("查询失败");

    assert!(
        count > 0,
        "未找到日志记录: task_id={}, status={}",
        task_id,
        status
    );
}

/// 断言队列大小
pub fn assert_queue_size(center: &SyncControlCenter, expected: usize) {
    assert_eq!(
        center.task_queue.len(),
        expected,
        "队列大小不匹配: 期望 {}, 实际 {}",
        expected,
        center.task_queue.len()
    );
}

/// 断言运行中任务数
pub fn assert_running_tasks(center: &SyncControlCenter, expected: usize) {
    assert_eq!(
        center.running_tasks.len(),
        expected,
        "运行中任务数不匹配: 期望 {}, 实际 {}",
        expected,
        center.running_tasks.len()
    );
}

/// 断言统计数据
pub fn assert_statistics(
    center: &SyncControlCenter,
    expected_synced: u64,
    expected_failed: u64,
) {
    assert_eq!(
        center.state.total_synced,
        expected_synced,
        "成功数不匹配: 期望 {}, 实际 {}",
        expected_synced,
        center.state.total_synced
    );

    assert_eq!(
        center.state.total_failed,
        expected_failed,
        "失败数不匹配: 期望 {}, 实际 {}",
        expected_failed,
        center.state.total_failed
    );
}
