# 异地更新系统测试策略与方案

**文档版本**: 1.0.0  
**创建日期**: 2025-01-18  
**适用系统**: 异地更新（Remote Sync）增量检测与同步系统

---

## 目录

1. [测试挑战分析](#测试挑战分析)
2. [测试策略](#测试策略)
3. [测试环境搭建](#测试环境搭建)
4. [单元测试方案](#单元测试方案)
5. [集成测试方案](#集成测试方案)
6. [端到端测试方案](#端到端测试方案)
7. [性能测试方案](#性能测试方案)
8. [故障注入测试](#故障注入测试)
9. [测试工具和框架](#测试工具和框架)
10. [测试数据准备](#测试数据准备)
11. [自动化测试流程](#自动化测试流程)
12. [测试覆盖率要求](#测试覆盖率要求)

---

## 测试挑战分析

### 1.1 系统特点

异地更新系统是一个**复杂的异步分布式系统**，具有以下特点：

```
┌─────────────────────────────────────────────────────────────┐
│                    测试复杂度分析                              │
└─────────────────────────────────────────────────────────────┘

1. 文件系统交互
   - notify 文件监听
   - PDMS 二进制文件读取
   - 跨平台文件操作

2. 数据库交互
   - SurrealDB 增量更新
   - SQLite 配置和日志
   - 会话号查询和对比

3. 网络交互
   - MQTT 消息发布/订阅
   - HTTP PUT 文件上传
   - 断线重连机制

4. 并发控制
   - 多任务队列
   - Worker 线程池
   - 文件监听事件流

5. 状态管理
   - 全局共享状态 (Arc<RwLock>)
   - 任务状态流转
   - 重试计数和历史记录

6. 时序依赖
   - 文件修改 → 增量检测 → 压缩 → 入队 → 执行
   - MQTT 通知的顺序性
   - 并发任务的执行顺序
```

### 1.2 核心测试难点

| 难点 | 描述 | 影响 |
|------|------|------|
| **文件格式专有** | PDMS 二进制文件格式复杂 | 难以生成测试数据 |
| **增量检测逻辑** | 基于会话号对比，涉及多表查询 | 需要精确控制数据库状态 |
| **异步事件流** | notify 文件监听是异步的 | 测试时序难以控制 |
| **MQTT 外部依赖** | 需要运行 MQTT Broker | 集成测试复杂度高 |
| **并发竞态** | 多个任务同时执行 | 难以重现问题 |
| **网络不可靠性** | HTTP 上传可能失败 | 需要模拟各种网络条件 |
| **状态持久化** | SQLite 日志和缓存 | 测试隔离困难 |

### 1.3 测试目标

```
┌─────────────────────────────────────────────────────────────┐
│                        测试金字塔                              │
└─────────────────────────────────────────────────────────────┘

                    ▲
                   ╱ ╲
                  ╱E2E╲              1. 端到端测试 (5%)
                 ╱─────╲                - 真实环境
                ╱       ╲               - 完整流程
               ╱─────────╲
              ╱ Integration╲         2. 集成测试 (25%)
             ╱─────────────╲            - 模块协作
            ╱               ╲           - 外部依赖
           ╱─────────────────╲
          ╱                   ╲      3. 单元测试 (70%)
         ╱    Unit Tests       ╲        - 逻辑验证
        ╱───────────────────────╲       - 快速反馈
       ╱                         ╲      - 高覆盖率
      ╱───────────────────────────╲
     ───────────────────────────────
```

**目标指标**：
- **单元测试覆盖率**: ≥ 70%
- **集成测试场景**: ≥ 20 个
- **端到端测试**: ≥ 5 个关键流程
- **性能基准**: 100 任务/分钟
- **可靠性**: 失败重试成功率 ≥ 95%

---

## 测试策略

### 2.1 分层测试策略

```
┌──────────────────────────────────────────────────────────────┐
│ 测试层级              │ 测试对象            │ 隔离方式           │
├──────────────────────────────────────────────────────────────┤
│ 1. 单元测试            │ 单个函数/方法        │ Mock 所有外部依赖   │
│ 2. 组件测试            │ 单个模块            │ Mock 外部模块      │
│ 3. 集成测试            │ 多个模块协作         │ 真实依赖 + Stub    │
│ 4. 端到端测试          │ 完整业务流程         │ 真实环境           │
│ 5. 性能测试            │ 系统吞吐量           │ 生产级负载         │
│ 6. 故障注入测试         │ 异常处理能力         │ 模拟故障场景       │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 测试优先级

#### P0 - 关键路径（必须100%通过）
1. 增量检测正确性
2. 文件传输完整性
3. 任务状态流转正确性
4. 并发控制正确性

#### P1 - 核心功能（必须95%通过）
5. 重试机制
6. MQTT 消息发布
7. 元数据更新
8. 错误处理

#### P2 - 边界情况（必须80%通过）
9. 空队列处理
10. 大文件处理
11. 网络超时
12. 断线重连

### 2.3 测试数据策略

```
┌─────────────────────────────────────────────────────────────┐
│ 测试数据类型          │ 生成方式             │ 用途              │
├─────────────────────────────────────────────────────────────┤
│ 1. 真实 PDMS 文件     │ 从生产环境采样        │ 真实性验证         │
│ 2. 模拟 PDMS 文件     │ 工具生成             │ 边界测试           │
│ 3. 假数据            │ 随机生成 .cba        │ 传输测试           │
│ 4. SQLite Fixtures   │ SQL 脚本初始化       │ 配置测试           │
│ 5. Mock 数据         │ 代码中硬编码         │ 单元测试           │
└─────────────────────────────────────────────────────────────┘
```

---

## 测试环境搭建

### 3.1 本地测试环境

```yaml
# docker-compose.test.yml
version: '3.8'

services:
  # SurrealDB 测试实例
  surrealdb-test:
    image: surrealdb/surrealdb:latest
    ports:
      - "18000:8000"
    command: start --user root --pass root memory
    
  # MQTT Broker 测试实例
  mosquitto-test:
    image: eclipse-mosquitto:2
    ports:
      - "11883:1883"
    volumes:
      - ./tests/mosquitto-test.conf:/mosquitto/config/mosquitto.conf
    
  # HTTP 文件接收器（测试用）
  file-receiver:
    build:
      context: ./tests/file-receiver
    ports:
      - "18080:8080"
    volumes:
      - ./test-output:/app/files
```

**启动命令**：
```bash
# 启动测试环境
docker-compose -f docker-compose.test.yml up -d

# 验证服务
curl http://localhost:18000/health  # SurrealDB
curl http://localhost:18080/health  # 文件接收器

# 清理环境
docker-compose -f docker-compose.test.yml down -v
```

### 3.2 测试目录结构

```
tests/
├── unit/                           # 单元测试
│   ├── sync_control_center_test.rs
│   ├── increment_detector_test.rs
│   ├── task_executor_test.rs
│   └── destination_resolver_test.rs
├── integration/                    # 集成测试
│   ├── full_workflow_test.rs
│   ├── mqtt_integration_test.rs
│   ├── http_upload_test.rs
│   └── database_persistence_test.rs
├── e2e/                           # 端到端测试
│   ├── real_file_sync_test.rs
│   └── multi_site_sync_test.rs
├── performance/                    # 性能测试
│   ├── throughput_test.rs
│   └── concurrent_tasks_test.rs
├── fixtures/                       # 测试数据
│   ├── sample_pdms/
│   ├── sqlite/
│   └── mock_data/
├── helpers/                        # 测试工具
│   ├── mock_file_watcher.rs
│   ├── mock_mqtt_client.rs
│   ├── test_db_builder.rs
│   └── assertions.rs
└── docker/                         # 测试容器配置
    ├── file-receiver/
    └── mosquitto-test.conf
```

### 3.3 测试配置

```toml
# tests/test_config.toml

[test_environment]
surrealdb_url = "http://localhost:18000"
mqtt_broker = "localhost:11883"
file_receiver = "http://localhost:18080"
sqlite_path = ":memory:"          # 内存数据库
temp_dir = "target/test-temp"

[test_timeouts]
default_timeout_ms = 5000
file_watch_timeout_ms = 2000
http_timeout_ms = 3000
mqtt_publish_timeout_ms = 1000

[test_concurrency]
max_concurrent_tests = 4
worker_threads = 2
```

---

## 单元测试方案

### 4.1 增量检测逻辑测试

**测试目标**: 验证会话号对比和增量范围计算的正确性

```rust
// tests/unit/increment_detector_test.rs

use aios_database::data_interface::increment_manager::AiosDBManager;
use std::path::PathBuf;

#[tokio::test]
async fn test_detect_increment_when_file_sesno_greater() {
    // Arrange: 准备测试数据
    let file_sesno = 12345;
    let db_sesno = 12340;
    
    // Mock 数据库查询
    let mock_db = MockSurrealDB::new()
        .expect_query_sesno("CATA", db_sesno);
    
    // Mock 文件读取
    let mock_file = MockPdmsFile::new()
        .with_sesno(file_sesno)
        .build("test_data/CATA.db");
    
    // Act: 执行增量检测
    let result = detect_increment(&mock_file, &mock_db).await.unwrap();
    
    // Assert: 验证结果
    assert!(result.has_increment);
    assert_eq!(result.range, 12341..=12345);
    assert_eq!(result.count, 5);
}

#[tokio::test]
async fn test_no_increment_when_sesno_equal() {
    let file_sesno = 12345;
    let db_sesno = 12345;
    
    let mock_db = MockSurrealDB::new()
        .expect_query_sesno("CATA", db_sesno);
    let mock_file = MockPdmsFile::new()
        .with_sesno(file_sesno)
        .build("test_data/CATA.db");
    
    let result = detect_increment(&mock_file, &mock_db).await.unwrap();
    
    assert!(!result.has_increment);
    assert_eq!(result.count, 0);
}

#[tokio::test]
async fn test_no_increment_when_file_sesno_smaller() {
    // 文件回滚的情况（不应该发生，但要测试）
    let file_sesno = 12340;
    let db_sesno = 12345;
    
    let mock_db = MockSurrealDB::new()
        .expect_query_sesno("CATA", db_sesno);
    let mock_file = MockPdmsFile::new()
        .with_sesno(file_sesno)
        .build("test_data/CATA.db");
    
    let result = detect_increment(&mock_file, &mock_db).await.unwrap();
    
    assert!(!result.has_increment);
}

#[tokio::test]
async fn test_large_increment_range() {
    // 测试大量增量（如从 0 到 10000）
    let file_sesno = 10000;
    let db_sesno = 0;
    
    let mock_db = MockSurrealDB::new()
        .expect_query_sesno("CATA", db_sesno);
    let mock_file = MockPdmsFile::new()
        .with_sesno(file_sesno)
        .build("test_data/CATA.db");
    
    let result = detect_increment(&mock_file, &mock_db).await.unwrap();
    
    assert!(result.has_increment);
    assert_eq!(result.count, 10000);
}
```

**关键测试点**：
1. ✅ 会话号对比逻辑
2. ✅ 增量范围计算
3. ✅ 边界条件（相等、小于、零值）
4. ✅ 大范围增量

### 4.2 任务队列管理测试

```rust
// tests/unit/sync_control_center_test.rs

use aios_database::web_server::sync_control_center::*;

#[test]
fn test_task_priority_ordering() {
    let mut center = SyncControlCenter::new();
    
    // 添加不同优先级的任务
    let task1_id = center.add_task(NewSyncTaskParams {
        file_path: "file1.cba".into(),
        file_size: 1024,
        priority: 3,
        ..Default::default()
    });
    
    let task2_id = center.add_task(NewSyncTaskParams {
        file_path: "file2.cba".into(),
        file_size: 1024,
        priority: 8,
        ..Default::default()
    });
    
    let task3_id = center.add_task(NewSyncTaskParams {
        file_path: "file3.cba".into(),
        file_size: 1024,
        priority: 5,
        ..Default::default()
    });
    
    // 验证优先级排序
    assert_eq!(center.task_queue.len(), 3);
    assert_eq!(center.task_queue[0].id, task2_id); // priority 8
    assert_eq!(center.task_queue[1].id, task3_id); // priority 5
    assert_eq!(center.task_queue[2].id, task1_id); // priority 3
}

#[test]
fn test_concurrent_limit_enforcement() {
    let mut center = SyncControlCenter::new();
    center.config.max_concurrent_syncs = 2;
    
    // 添加 5 个任务
    for i in 0..5 {
        center.add_task(NewSyncTaskParams {
            file_path: format!("file{}.cba", i),
            file_size: 1024,
            priority: 5,
            ..Default::default()
        });
    }
    
    // 获取任务直到达到并发限制
    let task1 = center.get_next_task();
    let task2 = center.get_next_task();
    let task3 = center.get_next_task(); // 应该返回 None
    
    assert!(task1.is_some());
    assert!(task2.is_some());
    assert!(task3.is_none());
    assert_eq!(center.running_tasks.len(), 2);
}

#[test]
fn test_task_completion_success() {
    let mut center = SyncControlCenter::new();
    
    let task_id = center.add_task(NewSyncTaskParams {
        file_path: "test.cba".into(),
        file_size: 1024,
        priority: 5,
        ..Default::default()
    });
    
    let task = center.get_next_task().unwrap();
    
    // 完成任务
    center.complete_task(&task.id, true, None);
    
    // 验证状态
    assert_eq!(center.state.total_synced, 1);
    assert_eq!(center.state.total_failed, 0);
    assert_eq!(center.running_tasks.len(), 0);
    assert_eq!(center.history.len(), 1);
    assert_eq!(center.history[0].status, SyncTaskStatus::Completed);
}

#[test]
fn test_task_retry_mechanism() {
    let mut center = SyncControlCenter::new();
    center.config.auto_retry = true;
    center.config.max_retries = 3;
    
    let task_id = center.add_task(NewSyncTaskParams {
        file_path: "test.cba".into(),
        file_size: 1024,
        priority: 5,
        ..Default::default()
    });
    
    let task = center.get_next_task().unwrap();
    
    // 第一次失败
    center.complete_task(&task.id, false, Some("网络错误".into()));
    
    // 验证重新入队
    assert_eq!(center.state.total_failed, 1);
    assert_eq!(center.task_queue.len(), 1);
    assert_eq!(center.task_queue[0].retry_count, 1);
    assert_eq!(center.task_queue[0].status, SyncTaskStatus::Pending);
    
    // 第二次失败
    let task2 = center.get_next_task().unwrap();
    center.complete_task(&task2.id, false, Some("网络错误".into()));
    assert_eq!(center.task_queue[0].retry_count, 2);
    
    // 第三次失败
    let task3 = center.get_next_task().unwrap();
    center.complete_task(&task3.id, false, Some("网络错误".into()));
    assert_eq!(center.task_queue[0].retry_count, 3);
    
    // 第四次失败，不再重试
    let task4 = center.get_next_task().unwrap();
    center.complete_task(&task4.id, false, Some("网络错误".into()));
    
    assert_eq!(center.task_queue.len(), 0); // 不再入队
    assert_eq!(center.history.last().unwrap().status, SyncTaskStatus::Failed);
}

#[test]
fn test_cancel_pending_task() {
    let mut center = SyncControlCenter::new();
    
    let task_id = center.add_task(NewSyncTaskParams {
        file_path: "test.cba".into(),
        file_size: 1024,
        priority: 5,
        ..Default::default()
    });
    
    // 取消任务
    let cancelled = center.cancel_pending_task(&task_id, "用户取消");
    
    assert!(cancelled);
    assert_eq!(center.task_queue.len(), 0);
    assert_eq!(center.history.len(), 1);
    assert_eq!(center.history[0].status, SyncTaskStatus::Cancelled);
}

#[test]
fn test_clear_queue() {
    let mut center = SyncControlCenter::new();
    
    // 添加多个任务
    for i in 0..10 {
        center.add_task(NewSyncTaskParams {
            file_path: format!("file{}.cba", i),
            file_size: 1024,
            priority: 5,
            ..Default::default()
        });
    }
    
    // 清空队列
    let removed_count = center.clear_queue("测试清理");
    
    assert_eq!(removed_count, 10);
    assert_eq!(center.task_queue.len(), 0);
    assert_eq!(center.history.len(), 10);
}
```

### 4.3 目标解析测试

```rust
// tests/unit/destination_resolver_test.rs

use aios_database::web_server::sync_control_center::*;

#[tokio::test]
async fn test_resolve_http_destination() {
    // Mock SQLite 返回 HTTP URL
    let mock_sqlite = MockSQLite::new()
        .expect_query_site(SiteConfig {
            id: "site-001",
            name: "test-site",
            http_host: Some("http://192.168.1.100:8080/files"),
            ..Default::default()
        });
    
    let task = SyncTask {
        target_site: Some("site-001".into()),
        ..mock_task()
    };
    
    let destination = resolve_sync_destination(task).await.unwrap();
    
    match destination.target {
        ResolvedTarget::Http { url } => {
            assert!(url.starts_with("http://192.168.1.100:8080/files"));
            assert!(url.contains("test.cba"));
        }
        _ => panic!("Expected HTTP target"),
    }
}

#[tokio::test]
async fn test_resolve_local_destination() {
    let mock_sqlite = MockSQLite::new()
        .expect_query_site(SiteConfig {
            id: "site-002",
            name: "local-site",
            http_host: Some("local:/data/sync"),
            ..Default::default()
        });
    
    let task = SyncTask {
        target_site: Some("site-002".into()),
        ..mock_task()
    };
    
    let destination = resolve_sync_destination(task).await.unwrap();
    
    match destination.target {
        ResolvedTarget::Local { final_path } => {
            assert!(final_path.to_str().unwrap().contains("sync"));
        }
        _ => panic!("Expected Local target"),
    }
}

#[tokio::test]
async fn test_path_sanitization() {
    // 测试路径清洗（防止路径遍历）
    let malicious_name = "../../../etc/passwd";
    let sanitized = sanitize_path_segment(malicious_name);
    
    assert!(!sanitized.contains(".."));
    assert!(!sanitized.contains("/"));
}
```

---

## 集成测试方案

### 5.1 完整工作流测试

```rust
// tests/integration/full_workflow_test.rs

use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_file_modification_to_sync_completion() {
    // 1. 启动测试环境
    let test_env = TestEnvironment::new().await;
    test_env.start_services().await; // SurrealDB, MQTT, File Receiver
    
    // 2. 初始化同步控制中心
    let mut center = SyncControlCenter::new();
    center.start("test-env".into()).await.unwrap();
    
    // 3. 模拟文件修改
    let test_file = test_env.create_test_pdms_file("CATA.db", 12345);
    
    // 4. 等待文件监听触发
    sleep(Duration::from_secs(2)).await;
    
    // 5. 验证增量检测
    let db_sesno = query_sesno_from_db("CATA").await.unwrap();
    assert_eq!(db_sesno, 12345);
    
    // 6. 验证任务入队
    let state = center.get_state_snapshot();
    assert_eq!(state.queue_size, 1);
    
    // 7. 等待任务完成
    wait_for_condition(|| async {
        let state = center.get_state_snapshot();
        state.total_synced > 0
    }, Duration::from_secs(10)).await;
    
    // 8. 验证文件已传输
    let remote_file = test_env.get_received_file("CATA.cba").await;
    assert!(remote_file.exists());
    
    // 9. 验证元数据更新
    let metadata = test_env.read_metadata_json().await.unwrap();
    assert!(metadata.entries.iter().any(|e| e.file_name == "CATA.cba"));
    
    // 10. 验证 SQLite 日志
    let logs = test_env.query_sync_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].status, "completed");
    
    // 清理
    test_env.cleanup().await;
}
```

### 5.2 MQTT 集成测试

```rust
// tests/integration/mqtt_integration_test.rs

#[tokio::test]
async fn test_mqtt_publish_on_increment() {
    // 启动 MQTT Broker 和订阅者
    let broker = start_test_mqtt_broker().await;
    let subscriber = MqttSubscriber::new("localhost:11883")
        .subscribe("Sync/E3d")
        .await;
    
    // 触发增量更新
    let mgr = AiosDBManager::init_from_config().await.unwrap();
    trigger_increment_update(&mgr, "CATA.db", 12340, 12345).await;
    
    // 等待 MQTT 消息
    let message = subscriber.wait_for_message(Duration::from_secs(5)).await.unwrap();
    
    // 验证消息内容
    let payload: SyncE3dFileMsg = serde_json::from_slice(&message.payload).unwrap();
    assert!(payload.file_names.contains(&"CATA".to_string()));
    assert!(payload.file_hashes.len() > 0);
    
    broker.stop().await;
}

#[tokio::test]
async fn test_mqtt_reconnect_on_disconnect() {
    let broker = start_test_mqtt_broker().await;
    
    let mgr = AiosDBManager::init_from_config().await.unwrap();
    let mqtt_client = mgr.mqtt_client.clone();
    
    // 验证初始连接
    assert!(is_mqtt_connected(&mqtt_client).await);
    
    // 停止 Broker 模拟断线
    broker.stop().await;
    sleep(Duration::from_secs(1)).await;
    assert!(!is_mqtt_connected(&mqtt_client).await);
    
    // 重启 Broker
    let broker = start_test_mqtt_broker().await;
    
    // 等待自动重连
    wait_for_condition(|| async {
        is_mqtt_connected(&mqtt_client).await
    }, Duration::from_secs(35)).await; // 最大重连间隔 30s
    
    assert!(is_mqtt_connected(&mqtt_client).await);
    
    broker.stop().await;
}
```

### 5.3 HTTP 上传集成测试

```rust
// tests/integration/http_upload_test.rs

#[tokio::test]
async fn test_http_upload_success() {
    // 启动文件接收器
    let receiver = FileReceiver::start("0.0.0.0:18080").await;
    
    // 创建测试任务
    let task = create_test_task("http://localhost:18080/files/test.cba");
    
    // 执行上传
    let result = process_sync_task(&task).await;
    
    assert!(result.is_ok());
    
    // 验证文件已接收
    let received_file = receiver.get_file("test.cba").await;
    assert!(received_file.is_some());
    assert_eq!(received_file.unwrap().len(), task.file_size as usize);
    
    receiver.stop().await;
}

#[tokio::test]
async fn test_http_upload_retry_on_timeout() {
    // 启动慢速接收器（模拟超时）
    let receiver = SlowFileReceiver::start("0.0.0.0:18080")
        .with_delay(Duration::from_secs(40)); // 超过 30s 超时
    
    let task = create_test_task("http://localhost:18080/files/test.cba");
    
    let result = process_sync_task(&task).await;
    
    // 应该失败并触发重试
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
    
    receiver.stop().await;
}

#[tokio::test]
async fn test_http_upload_404_handling() {
    let receiver = FileReceiver::start("0.0.0.0:18080").await;
    
    // 错误的 URL
    let task = create_test_task("http://localhost:18080/wrong/path/test.cba");
    
    let result = process_sync_task(&task).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("404"));
    
    receiver.stop().await;
}
```

### 5.4 数据持久化测试

```rust
// tests/integration/database_persistence_test.rs

#[tokio::test]
async fn test_task_log_persistence() {
    let temp_db = create_temp_sqlite_db().await;
    
    let mut center = SyncControlCenter::new();
    
    // 添加任务
    let task_id = center.add_task(NewSyncTaskParams {
        file_path: "test.cba".into(),
        file_size: 1024,
        priority: 5,
        env_id: Some("test-env".into()),
        target_site: Some("test-site".into()),
        ..Default::default()
    });
    
    // 验证 pending 日志
    let logs = query_logs_from_sqlite(&temp_db, "pending").await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].task_id, task_id);
    assert_eq!(logs[0].status, "pending");
    
    // 执行任务
    let task = center.get_next_task().unwrap();
    
    // 验证 running 日志
    let logs = query_logs_from_sqlite(&temp_db, "running").await;
    assert_eq!(logs.len(), 1);
    assert!(logs[0].started_at.is_some());
    
    // 完成任务
    center.complete_task(&task.id, true, None);
    
    // 验证 completed 日志
    let logs = query_logs_from_sqlite(&temp_db, "completed").await;
    assert_eq!(logs.len(), 1);
    assert!(logs[0].completed_at.is_some());
    
    temp_db.cleanup().await;
}

#[tokio::test]
async fn test_service_restart_recovery() {
    let temp_db = create_temp_sqlite_db().await;
    
    // 第一次启动，添加任务
    {
        let mut center = SyncControlCenter::new();
        center.add_task(NewSyncTaskParams {
            file_path: "test.cba".into(),
            file_size: 1024,
            priority: 5,
            ..Default::default()
        });
        // 服务崩溃，未完成任务
    }
    
    // 第二次启动，恢复任务
    {
        let mut center = SyncControlCenter::new();
        
        // 从数据库恢复未完成任务
        let pending_logs = query_logs_from_sqlite(&temp_db, "pending").await;
        let running_logs = query_logs_from_sqlite(&temp_db, "running").await;
        
        for log in pending_logs.into_iter().chain(running_logs) {
            // 重新入队
            center.add_task(NewSyncTaskParams {
                file_path: log.file_path.unwrap(),
                file_size: log.file_size.unwrap() as u64,
                priority: 5,
                ..Default::default()
            });
        }
        
        assert!(center.state.queue_size > 0);
    }
    
    temp_db.cleanup().await;
}
```

---

## 端到端测试方案

### 6.1 真实文件同步测试

```rust
// tests/e2e/real_file_sync_test.rs

#[tokio::test]
#[ignore] // 标记为 E2E 测试，需要完整环境
async fn test_real_pdms_file_sync() {
    // 这个测试需要真实的 PDMS 文件
    let test_env = RealTestEnvironment::new().await;
    
    // 1. 准备真实的 PDMS 文件
    let pdms_file = test_env.get_sample_pdms_file("CATA.db");
    assert!(pdms_file.exists());
    
    // 2. 启动完整服务
    test_env.start_full_stack().await;
    
    // 3. 修改 PDMS 文件（模拟设计软件修改）
    modify_pdms_file_add_sessions(&pdms_file, 5).await;
    
    // 4. 等待自动同步
    wait_for_sync_completion(&test_env, Duration::from_secs(60)).await;
    
    // 5. 验证远程文件
    let remote_file = test_env.get_remote_file("CATA.cba").await.unwrap();
    assert!(remote_file.len() > 0);
    
    // 6. 验证元数据
    let metadata = test_env.get_remote_metadata().await.unwrap();
    let entry = metadata.entries.iter()
        .find(|e| e.file_name == "CATA.cba")
        .unwrap();
    
    assert!(entry.record_count.unwrap() >= 5);
    
    test_env.cleanup().await;
}
```

### 6.2 多站点同步测试

```rust
// tests/e2e/multi_site_sync_test.rs

#[tokio::test]
#[ignore]
async fn test_sync_to_multiple_sites() {
    let test_env = MultiSiteTestEnvironment::new().await;
    
    // 配置 3 个目标站点
    test_env.add_site("site-bj", "http://192.168.1.100:8080/files");
    test_env.add_site("site-sh", "http://192.168.1.101:8080/files");
    test_env.add_site("site-gz", "http://192.168.1.102:8080/files");
    
    // 启动服务
    test_env.start_sync_service().await;
    
    // 触发增量更新
    let pdms_file = test_env.get_pdms_file("CATA.db");
    modify_pdms_file(&pdms_file).await;
    
    // 等待所有站点同步完成
    wait_for_all_sites_synced(&test_env, Duration::from_secs(120)).await;
    
    // 验证所有站点都收到文件
    for site_name in ["site-bj", "site-sh", "site-gz"] {
        let file = test_env.get_site_file(site_name, "CATA.cba").await;
        assert!(file.is_some(), "站点 {} 未收到文件", site_name);
    }
    
    // 验证日志记录
    let logs = test_env.query_logs().await;
    assert_eq!(logs.len(), 3); // 3 个站点各 1 条日志
    assert!(logs.iter().all(|log| log.status == "completed"));
    
    test_env.cleanup().await;
}
```

---

## 性能测试方案

### 7.1 吞吐量测试

```rust
// tests/performance/throughput_test.rs

#[tokio::test]
#[ignore]
async fn test_throughput_100_tasks_per_minute() {
    let test_env = PerformanceTestEnvironment::new().await;
    test_env.start_services().await;
    
    let mut center = SyncControlCenter::new();
    center.config.max_concurrent_syncs = 10; // 提高并发
    center.start("perf-test".into()).await.unwrap();
    
    let start = Instant::now();
    
    // 添加 100 个任务
    for i in 0..100 {
        center.add_task(NewSyncTaskParams {
            file_path: format!("test_{}.cba", i),
            file_size: 1024 * 100, // 100KB
            priority: 5,
            ..Default::default()
        });
    }
    
    // 等待全部完成
    wait_for_condition(|| async {
        let state = center.get_state_snapshot();
        state.total_synced == 100
    }, Duration::from_secs(120)).await;
    
    let duration = start.elapsed();
    let tasks_per_min = 100.0 / duration.as_secs_f64() * 60.0;
    
    println!("吞吐量: {:.2} 任务/分钟", tasks_per_min);
    println!("平均耗时: {:.2} 秒/任务", duration.as_secs_f64() / 100.0);
    
    // 性能要求：至少 100 任务/分钟
    assert!(tasks_per_min >= 100.0, "吞吐量不达标: {:.2}", tasks_per_min);
    
    test_env.cleanup().await;
}
```

### 7.2 并发压力测试

```rust
#[tokio::test]
#[ignore]
async fn test_concurrent_stress_1000_tasks() {
    let test_env = PerformanceTestEnvironment::new().await;
    test_env.start_services().await;
    
    let mut center = SyncControlCenter::new();
    center.config.max_concurrent_syncs = 20;
    center.start("stress-test".into()).await.unwrap();
    
    // 快速添加 1000 个任务
    let start = Instant::now();
    for i in 0..1000 {
        center.add_task(NewSyncTaskParams {
            file_path: format!("stress_{}.cba", i),
            file_size: 1024 * 50,
            priority: (i % 10) as u8, // 不同优先级
            ..Default::default()
        });
    }
    
    let queue_time = start.elapsed();
    println!("入队耗时: {:?}", queue_time);
    
    // 等待全部完成
    wait_for_condition(|| async {
        let state = center.get_state_snapshot();
        (state.total_synced + state.total_failed) == 1000
    }, Duration::from_secs(600)).await;
    
    let total_time = start.elapsed();
    let state = center.get_state_snapshot();
    
    println!("总耗时: {:?}", total_time);
    println!("成功: {}, 失败: {}", state.total_synced, state.total_failed);
    println!("成功率: {:.2}%", state.total_synced as f64 / 1000.0 * 100.0);
    
    // 验证成功率
    assert!(state.total_synced >= 950, "成功率低于 95%");
    
    test_env.cleanup().await;
}
```

### 7.3 大文件传输测试

```rust
#[tokio::test]
#[ignore]
async fn test_large_file_transfer() {
    let test_env = PerformanceTestEnvironment::new().await;
    test_env.start_services().await;
    
    // 创建 100MB 测试文件
    let large_file = test_env.create_large_file(100 * 1024 * 1024);
    
    let mut center = SyncControlCenter::new();
    center.start("large-file-test".into()).await.unwrap();
    
    let start = Instant::now();
    
    let task_id = center.add_task(NewSyncTaskParams {
        file_path: large_file.to_str().unwrap().into(),
        file_size: 100 * 1024 * 1024,
        priority: 10,
        ..Default::default()
    });
    
    // 等待完成
    wait_for_task_completion(&center, &task_id, Duration::from_secs(300)).await;
    
    let duration = start.elapsed();
    let mbps = (100.0 / duration.as_secs_f64()) * 8.0;
    
    println!("传输耗时: {:?}", duration);
    println!("传输速率: {:.2} Mbps", mbps);
    
    // 验证文件完整性
    let remote_file = test_env.get_received_file("large_file.cba").await.unwrap();
    assert_eq!(remote_file.len(), 100 * 1024 * 1024);
    
    test_env.cleanup().await;
}
```

---

## 故障注入测试

### 8.1 网络故障测试

```rust
// tests/fault_injection/network_failure_test.rs

#[tokio::test]
async fn test_retry_on_network_timeout() {
    let test_env = FaultInjectionTestEnvironment::new().await;
    
    // 配置故障注入：前 2 次请求超时，第 3 次成功
    test_env.inject_http_fault(
        FaultPattern::TimeoutFirst(2)
    );
    
    let mut center = SyncControlCenter::new();
    center.config.auto_retry = true;
    center.config.max_retries = 3;
    center.start("fault-test".into()).await.unwrap();
    
    let task_id = center.add_task(NewSyncTaskParams {
        file_path: "test.cba".into(),
        file_size: 1024,
        priority: 5,
        ..Default::default()
    });
    
    // 等待任务完成（经过重试）
    wait_for_task_completion(&center, &task_id, Duration::from_secs(30)).await;
    
    let state = center.get_state_snapshot();
    assert_eq!(state.total_synced, 1);
    
    // 验证重试次数
    let logs = test_env.query_logs().await;
    assert!(logs[0].retry_count >= 2);
    
    test_env.cleanup().await;
}

#[tokio::test]
async fn test_failure_after_max_retries() {
    let test_env = FaultInjectionTestEnvironment::new().await;
    
    // 始终失败
    test_env.inject_http_fault(FaultPattern::AlwaysFail);
    
    let mut center = SyncControlCenter::new();
    center.config.max_retries = 3;
    center.start("fault-test".into()).await.unwrap();
    
    let task_id = center.add_task(NewSyncTaskParams {
        file_path: "test.cba".into(),
        file_size: 1024,
        priority: 5,
        ..Default::default()
    });
    
    // 等待重试耗尽
    sleep(Duration::from_secs(20)).await;
    
    let state = center.get_state_snapshot();
    assert_eq!(state.total_failed, 1);
    
    let logs = test_env.query_logs().await;
    assert_eq!(logs[0].status, "failed");
    assert_eq!(logs[0].retry_count, 3);
    
    test_env.cleanup().await;
}
```

### 8.2 MQTT 断线测试

```rust
#[tokio::test]
async fn test_mqtt_disconnect_and_reconnect() {
    let broker = TestMqttBroker::start().await;
    let mgr = AiosDBManager::init_from_config().await.unwrap();
    
    // 验证初始连接
    assert!(is_mqtt_connected(&mgr).await);
    
    // 断开 MQTT
    broker.disconnect_client("test-client").await;
    sleep(Duration::from_secs(1)).await;
    assert!(!is_mqtt_connected(&mgr).await);
    
    // 触发增量更新（应该缓存消息）
    trigger_increment_update(&mgr, "CATA.db", 100, 105).await;
    
    // 重连
    sleep(Duration::from_secs(5)).await; // 等待自动重连
    assert!(is_mqtt_connected(&mgr).await);
    
    // 验证消息已发送
    let subscriber = broker.get_messages("Sync/E3d").await;
    assert!(!subscriber.is_empty());
    
    broker.stop().await;
}
```

### 8.3 文件系统故障测试

```rust
#[tokio::test]
async fn test_file_not_found_error() {
    let mut center = SyncControlCenter::new();
    center.start("fault-test".into()).await.unwrap();
    
    // 添加不存在的文件
    let task_id = center.add_task(NewSyncTaskParams {
        file_path: "/nonexistent/file.cba".into(),
        file_size: 1024,
        priority: 5,
        ..Default::default()
    });
    
    let task = center.get_next_task().unwrap();
    let result = process_sync_task(&task).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("无法访问"));
    
    center.complete_task(&task.id, false, Some("文件不存在".into()));
    
    let state = center.get_state_snapshot();
    assert_eq!(state.total_failed, 1);
}
```

---

## 测试工具和框架

### 9.1 Mock 工具

```rust
// tests/helpers/mock_file_watcher.rs

pub struct MockFileWatcher {
    events: Vec<notify::Event>,
    event_index: usize,
}

impl MockFileWatcher {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            event_index: 0,
        }
    }
    
    pub fn add_modify_event(&mut self, path: PathBuf) {
        self.events.push(notify::Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any
            )),
            paths: vec![path],
            ..Default::default()
        });
    }
    
    pub async fn next_event(&mut self) -> Option<notify::Event> {
        if self.event_index < self.events.len() {
            let event = self.events[self.event_index].clone();
            self.event_index += 1;
            Some(event)
        } else {
            None
        }
    }
}

// tests/helpers/mock_mqtt_client.rs

pub struct MockMqttClient {
    published_messages: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
}

impl MockMqttClient {
    pub fn new() -> Self {
        Self {
            published_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub async fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<()> {
        let mut messages = self.published_messages.lock().await;
        messages.push((topic.to_string(), payload));
        Ok(())
    }
    
    pub async fn get_published_messages(&self) -> Vec<(String, Vec<u8>)> {
        self.published_messages.lock().await.clone()
    }
}

// tests/helpers/test_db_builder.rs

pub struct TestDatabaseBuilder {
    sqlite_path: PathBuf,
    envs: Vec<RemoteSyncEnv>,
    sites: Vec<RemoteSyncSite>,
}

impl TestDatabaseBuilder {
    pub fn new() -> Self {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        Self {
            sqlite_path: temp_file.path().to_path_buf(),
            envs: Vec::new(),
            sites: Vec::new(),
        }
    }
    
    pub fn add_env(mut self, env: RemoteSyncEnv) -> Self {
        self.envs.push(env);
        self
    }
    
    pub fn add_site(mut self, site: RemoteSyncSite) -> Self {
        self.sites.push(site);
        self
    }
    
    pub async fn build(self) -> TestDatabase {
        let conn = rusqlite::Connection::open(&self.sqlite_path).unwrap();
        
        // 创建表
        create_tables(&conn).unwrap();
        
        // 插入数据
        for env in &self.envs {
            insert_env(&conn, env).unwrap();
        }
        for site in &self.sites {
            insert_site(&conn, site).unwrap();
        }
        
        TestDatabase {
            path: self.sqlite_path,
            conn,
        }
    }
}
```

### 9.2 断言工具

```rust
// tests/helpers/assertions.rs

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
        
        if center.history.iter().any(|t| t.id == task_id && t.status == SyncTaskStatus::Completed) {
            return;
        }
        
        sleep(Duration::from_millis(100)).await;
    }
}

pub async fn assert_file_exists(path: &Path) {
    assert!(path.exists(), "文件不存在: {:?}", path);
}

pub async fn assert_file_size(path: &Path, expected_size: u64) {
    let metadata = fs::metadata(path).await.unwrap();
    assert_eq!(metadata.len(), expected_size, "文件大小不匹配");
}

pub async fn assert_log_entry_exists(
    sqlite_path: &Path,
    task_id: &str,
    status: &str,
) {
    let conn = rusqlite::Connection::open(sqlite_path).unwrap();
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM remote_sync_logs WHERE task_id = ?1 AND status = ?2"
    ).unwrap();
    
    let count: i64 = stmt.query_row([task_id, status], |row| row.get(0)).unwrap();
    assert!(count > 0, "未找到日志: task_id={}, status={}", task_id, status);
}
```

---

## 测试数据准备

### 10.1 PDMS 文件生成工具

```rust
// tests/fixtures/pdms_file_generator.rs

pub struct PdmsFileGenerator {
    base_sesno: i32,
    db_num: i32,
}

impl PdmsFileGenerator {
    pub fn new() -> Self {
        Self {
            base_sesno: 12340,
            db_num: 1,
        }
    }
    
    pub fn with_sesno(mut self, sesno: i32) -> Self {
        self.base_sesno = sesno;
        self
    }
    
    pub fn with_db_num(mut self, db_num: i32) -> Self {
        self.db_num = db_num;
        self
    }
    
    pub async fn generate(&self, output_path: &Path) -> Result<PathBuf> {
        // 生成最小的 PDMS 文件头部
        let mut file = File::create(output_path).await?;
        
        // 写入文件头（简化版）
        write_pdms_header(&mut file, self.base_sesno, self.db_num).await?;
        
        Ok(output_path.to_path_buf())
    }
    
    pub async fn generate_with_increments(
        &self,
        output_path: &Path,
        increment_count: usize,
    ) -> Result<PathBuf> {
        self.generate(output_path).await?;
        
        // 添加增量会话
        for i in 1..=increment_count {
            append_session(output_path, self.base_sesno + i as i32).await?;
        }
        
        Ok(output_path.to_path_buf())
    }
}

async fn write_pdms_header(file: &mut File, sesno: i32, db_num: i32) -> Result<()> {
    // 简化的 PDMS 头部格式
    // 实际需要根据 PDMS 格式规范编写
    let header = vec![0u8; 512]; // 占位
    file.write_all(&header).await?;
    Ok(())
}
```

### 10.2 测试数据集

```rust
// tests/fixtures/test_datasets.rs

pub struct TestDatasets;

impl TestDatasets {
    /// 小型数据集（快速测试）
    pub fn small() -> TestDataset {
        TestDataset {
            pdms_files: vec![
                ("CATA.db", 12340, 1024),
            ],
            expected_increments: vec![
                ("CATA", 12340..=12345, 5),
            ],
        }
    }
    
    /// 中型数据集（常规测试）
    pub fn medium() -> TestDataset {
        TestDataset {
            pdms_files: vec![
                ("CATA.db", 5000, 1024 * 100),
                ("DESI.db", 3000, 1024 * 50),
                ("DICT.db", 1000, 1024 * 20),
            ],
            expected_increments: vec![
                ("CATA", 5000..=5010, 10),
                ("DESI", 3000..=3005, 5),
            ],
        }
    }
    
    /// 大型数据集（压力测试）
    pub fn large() -> TestDataset {
        TestDataset {
            pdms_files: vec![
                ("CATA.db", 50000, 1024 * 1024 * 10),
                ("DESI.db", 30000, 1024 * 1024 * 5),
                // ... 更多文件
            ],
            expected_increments: vec![
                ("CATA", 50000..=50100, 100),
                // ... 更多增量
            ],
        }
    }
}
```

---

## 自动化测试流程

### 11.1 CI/CD 集成

```yaml
# .github/workflows/test.yml

name: 异地更新系统测试

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: 安装 Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: 运行单元测试
        run: cargo test --lib --features web_server
        
      - name: 生成覆盖率报告
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml --features web_server
          
      - name: 上传覆盖率
        uses: codecov/codecov-action@v3
  
  integration-tests:
    runs-on: ubuntu-latest
    services:
      surrealdb:
        image: surrealdb/surrealdb:latest
        ports:
          - 8000:8000
      mosquitto:
        image: eclipse-mosquitto:2
        ports:
          - 1883:1883
          
    steps:
      - uses: actions/checkout@v3
      
      - name: 运行集成测试
        run: cargo test --test '*' --features web_server
        env:
          SURREALDB_URL: http://localhost:8000
          MQTT_BROKER: localhost:1883
  
  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: 启动完整环境
        run: docker-compose -f docker-compose.test.yml up -d
        
      - name: 等待服务就绪
        run: ./tests/scripts/wait-for-services.sh
        
      - name: 运行端到端测试
        run: cargo test --test '*' --features web_server -- --ignored
        
      - name: 收集日志
        if: failure()
        run: docker-compose -f docker-compose.test.yml logs > test-logs.txt
        
      - name: 上传日志
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: test-logs
          path: test-logs.txt
```

### 11.2 本地测试脚本

```bash
#!/bin/bash
# tests/run_all_tests.sh

set -e

echo "🧪 运行异地更新系统完整测试套件"

echo ""
echo "📦 1. 启动测试环境..."
docker-compose -f docker-compose.test.yml up -d
sleep 5

echo ""
echo "✅ 2. 运行单元测试..."
cargo test --lib --features web_server -- --nocapture

echo ""
echo "🔗 3. 运行集成测试..."
cargo test --test integration --features web_server -- --nocapture

echo ""
echo "🌐 4. 运行端到端测试..."
cargo test --test e2e --features web_server -- --ignored --nocapture

echo ""
echo "⚡ 5. 运行性能测试..."
cargo test --test performance --features web_server --release -- --ignored --nocapture

echo ""
echo "💥 6. 运行故障注入测试..."
cargo test --test fault_injection --features web_server -- --ignored --nocapture

echo ""
echo "🧹 7. 清理测试环境..."
docker-compose -f docker-compose.test.yml down -v

echo ""
echo "✨ 所有测试完成！"
```

---

## 测试覆盖率要求

### 12.1 代码覆盖率目标

| 模块 | 目标覆盖率 | 当前覆盖率 |
|------|-----------|-----------|
| sync_control_center.rs | 80% | 待测试 |
| remote_runtime.rs | 75% | 待测试 |
| increment_manager.rs | 70% | 待测试 |
| process_sync_task() | 85% | 待测试 |
| remote_sync_handlers.rs | 60% | 待测试 |
| site_metadata.rs | 70% | 待测试 |
| **总体** | **≥ 70%** | **待测试** |

### 12.2 关键路径覆盖

必须 100% 覆盖的关键路径：

1. ✅ 增量检测流程
   - 文件监听 → 会话号查询 → 对比 → 入队

2. ✅ 任务执行流程
   - 获取任务 → 解析目标 → 传输文件 → 更新状态

3. ✅ 重试机制
   - 失败检测 → 重试计数 → 重新入队 → 最终失败

4. ✅ 并发控制
   - 并发限制检查 → 任务分配 → 完成释放

---

## 总结

### 测试实施优先级

#### 第一阶段（1周）- 基础测试
1. ✅ 单元测试：任务队列管理
2. ✅ 单元测试：增量检测逻辑
3. ✅ 单元测试：目标解析
4. ✅ 集成测试：SQLite 持久化

#### 第二阶段（1周）- 集成测试
5. ✅ 集成测试：HTTP 上传
6. ✅ 集成测试：MQTT 发布
7. ✅ 集成测试：完整工作流
8. ✅ 故障注入：重试机制

#### 第三阶段（1周）- E2E 和性能
9. ✅ 端到端测试：真实文件同步
10. ✅ 端到端测试：多站点
11. ✅ 性能测试：吞吐量
12. ✅ 性能测试：并发压力

### 关键成功因素

1. **测试隔离**: 使用 Docker 和临时数据库
2. **Mock 依赖**: 减少外部依赖，提高测试速度
3. **自动化**: CI/CD 集成，自动运行测试
4. **覆盖率**: 持续监控，保持 ≥ 70%
5. **真实数据**: 使用真实 PDMS 文件验证

---

**文档结束**

*下一步：实施测试代码并集成到 CI/CD 流程*
