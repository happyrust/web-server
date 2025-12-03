# MQTT Server 使用说明

## 概述

MQTT Server 用于异地同步系统中的消息传递。web-server 可以通过 Web UI 启动和管理 MQTT 服务器。

## 文件要求

在 `site-marine/bin` 目录下需要以下文件：

1. **rumqttd.exe** - MQTT broker 可执行文件
   - Windows: `rumqttd.exe`
   - Linux: `rumqttd`
   - 安装方式：`cargo install rumqttd`

2. **rumqttd.toml** - MQTT 服务器配置文件
   - 已包含在 `site-marine/bin` 目录
   - 默认监听端口：1883（MQTT）
   - 管理控制台端口：18083

## 启动方式

### 方式一：通过 Web UI 启动（推荐）

1. 启动 web-server：
   ```bash
   web_server.exe
   ```

2. 访问 Web UI：`http://localhost:8080`

3. 导航到"异地同步"页面

4. 点击"启动 MQTT 服务器"按钮

### 方式二：通过 API 启动

```bash
curl -X POST http://localhost:8080/api/sync/mqtt/start \
  -H "Content-Type: application/json" \
  -d '{"port": 1883}'
```

## 配置说明

### rumqttd.toml 配置项

- **listen**: MQTT 服务器监听地址和端口
  - 默认：`0.0.0.0:1883`
  - 修改此值需要同时更新 `DbOption.toml` 中的 `mqtt_host` 和 `mqtt_port`

- **max_connections**: 最大连接数
  - 默认：10010

- **max_payload_size**: 最大消息大小（字节）
  - 默认：20480 (20KB)

### 修改端口

如果需要修改 MQTT 端口：

1. 编辑 `rumqttd.toml`：
   ```toml
   [v4.1]
   listen = "0.0.0.0:1883"  # 修改为你想要的端口
   ```

2. 编辑 `DbOption.toml`：
   ```toml
   mqtt_host = "192.168.8.30"
   mqtt_port = 1883  # 修改为相同端口
   ```

3. 重启 web-server

## 文件查找优先级

web-server 会按以下顺序查找文件：

1. **可执行文件同目录**（`site-marine/bin/`）
   - `rumqttd.exe`
   - `rumqttd.toml`
   - ✅ **推荐**：便携式部署，所有文件在同一目录

2. **当前工作目录**
   - `./rumqttd.exe`
   - `./rumqttd.toml`

3. **开发环境路径**
   - `remote-test-dir/test-real/rumqttd.toml`

4. **系统 PATH**
   - `rumqttd.exe`（需要全局安装）

## 验证运行状态

### 通过 Web UI

访问"异地同步"页面，查看 MQTT 服务器状态。

### 通过 API

```bash
curl http://localhost:8080/api/sync/mqtt/status
```

### 通过 MQTT 客户端

使用 MQTT 客户端工具（如 MQTTX）连接到：
- 地址：`192.168.8.30`（或你的服务器 IP）
- 端口：`1883`

## 停止服务器

### 通过 Web UI

在"异地同步"页面点击"停止 MQTT 服务器"按钮。

### 通过 API

```bash
curl -X POST http://localhost:8080/api/sync/mqtt/stop
```

## 日志查看

MQTT 服务器日志可以通过 Web UI 的"MQTT Broker 日志"功能查看，或通过 API：

```bash
curl http://localhost:8080/api/mqtt/broker/logs
```

## 故障排查

### 问题：无法启动 MQTT 服务器

1. 检查 `rumqttd.exe` 是否存在：
   ```bash
   dir rumqttd.exe
   ```

2. 检查 `rumqttd.toml` 是否存在：
   ```bash
   dir rumqttd.toml
   ```

3. 检查端口是否被占用：
   ```bash
   netstat -ano | findstr :1883
   ```

4. 查看 web-server 日志中的错误信息

### 问题：配置文件找不到

确保 `rumqttd.toml` 在以下位置之一：
- `site-marine/bin/rumqttd.toml`（推荐）
- `./rumqttd.toml`（当前工作目录）
- `remote-test-dir/test-real/rumqttd.toml`（开发环境）

### 问题：可执行文件找不到

确保 `rumqttd.exe` 在以下位置之一：
- `site-marine/bin/rumqttd.exe`（推荐）
- 系统 PATH 中（需要全局安装）

## 相关文件

- `DbOption.toml` - 主配置文件，包含 MQTT 连接信息
- `src/web_server/sync_control_center.rs` - MQTT 服务器管理实现
- `src/web_server/sync_control_handlers.rs` - MQTT 服务器 API 处理器


