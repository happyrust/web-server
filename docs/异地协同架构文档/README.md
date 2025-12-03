# 异地协同架构文档

本目录包含 AIOS 异地协同功能的完整技术文档。

## 文档索引

| 文档 | 说明 |
|------|------|
| [remote_sync_workflow.md](./remote_sync_workflow.md) | 运行原理与技术架构 |
| [deployment_guide.md](./deployment_guide.md) | 部署与配置指南 |
| [operation_manual.md](./operation_manual.md) | 操作手册与日常运维 |
| [troubleshooting.md](./troubleshooting.md) | 故障排查指南 |

## 快速开始

### 主节点部署
```bash
# 1. 配置 DbOption.toml
location = "master"
is_master_node = true
location_dbs = [1112, 1113]  # 本节点负责的数据库

# 2. 启动服务
cargo run --bin web_server --features web_server

# 3. 通过前端启动 MQTT
# 访问 http://localhost:8080 → MQTT 节点监控
# 点击 "启动 Broker" → 点击 "启动订阅"
```

### 从节点部署
```bash
# 1. 配置 DbOption.toml
location = "client-bj"
is_master_node = false
master_mqtt_host = "192.168.1.100"  # 主节点 IP
master_mqtt_port = 1883

# 2. 启动服务
cargo run --bin web_server --features web_server

# 3. 通过前端启动订阅
# 访问 http://localhost:8080 → MQTT 节点监控
# 点击 "启动订阅"（自动连接主节点）
```

## 系统架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                         异地协同系统架构                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐          ┌─────────────┐          ┌─────────────┐ │
│  │  主节点 A   │          │ MQTT Broker │          │  从节点 B   │ │
│  │  (Master)   │          │  (rumqttd)  │          │  (Client)   │ │
│  └──────┬──────┘          └──────┬──────┘          └──────┬──────┘ │
│         │                        │                        │        │
│         │    ┌───────────────────┼───────────────────┐    │        │
│         │    │                   │                   │    │        │
│         ▼    ▼                   ▼                   ▼    ▼        │
│  ┌────────────────┐       ┌────────────┐      ┌────────────────┐  │
│  │ File Watcher   │       │  Sync/E3d  │      │ MQTT Subscriber │  │
│  │ (async_watch)  │       │   Topic    │      │ (poll_events)   │  │
│  └───────┬────────┘       └─────┬──────┘      └───────┬─────────┘  │
│          │                      │                     │            │
│          ▼                      │                     ▼            │
│  ┌────────────────┐             │             ┌────────────────┐   │
│  │ CBA Generator  │─────────────┼─────────────│ CBA Downloader │   │
│  │ (增量压缩)      │             │             │ (HTTP 下载)     │   │
│  └───────┬────────┘             │             └───────┬─────────┘  │
│          │                      │                     │            │
│          ▼                      │                     ▼            │
│  ┌────────────────┐             │             ┌────────────────┐   │
│  │ HTTP Server    │◄────────────┴─────────────│ Clone Engine   │   │
│  │ (CBA 文件服务)  │                           │ (增量应用)      │   │
│  └────────────────┘                           └────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## 数据流向

```
文件变化 → 增量检测 → CBA生成 → MQTT发布 → 从节点接收 → HTTP下载 → 增量应用
    │                    │           │            │           │          │
    ▼                    ▼           ▼            ▼           ▼          ▼
 notify事件        assets/archives  Sync/E3d   订阅处理   临时目录   本地数据库
```

## 技术栈

- **消息队列**: rumqttc / rumqttd (MQTT v5)
- **文件监听**: notify (inotify/FSEvents/ReadDirectoryChanges)
- **HTTP 服务**: axum
- **数据库**: SurrealDB + SQLite
- **前端**: Vue 3 + TailwindCSS
