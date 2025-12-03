//! SesnoCache - 会话号查询缓存
//!
//! 用于缓存数据库会话号查询结果，减少数据库压力
//! TTL: 5秒

use dashmap::DashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 会话号
    sesno: u32,
    /// 缓存时间
    cached_at: Instant,
}

impl CacheEntry {
    fn new(sesno: u32) -> Self {
        Self {
            sesno,
            cached_at: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.cached_at.elapsed() > ttl
    }
}

/// 会话号缓存管理器
#[derive(Clone)]
pub struct SesnoCache {
    /// 缓存存储 (dbnum -> (sesno, timestamp))
    cache: Arc<DashMap<u32, CacheEntry>>,
    /// 缓存过期时间（默认5秒）
    ttl: Duration,
}

impl SesnoCache {
    /// 创建新的缓存实例
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            ttl,
        }
    }

    /// 从缓存获取会话号，如果不存在或已过期则执行查询函数
    ///
    /// # Arguments
    /// * `dbnum` - 数据库编号
    /// * `query_fn` - 查询函数（当缓存未命中时执行）
    ///
    /// # Returns
    /// * `Ok(sesno)` - 会话号
    /// * `Err(e)` - 查询失败错误
    pub async fn get_or_query<F, Fut>(&self, dbnum: u32, query_fn: F) -> anyhow::Result<u32>
    where
        F: FnOnce(u32) -> Fut,
        Fut: Future<Output = anyhow::Result<u32>>,
    {
        // 1. 尝试从缓存获取
        if let Some(entry) = self.cache.get(&dbnum) {
            if !entry.is_expired(self.ttl) {
                // 缓存命中且未过期
                let sesno = entry.sesno;
                drop(entry); // 释放读锁
                return Ok(sesno);
            }
            // 缓存已过期，需要删除
            drop(entry);
            self.cache.remove(&dbnum);
        }

        // 2. 缓存未命中，执行查询
        let sesno = query_fn(dbnum).await?;

        // 3. 更新缓存
        self.cache.insert(dbnum, CacheEntry::new(sesno));

        Ok(sesno)
    }

    /// 使指定dbnum的缓存失效
    pub fn invalidate(&self, dbnum: u32) {
        self.cache.remove(&dbnum);
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.cache.len(),
            ttl_seconds: self.ttl.as_secs(),
        }
    }

    /// 清理所有过期的缓存条目
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let ttl = self.ttl;

        self.cache
            .retain(|_, entry| now.duration_since(entry.cached_at) <= ttl);
    }

    /// 启动后台清理任务（每60秒清理一次过期条目）
    pub fn start_cleanup_worker(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;

                let before = self.cache.len();
                self.cleanup_expired();
                let after = self.cache.len();

                if before != after {
                    println!(
                        "🧹 SesnoCache清理完成: 清理了 {} 个过期条目，剩余 {} 个",
                        before - after,
                        after
                    );
                }
            }
        });
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 总条目数
    pub total_entries: usize,
    /// TTL（秒）
    pub ttl_seconds: u64,
}

impl Default for SesnoCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sesno_cache_basic() {
        let cache = SesnoCache::new(Duration::from_secs(5));
        let mut query_count = 0;

        // 第一次查询（缓存未命中）
        let result1 = cache
            .get_or_query(1001, |_dbnum| async {
                query_count += 1;
                Ok(12345)
            })
            .await;
        assert_eq!(result1.unwrap(), 12345);
        assert_eq!(query_count, 1); // 执行了查询

        // 第二次查询（缓存命中）
        let result2 = cache
            .get_or_query(1001, |_dbnum| async {
                query_count += 1;
                Ok(99999)
            })
            .await;
        assert_eq!(result2.unwrap(), 12345); // 返回缓存值，不是99999
        assert_eq!(query_count, 1); // 没有执行新查询
    }

    #[tokio::test]
    async fn test_sesno_cache_expiration() {
        let cache = SesnoCache::new(Duration::from_millis(100)); // 100ms过期
        let mut query_count = 0;

        // 第一次查询
        cache
            .get_or_query(1001, |_dbnum| async {
                query_count += 1;
                Ok(12345)
            })
            .await
            .unwrap();
        assert_eq!(query_count, 1);

        // 等待过期
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 第二次查询（缓存已过期）
        cache
            .get_or_query(1001, |_dbnum| async {
                query_count += 1;
                Ok(67890)
            })
            .await
            .unwrap();
        assert_eq!(query_count, 2); // 重新执行了查询
    }

    #[tokio::test]
    async fn test_sesno_cache_invalidate() {
        let cache = SesnoCache::new(Duration::from_secs(60));
        let mut query_count = 0;

        // 插入缓存
        cache
            .get_or_query(1001, |_dbnum| async {
                query_count += 1;
                Ok(12345)
            })
            .await
            .unwrap();

        // 主动失效
        cache.invalidate(1001);

        // 再次查询（缓存已失效）
        cache
            .get_or_query(1001, |_dbnum| async {
                query_count += 1;
                Ok(67890)
            })
            .await
            .unwrap();

        assert_eq!(query_count, 2); // 重新执行了查询
    }

    #[tokio::test]
    async fn test_sesno_cache_concurrent() {
        let cache = Arc::new(SesnoCache::new(Duration::from_secs(5)));
        let mut tasks = vec![];

        // 10个并发查询相同的dbnum
        for _ in 0..10 {
            let cache = cache.clone();
            let task = tokio::spawn(async move {
                cache
                    .get_or_query(1001, |_dbnum| async {
                        // 模拟数据库查询延迟
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Ok(12345)
                    })
                    .await
            });
            tasks.push(task);
        }

        // 等待所有任务完成
        let results: Vec<_> = futures::future::join_all(tasks).await;

        // 验证所有结果一致
        for result in results {
            assert_eq!(result.unwrap().unwrap(), 12345);
        }

        // 缓存中应该只有一个条目
        assert_eq!(cache.cache.len(), 1);
    }

    #[test]
    fn test_cache_stats() {
        let cache = SesnoCache::new(Duration::from_secs(10));
        let stats = cache.stats();

        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.ttl_seconds, 10);
    }
}
