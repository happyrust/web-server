# 异地更新测试快速入门

## 📋 测试概述

异地更新系统需要测试的核心环节：

1. **增量检测** - 会话号对比和增量范围计算
2. **任务队列** - 优先级排序、并发控制、重试机制
3. **文件传输** - 本地复制和 HTTP 上传
4. **状态管理** - 任务状态流转、持久化
5. **容错处理** - 网络故障、重连、错误恢复

## 🚀 快速开始

### 运行已有的烟雾测试

```bash
# 运行现有的集成测试
cargo test --bin remote_sync_smoke_test --features web_server

# 查看详细输出
cargo test --bin remote_sync_smoke_test --features web_server -- --nocapture
```

### 运行单元测试

```bash
# 运行单元测试
cargo test --test unit --features web_server

# 运行特定测试
cargo test --test unit test_task_priority_ordering --features web_server
```

## 🧪 关键测试场景

### 场景 1：任务优先级队列

**测试什么**：验证高优先级任务先执行

```rust
#[test]
fn test_task_priority_ordering() {
    let mut center = SyncControlCenter::new();
    
    // 添加不同优先级：3, 8, 5
    center.add_task(priority: 3);
    center.add_task(priority: 8);
    center.add_task(priority: 5);
    
    // 验证顺序：8, 5, 3
    assert_eq!(queue[0].priority, 8);
    assert_eq!(queue[1].priority, 5);
    assert_eq!(queue[2].priority, 3);
}
```

### 场景 2：并发限制

**测试什么**：同时运行的任务不超过限制

```rust
#[test]
fn test_concurrent_limit() {
    center.config.max_concurrent_syncs = 2;
    
    // 添加 5 个任务
    for i in 0..5 { add_task(); }
    
    // 只能获取 2 个
    assert!(get_next_task().is_some()); // 1
    assert!(get_next_task().is_some()); // 2
    assert!(get_next_task().is_none());  // 被限制
}
```

### 场景 3：重试机制

**测试什么**：失败任务自动重试，达到上限后停止

```rust
#[test]
fn test_retry_mechanism() {
    center.config.max_retries = 3;
    
    let task = get_next_task();
    
    // 失败 3 次，每次重新入队
    complete_task(false); // retry_count = 1
    complete_task(false); // retry_count = 2
    complete_task(false); // retry_count = 3
    
    // 第 4 次失败，不再重试
    complete_task(false);
    assert!(queue.is_empty());
    assert_eq!(history.last().status, Failed);
}
```

### 场景 4：增量检测

**测试什么**：正确检测文件和数据库的会话号差异

```rust
#[test]
async fn test_increment_detection() {
    let file_sesno = 12345;
    let db_sesno = 12340;
    
    let result = detect_increment(file, db).await;
    
    assert!(result.has_increment);
    assert_eq!(result.range, 12341..=12345);
    assert_eq!(result.count, 5);
}
```

### 场景 5：文件传输

**测试什么**：文件正确上传到目标位置

```rust
#[tokio::test]
async fn test_http_upload() {
    // 启动文件接收器
    let receiver = start_file_receiver(port: 18080);
    
    // 执行上传
    let task = create_task("http://localhost:18080/test.cba");
    process_sync_task(&task).await.unwrap();
    
    // 验证文件已接收
    assert!(receiver.has_file("test.cba"));
    assert_eq!(receiver.file_size("test.cba"), 1024);
}
```

## 📊 测试策略

### 测试金字塔

```
     E2E (5%)        ← 真实环境，完整流程
    ───────
   Integration (25%) ← 模块协作，外部依赖
  ─────────────
 Unit Tests (70%)    ← 逻辑验证，快速反馈
───────────────
```

### 优先级

**P0 - 必须测试**
- ✅ 增量检测正确性
- ✅ 任务队列管理
- ✅ 并发控制
- ✅ 重试机制

**P1 - 重要测试**
- ⚠️ 文件传输完整性
- ⚠️ 网络故障恢复
- ⚠️ 数据持久化

**P2 - 可选测试**
- 📝 性能基准
- 📝 压力测试
- 📝 长时间运行

## 🛠️ 测试工具

### 已实现的工具

1. **`remote_sync_smoke_test.rs`**
   - 完整的端到端测试
   - 临时环境、HTTP 接收器、SQLite 日志
   - 验证完整流程

2. **测试辅助函数**
   - `wait_for_condition()` - 等待条件满足
   - `assert_task_completed()` - 断言任务完成
   - `assert_file_exists()` - 断言文件存在

### 需要实现的工具

```rust
// Mock PDMS 文件生成器
pub struct PdmsFileGenerator {
    pub fn with_sesno(sesno: i32) -> Self;
    pub fn generate(path: &Path) -> Result<()>;
}

// Mock MQTT 客户端
pub struct MockMqttClient {
    pub fn publish(topic, payload) -> Result<()>;
    pub fn get_messages() -> Vec<Message>;
}

// 测试数据库构建器
pub struct TestDatabaseBuilder {
    pub fn add_env(env: RemoteSyncEnv) -> Self;
    pub fn add_site(site: RemoteSyncSite) -> Self;
    pub fn build() -> TestDatabase;
}
```

## 🔧 调试技巧

### 1. 打印详细日志

```bash
# 启用 debug 日志
RUST_LOG=debug cargo test --features web_server -- --nocapture
```

### 2. 保留测试数据

```rust
// 不自动清理，方便检查
let temp_dir = create_temp_dir("test_data");
// ... 测试代码 ...
// temp_dir.cleanup(); // 注释掉清理代码
println!("测试数据保存在: {:?}", temp_dir.path());
```

### 3. 单独运行测试

```bash
# 只运行一个测试
cargo test test_task_priority_ordering --features web_server -- --exact
```

### 4. 检查 SQLite 日志

```bash
# 查看测试生成的日志
sqlite3 deployment_sites.sqlite "SELECT * FROM remote_sync_logs ORDER BY created_at DESC LIMIT 10;"
```

## 🎯 测试检查清单

在提交代码前，确保：

- [ ] 所有单元测试通过
- [ ] 烟雾测试通过
- [ ] 覆盖了新增代码的关键路径
- [ ] 添加了失败场景测试
- [ ] 测试代码有清晰的注释
- [ ] 清理了临时文件和资源

## 📈 下一步

1. **实施单元测试** - 按照 `tests/unit/sync_control_center_test.rs` 模板
2. **添加集成测试** - 测试 MQTT、HTTP、数据库交互
3. **完善测试工具** - 实现 Mock 对象和辅助函数
4. **CI/CD 集成** - 自动运行测试

## 📚 参考文档

- [完整测试策略](./REMOTE_SYNC_TEST_STRATEGY.md)
- [开发指南](./REMOTE_SYNC_DEVELOPMENT_GUIDE.md)
- [实现检查报告](./REMOTE_SYNC_IMPLEMENTATION_CHECK.md)
