// 测试辅助工具模块

pub mod test_environment;
pub mod assertions;
pub mod mock_data;

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::{sleep, Instant};

/// 等待条件满足（带超时）
pub async fn wait_for_condition<F, Fut>(
    mut condition: F,
    timeout: Duration,
) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = Instant::now();

    loop {
        if start.elapsed() > timeout {
            return false;
        }

        if condition().await {
            return true;
        }

        sleep(Duration::from_millis(100)).await;
    }
}

/// 创建临时测试目录
pub fn create_temp_dir(name: &str) -> PathBuf {
    let temp_base = std::env::temp_dir().join("aios_test");
    let temp_dir = temp_base.join(name);
    std::fs::create_dir_all(&temp_dir).unwrap();
    temp_dir
}

/// 清理临时测试目录
pub fn cleanup_temp_dir(path: &Path) {
    if path.exists() {
        std::fs::remove_dir_all(path).ok();
    }
}
