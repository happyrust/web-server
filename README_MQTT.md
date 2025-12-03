# MQTT 服务器 (rumqttd) 使用指南

## 安装状态

✅ **rumqttd v0.20.0** 已安装到系统 PATH: `C:\Users\Administrator\.cargo\bin\rumqttd.exe`

## 快速启动

### 方法 1: 使用批处理脚本（推荐）

```bash
# Windows
start-mqtt-server.bat
```

### 方法 2: 手动启动

```bash
cd remote-test-dir/test-real
rumqttd.exe -c rumqttd.toml -vv
```

### 方法 3: 使用 PowerShell 脚本

```bash
.\scripts\test-real\start-mqtt-server.ps1
```

## 配置信息

- **配置文件**: `remote-test-dir/test-real/rumqttd.toml`
- **MQTT 端口**: 1883 (标准 MQTT v4 端口)
- **控制台端口**: 18083 (管理控制台)

## 测试连接

### 使用 mosquitto 客户端测试

```bash
# 订阅主题
mosquitto_sub -h localhost -p 1883 -t "test/#"

# 发布消息
mosquitto_pub -h localhost -p 1883 -t "test/topic" -m "Hello MQTT"
```

### 使用 MQTT Explorer GUI 工具

1. 下载 [MQTT Explorer](http://mqtt-explorer.com/)
2. 连接到 `localhost:1883`
3. 订阅和发布消息

## 与项目集成

在 `DbOption.toml` 中配置 MQTT 连接：

```toml
mqtt_host = "localhost"
mqtt_port = 1883
```

## 编译信息

### 已编译的二进制文件

- ✅ `rumqttd.exe` - MQTT 服务器（通过 `cargo install` 安装）
- ✅ `rumqttd-server/target/release/mqtt-server.exe` - 包装程序（可选）

### 重新编译（可选）

```bash
# 重新安装 rumqttd
cargo install rumqttd --version 0.20.0 --force

# 编译包装程序（可选）
cd rumqttd-server
cargo build --release --bin mqtt-server
```

## 配置文件说明

主要配置项（`rumqttd.toml`）：

```toml
id = 0  # 必需字段

[router]
id = 0
max_connections = 10010
max_outgoing_packet_count = 200
max_segment_size = 104857600
max_segment_count = 10

[v4.1]
name = "v4-1"
listen = "0.0.0.0:1883"
next_connection_delay_ms = 1

    [v4.1.connections]
    connection_timeout_ms = 60000
    max_payload_size = 20480
    max_inflight_count = 100
    dynamic_filters = true

[console]
listen = "0.0.0.0:18083"
```

## 监控和管理

### 查看实时日志

服务器启动时使用 `-vv` 参数可以查看详细调试日志：

```bash
rumqttd.exe -c rumqttd.toml -vv
```

日志级别：
- `-v` - info 级别
- `-vv` - debug 级别
- `-vvv` - trace 级别

### 检查端口占用

```bash
# Windows
netstat -an | findstr "1883"

# 查看进程
tasklist | findstr rumqttd
```

### 停止服务器

按 `Ctrl+C` 或：

```bash
# Windows
taskkill /F /IM rumqttd.exe
```

## 性能优化

根据实际需求调整配置：

- **增加最大连接数**: 修改 `router.max_connections`
- **调整消息缓冲**: 修改 `max_segment_size` 和 `max_segment_count`
- **优化内存使用**: 调整 `max_payload_size` 和 `max_inflight_count`

## 故障排查

### 端口被占用

```bash
# 查找占用端口的进程
netstat -ano | findstr :1883

# 结束进程（使用上面找到的 PID）
taskkill /F /PID <PID>
```

### 配置文件错误

如果配置文件有问题，可以生成默认配置：

```bash
rumqttd.exe generate-config > new-config.toml
```

### 连接失败

1. 检查防火墙设置
2. 确认端口 1883 未被占用
3. 查看服务器日志输出

## 相关文档

- [rumqttd GitHub](https://github.com/bytebeamio/rumqtt)
- [rumqttd 文档](https://docs.rs/rumqttd)
- [MQTT 协议规范](https://mqtt.org/)
- 项目相关文档:
  - [MQTT_MASTER_CLIENT_ROLE.md](MQTT_MASTER_CLIENT_ROLE.md)
  - [MQTT_MONITORING_SUMMARY.md](MQTT_MONITORING_SUMMARY.md)
  - [MQTT_SUBSCRIPTION_CONTROL.md](MQTT_SUBSCRIPTION_CONTROL.md)

## 常用命令速查

```bash
# 查看版本
rumqttd.exe --version

# 查看帮助
rumqttd.exe --help

# 生成默认配置
rumqttd.exe generate-config

# 启动服务器（info 日志）
rumqttd.exe -c rumqttd.toml -v

# 启动服务器（debug 日志）
rumqttd.exe -c rumqttd.toml -vv

# 静默启动（无 banner）
rumqttd.exe -c rumqttd.toml -q
```
