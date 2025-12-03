# 异地协同部署指南

## 目录

1. [环境要求](#环境要求)
2. [网络规划](#网络规划)
3. [主节点部署](#主节点部署)
4. [从节点部署](#从节点部署)
5. [配置详解](#配置详解)
6. [验证部署](#验证部署)

---

## 环境要求

### 硬件要求

| 组件 | 最低配置 | 推荐配置 |
|------|---------|---------|
| CPU | 2 核 | 4 核+ |
| 内存 | 4 GB | 8 GB+ |
| 磁盘 | 50 GB SSD | 100 GB+ SSD |
| 网络 | 10 Mbps | 100 Mbps+ |

### 软件要求

- 操作系统: Windows 10/11, Windows Server 2016+, Linux (CentOS 7+, Ubuntu 18.04+)
- Rust: nightly (通过 rustup 安装)
- MQTT Broker: rumqttd (内置) 或外部 MQTT Broker

### 端口规划

| 端口 | 服务 | 说明 |
|------|------|------|
| 8080 | Web Server | HTTP API 和前端界面 |
| 1883 | MQTT Broker | MQTT 消息服务 (仅主节点) |
| 8000 | SurrealDB | 数据库服务 (可选) |

---

## 网络规划

### 典型部署拓扑

```
                    ┌─────────────────────┐
                    │     互联网/专线      │
                    └──────────┬──────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│   主节点 A     │      │   从节点 B     │      │   从节点 C     │
│ (北京总部)     │      │ (上海分部)     │      │ (广州分部)     │
│               │      │               │      │               │
│ MQTT Broker   │◄────►│ MQTT Client   │◄────►│ MQTT Client   │
│ :1883         │      │               │      │               │
│               │      │               │      │               │
│ Web Server    │      │ Web Server    │      │ Web Server    │
│ :8080         │      │ :8080         │      │ :8080         │
│               │      │               │      │               │
│ location_dbs  │      │ location_dbs  │      │ location_dbs  │
│ = [1001-1100] │      │ = [2001-2100] │      │ = [3001-3100] │
└───────────────┘      └───────────────┘      └───────────────┘
```

### 防火墙规则

**主节点需要开放：**
```bash
# MQTT Broker 端口 (供从节点连接)
iptables -A INPUT -p tcp --dport 1883 -j ACCEPT

# HTTP 文件服务 (供从节点下载 CBA)
iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
```

**从节点需要开放：**
```bash
# 仅需开放 Web 界面端口 (如果需要远程访问)
iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
```

---

## 主节点部署

### 步骤 1: 准备配置文件

编辑 `DbOption.toml`:

```toml
# ============================================
# 主节点配置示例
# ============================================

# 项目基本信息
project_path = "D:/work/plant/ams1112"
project_code = "ams1112"

# 节点标识 (重要：每个节点必须唯一)
location = "master-bj"

# 主节点标志
is_master_node = true

# MQTT 配置 (主节点作为 Broker)
mqtt_host = "0.0.0.0"
mqtt_port = 1883

# 文件服务器地址 (供其他节点下载 CBA)
file_server_host = "http://192.168.1.100:8080"

# 本节点负责的数据库编号
# 只有在此列表中的数据库变更才会推送
location_dbs = [1112, 1113, 1114]

# 实时同步开关
sync_live = true

# 推送的数据库类型
sync_push_db_types = ["DESI", "CATA"]
```

### 步骤 2: 启动服务

```bash
# 编译 (首次或代码更新后)
cargo build --bin web_server --features web_server --release

# 启动服务
cargo run --bin web_server --features web_server --release
```

### 步骤 3: 启动 MQTT 功能

1. 打开浏览器访问 `http://localhost:8080`
2. 进入 **MQTT 节点监控** 页面
3. 确认显示 **节点角色: 主节点**
4. 点击 **启动 Broker** 按钮
5. 等待显示 **Broker: 运行中**
6. 点击 **启动订阅** 按钮
7. 等待显示 **订阅: 已订阅**

### 步骤 4: 验证主节点

```bash
# 检查 MQTT Broker 是否监听
netstat -an | findstr 1883

# 检查日志输出
# 应该看到:
# ℹ️  MQTT 功能由前端控制，请通过 MQTT 监控页面启动订阅
# [Publisher] Connected to MQTT broker.
```

---

## 从节点部署

### 步骤 1: 准备配置文件

编辑 `DbOption.toml`:

```toml
# ============================================
# 从节点配置示例
# ============================================

# 项目基本信息
project_path = "D:/work/plant/ams1112"
project_code = "ams1112"

# 节点标识 (重要：每个节点必须唯一)
location = "client-sh"

# 从节点标志
is_master_node = false

# 主节点 MQTT 地址 (从节点需要连接主节点)
master_mqtt_host = "192.168.1.100"
master_mqtt_port = 1883

# 文件服务器地址 (主节点的 HTTP 服务)
file_server_host = "http://192.168.1.100:8080"

# 本节点负责的数据库编号
# 这些数据库的变更会推送，但不会从远程拉取
location_dbs = [2001, 2002]

# 实时同步开关
sync_live = true

# 推送的数据库类型
sync_push_db_types = ["DESI", "CATA"]
```

### 步骤 2: 启动服务

```bash
# 编译 (首次或代码更新后)
cargo build --bin web_server --features web_server --release

# 启动服务
cargo run --bin web_server --features web_server --release
```

### 步骤 3: 启动 MQTT 订阅

1. 打开浏览器访问 `http://localhost:8080`
2. 进入 **MQTT 节点监控** 页面
3. 确认显示 **节点角色: 从节点**
4. 点击 **启动订阅** 按钮
5. 等待显示 **订阅: 已订阅**

### 步骤 4: 验证从节点

```bash
# 检查日志输出
# 应该看到:
# 从节点模式：连接到主节点MQTT服务器 192.168.1.100:1883
# [Subscriber] Connected to MQTT broker
```

---

## 配置详解

### 核心配置项

| 配置项 | 类型 | 说明 | 示例 |
|-------|------|------|------|
| `location` | string | 节点唯一标识 | `"master-bj"` |
| `is_master_node` | bool | 是否为主节点 | `true` |
| `mqtt_host` | string | MQTT 监听地址 | `"0.0.0.0"` |
| `mqtt_port` | int | MQTT 端口 | `1883` |
| `master_mqtt_host` | string | 主节点 MQTT 地址 | `"192.168.1.100"` |
| `master_mqtt_port` | int | 主节点 MQTT 端口 | `1883` |
| `file_server_host` | string | CBA 文件服务器地址 | `"http://192.168.1.100:8080"` |
| `location_dbs` | array | 本节点负责的数据库 | `[1112, 1113]` |
| `sync_live` | bool | 启用实时同步 | `true` |
| `sync_push_db_types` | array | 推送的数据库类型 | `["DESI", "CATA"]` |

### location_dbs 配置说明

```
┌─────────────────────────────────────────────────────────────────┐
│                    location_dbs 过滤机制                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  配置: location_dbs = [1112, 1113]                              │
│                                                                 │
│  推送行为:                                                       │
│  ┌─────────────┐                                                │
│  │ DB 1112 变化 │ ──► 在列表中 ──► ✅ 生成CBA + 推送MQTT         │
│  └─────────────┘                                                │
│  ┌─────────────┐                                                │
│  │ DB 2001 变化 │ ──► 不在列表 ──► ❌ 跳过推送                    │
│  └─────────────┘                                                │
│                                                                 │
│  拉取行为:                                                       │
│  ┌─────────────┐                                                │
│  │ 收到 DB 1112 │ ──► 在列表中 ──► ❌ 跳过拉取 (本地负责)         │
│  └─────────────┘                                                │
│  ┌─────────────┐                                                │
│  │ 收到 DB 2001 │ ──► 不在列表 ──► ✅ 下载CBA + 应用增量          │
│  └─────────────┘                                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 验证部署

### 1. 检查节点状态

访问主节点的 MQTT 监控页面，应该能看到：

```
┌─────────────────────────────────────────────┐
│  在线节点: 2  |  离线节点: 0  |  总节点数: 2 │
└─────────────────────────────────────────────┘
```

### 2. 测试文件同步

在主节点修改一个 PDMS 数据库文件，观察日志：

**主节点日志:**
```
changed: Event { kind: Modify(Data(Any)), paths: ["D:\\work\\plant\\ams1112\\1112\\d001112"] }
开始扫描数据库头部信息，路径: ["D:\\work\\plant\\ams1112\\1112\\d001112"]
成功扫描到 1 个数据库头部
检测到文件变化: d001112, SESNO: 1212 -> 1213
生成 CBA 文件: assets/archives/d001112_1213.cba
✅ MQTT 消息已发布到 Sync/E3d
```

**从节点日志:**
```
[Subscriber] Received message on topic: Sync/E3d
收到同步消息: sender=master-bj, files=1
开始下载 CBA: http://192.168.1.100:8080/assets/archives/d001112_1213.cba
下载完成，SHA256 校验通过
执行增量克隆: d001112_1213.cba -> D:\work\plant\ams1112\1112\d001112
✅ 增量应用完成
```

### 3. 检查数据一致性

在从节点查询 SurrealDB，验证数据已同步：

```sql
SELECT * FROM pe WHERE SESNO = 1213;
```

---

## 常见问题

### Q: 从节点连接不上主节点 MQTT?

A: 检查以下项目：
1. 主节点防火墙是否开放 1883 端口
2. `master_mqtt_host` 配置是否正确
3. 主节点 MQTT Broker 是否已启动

### Q: CBA 文件下载失败?

A: 检查以下项目：
1. `file_server_host` 配置是否正确
2. 主节点 8080 端口是否可访问
3. CBA 文件是否存在于 `assets/archives/` 目录

### Q: 文件变化未触发同步?

A: 检查以下项目：
1. `sync_live` 是否设置为 `true`
2. 数据库编号是否在 `location_dbs` 列表中
3. 数据库类型是否在 `sync_push_db_types` 列表中
