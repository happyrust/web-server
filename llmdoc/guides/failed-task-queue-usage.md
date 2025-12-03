# 如何使用失败任务队列

本指南说明如何在增量更新系统中使用错误恢复机制。

## 1. 添加失败任务到队列

当数据库操作或文件处理失败时,创建失败任务并推入队列:

```rust
use crate::data_interface::failed_task_queue::{FailedTask, FailedTaskType};

// 示例: 数据库查询失败
let task = FailedTask::new(
    FailedTaskType::DatabaseQuery {
        dbnum: 1001,
        operation: "query_sesno".to_string(),
    },
    format!("数据库查询失败: {:?}", error)
).with_metadata(serde_json::json!({
    "file_path": path.to_string_lossy(),
    "old_sesno": old_sesno,
    "new_sesno": new_sesno,
}));

self.failed_queue.push(task).await;
```

**关键步骤**:
1. 导入 `FailedTask` 和 `FailedTaskType`
2. 使用 `FailedTask::new()` 创建任务
3. 使用 `with_metadata()` 添加上下文信息 (可选但推荐)
4. 调用 `failed_queue.push(task).await` 推入队列

## 2. 处理CBA压缩失败

CBA压缩失败需要记录输入输出路径和会话号范围:

```rust
let task = FailedTask::new(
    FailedTaskType::Compression {
        input_path: path.clone(),
        output_path: output.clone(),
        sesno_range: format!("{}..={}", start_sesno, end_sesno),
    },
    format!("CBA压缩失败: {:?}", error)
).with_metadata(serde_json::json!({
    "file_name": file_name,
    "db_num": dbno,
    "is_new_file": true,
}));

self.failed_queue.push(task).await;
```

**重要**: `sesno_range` 字符串格式必须为 `"start..=end"`,以便重试时解析。

## 3. 监控失败任务队列

### 查看队列统计

```rust
let stats = failed_queue.get_stats().await;
println!("总任务数: {}", stats.total);
println!("待重试: {}", stats.pending);
println!("等待中: {}", stats.waiting);
println!("已耗尽: {}", stats.exhausted);
```

### 查看所有待重试任务

```rust
let pending = failed_queue.get_pending_tasks().await;
for task in pending {
    println!("待重试: {} (次数: {}/{})",
        task.description(),
        task.retry_count,
        task.max_retries
    );
}
```

### 查看已耗尽任务 (需要人工介入)

```rust
let exhausted = failed_queue.get_exhausted_tasks().await;
if !exhausted.is_empty() {
    eprintln!("⚠️ 有 {} 个任务已达到最大重试次数:", exhausted.len());
    for task in exhausted {
        eprintln!("  - [{}] {}: {}",
            task.id,
            task.description(),
            task.error
        );
    }
}
```

## 4. 自定义任务参数

### 设置优先级 (1-10, 10最高)

```rust
let task = FailedTask::new(...)
    .with_priority(8);  // 高优先级
```

### 设置最大重试次数

```rust
let task = FailedTask::new(...)
    .with_max_retries(3);  // 最多重试3次
```

### 添加错误堆栈

```rust
let task = FailedTask::new(...)
    .with_trace(format!("{:?}", backtrace));
```

## 5. 手动清理已耗尽任务

**警告**: 清理前应先检查并处理这些任务,避免数据丢失!

```rust
// 1. 先查看已耗尽任务
let exhausted = failed_queue.get_exhausted_tasks().await;
for task in &exhausted {
    // 检查任务元数据,确认数据已恢复或可以丢弃
    println!("准备清理: {:?}", task.metadata);
}

// 2. 确认后清理
let removed_count = failed_queue.cleanup_exhausted().await;
println!("🧹 清理了 {} 个已耗尽的任务", removed_count);
```

## 6. 查看持久化文件

失败任务持久化到 `assets/failed_tasks.json`,可直接查看:

```bash
# Windows
type assets\failed_tasks.json

# Linux/macOS
cat assets/failed_tasks.json
```

JSON格式示例:

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "task_type": {
      "type": "DatabaseQuery",
      "dbnum": 1001,
      "operation": "query_sesno"
    },
    "error": "数据库查询失败: Connection timeout",
    "retry_count": 2,
    "max_retries": 5,
    "first_failed_at": 1732089600,
    "last_retry_at": 1732089720,
    "next_retry_at": 1732089840,
    "priority": 5,
    "metadata": {
      "file_path": "D:/data/CATA.db",
      "old_sesno": 12340,
      "new_sesno": 12350
    }
  }
]
```

## 7. 验证重试机制是否正常工作

### 检查retry_worker日志

系统启动后应看到:

```
🚀 失败任务重试worker已启动
```

每60秒一次的扫描日志:

```
📋 发现 2 个待重试任务
🔄 开始重试任务: DB查询失败 (dbnum=1001, op=query_sesno) (重试次数: 1/5)
✅ 任务重试成功，已从队列移除: 550e8400-...
```

### 检查统计日志

```
📊 失败任务队列统计: 总数=3, 待重试=1, 等待中=2, 已耗尽=0
```

## 8. 常见问题排查

### Q: 任务一直重试失败怎么办?

A: 检查任务的 `error` 字段和 `metadata`,确认根本原因:
- 数据库连接问题: 检查网络和数据库状态
- 磁盘空间不足: 清理磁盘或扩容
- 文件权限问题: 检查文件可读写权限

### Q: 如何强制立即重试某个任务?

A: 当前不支持手动触发,但可以:
1. 从JSON文件中找到任务的 `next_retry_at` 时间戳
2. 修改为当前时间戳
3. 等待下一个60秒扫描周期

### Q: 如何调整重试间隔?

A: 当前重试间隔固定为指数退避(1/2/4/8/16分钟),不支持运行时调整。如需修改,编辑 `src/data_interface/failed_task_queue.rs:190`:

```rust
// 修改前: 60 * (2^retry_count)
let delay_secs = 60 * (2u64.pow(self.retry_count.min(4)));

// 修改后 (如改为30秒起步):
let delay_secs = 30 * (2u64.pow(self.retry_count.min(4)));
```

### Q: 断电后任务是否会丢失?

A: 不会。所有任务持久化到 `assets/failed_tasks.json`,系统重启时自动加载。

---

**文档版本**: 1.0
**最后更新**: 2025-11-20
**相关架构文档**: [增量更新错误恢复架构](../architecture/increment-error-recovery.md)
