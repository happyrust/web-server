# 失败任务类型参考

## 1. Core Summary

`FailedTaskType` 是失败任务队列系统的核心枚举,定义了4种可能的失败场景。每种类型包含恢复任务所需的完整上下文信息。

## 2. Source of Truth

**主代码文件**: `src/data_interface/failed_task_queue.rs:36-75`

**数据结构定义**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FailedTaskType {
    DatabaseQuery { dbnum: u32, operation: String },
    Compression { input_path: PathBuf, output_path: PathBuf, sesno_range: String },
    IncrementUpdate { path: PathBuf, sesno_range: String, dbnum: u32 },
    MqttPublish { topic: String, payload_summary: String },
}
```

**相关架构文档**: `/llmdoc/architecture/increment-error-recovery.md`

## 3. 类型详解

### DatabaseQuery - 数据库查询失败

**用途**: 记录数据库查询操作失败的任务

**字段**:
- `dbnum: u32` - 数据库编号 (如 1001, 7997)
- `operation: String` - 操作类型 (如 "query_sesno", "update_elements")

**使用场景**:
- SurrealDB连接超时
- SQL查询语法错误
- 数据库权限问题

**重试逻辑**: `src/data_interface/increment_manager.rs:1259-1268`
- 对于 "query_sesno" 操作,调用 `query_latest_sesno_by_dbnum(dbnum)`
- 成功后从队列移除

**示例**:
```rust
FailedTaskType::DatabaseQuery {
    dbnum: 1001,
    operation: "query_sesno".to_string(),
}
```

### Compression - CBA压缩失败

**用途**: 记录增量压缩包生成失败的任务

**字段**:
- `input_path: PathBuf` - 输入文件路径 (如 "D:/data/CATA.db")
- `output_path: PathBuf` - 输出文件路径 (如 "assets/archives/CATA.cba")
- `sesno_range: String` - 会话号范围 (如 "12341..=12350")

**使用场景**:
- 磁盘空间不足
- 临时IO错误
- 文件权限问题

**重试逻辑**: `src/data_interface/increment_manager.rs:1271-1289`
- 创建 `CompressOptions`
- 调用 `execute_compress()`
- 成功后从队列移除

**示例**:
```rust
FailedTaskType::Compression {
    input_path: PathBuf::from("D:/data/CATA.db"),
    output_path: PathBuf::from("assets/archives/CATA.cba"),
    sesno_range: "12341..=12350".to_string(),
}
```

### IncrementUpdate - 增量更新失败

**用途**: 记录增量数据更新失败的任务

**字段**:
- `path: PathBuf` - 数据库文件路径
- `sesno_range: String` - 会话号范围
- `dbnum: u32` - 数据库编号

**使用场景**:
- 增量数据解析失败
- SurrealDB批量插入失败
- 数据格式异常

**重试逻辑**: `src/data_interface/increment_manager.rs:1292-1304`
- ⚠️ **当前未完全实现**
- TODO: 需要重新解析 sesno_range 并调用 execute_incr_update

**示例**:
```rust
FailedTaskType::IncrementUpdate {
    path: PathBuf::from("D:/data/PIPE.db"),
    sesno_range: "100..=110".to_string(),
    dbnum: 2001,
}
```

### MqttPublish - MQTT推送失败

**用途**: 记录MQTT消息推送失败的任务

**字段**:
- `topic: String` - MQTT主题 (如 "Sync/E3d")
- `payload_summary: String` - 负载摘要 (避免存储完整消息体)

**使用场景**:
- MQTT broker连接失败
- 网络抖动
- 消息格式错误

**重试逻辑**: `src/data_interface/increment_manager.rs:1307-1318`
- ⚠️ **当前未完全实现**
- TODO: 需要重新构建消息并推送

**示例**:
```rust
FailedTaskType::MqttPublish {
    topic: "Sync/E3d".to_string(),
    payload_summary: "files=[CATA.cba], hash=abc123...".to_string(),
}
```

## 4. 序列化格式

使用 Serde 的 `tag = "type"` 属性,JSON序列化为tagged format:

```json
{
  "type": "DatabaseQuery",
  "dbnum": 1001,
  "operation": "query_sesno"
}
```

```json
{
  "type": "Compression",
  "input_path": "D:/data/CATA.db",
  "output_path": "assets/archives/CATA.cba",
  "sesno_range": "12341..=12350"
}
```

## 5. 扩展指南

### 添加新的失败类型

1. 在 `FailedTaskType` 枚举中添加新变体:
   ```rust
   SpatialIndexUpdate {
       dbnum: u32,
       element_count: usize,
   }
   ```

2. 在 `description()` 方法中添加对应分支:
   ```rust
   Self::SpatialIndexUpdate { dbnum, element_count } => {
       format!("空间索引更新失败 (dbnum={}, elements={})", dbnum, element_count)
   }
   ```

3. 在 `retry_failed_task()` 中实现重试逻辑:
   ```rust
   FailedTaskType::SpatialIndexUpdate { dbnum, element_count } => {
       // 重试空间索引更新
       rebuild_spatial_index(*dbnum).await?;
       Ok(())
   }
   ```

4. 在错误处理点创建新任务:
   ```rust
   let task = FailedTask::new(
       FailedTaskType::SpatialIndexUpdate {
           dbnum: 1001,
           element_count: 5000,
       },
       format!("空间索引更新失败: {:?}", error)
   );
   failed_queue.push(task).await;
   ```

## 6. 相关类型

### FailedTask

完整任务记录结构,包含:
- `task_type: FailedTaskType` - 任务类型
- `id: String` - UUID标识
- `error: String` - 错误信息
- `retry_count` / `max_retries` - 重试控制
- `metadata: Option<serde_json::Value>` - 额外元数据

**文件位置**: `src/data_interface/failed_task_queue.rs:100-139`

### FailedTaskQueue

队列管理器,提供:
- `push(task)` - 添加任务
- `remove(task_id)` - 移除任务
- `get_pending_tasks()` - 获取待重试任务
- `get_stats()` - 获取统计信息

**文件位置**: `src/data_interface/failed_task_queue.rs:212-396`

---

**文档版本**: 1.0
**最后更新**: 2025-11-20
**Rust版本要求**: nightly (需要 `#![feature(async_closure)]`)
