# MQTT Server 快速开始

## ⚡ 5 分钟上手指南

### 前提条件

✅ rumqttd 已安装:
```bash
rumqttd.exe --version  # 应显示: rumqttd 0.20.0
```

如果未安装:
```bash
cargo install rumqttd --version 0.20.0
```

---

## 🚀 方法 1: Web 界面控制（推荐）

### 步骤

1. **启动 Web 服务器**:
   ```bash
   cargo run --bin web_server --features web_server
   ```

2. **打开浏览器**: `http://localhost:8080`

3. **导航**: 找到 **MQTT 节点监控** 页面

4. **设置主节点**:
   - 如果显示 "从节点"，点击 👑 按钮
   - 确认对话框

5. **启动 Broker**:
   - 点击 **"启动 Broker"** 按钮
   - 等待状态变为 "Broker运行中" ✅

6. **验证**:
   ```bash
   netstat -an | findstr "1883"
   # 应该看到: TCP 0.0.0.0:1883 ... LISTENING
   ```

### 停止

- 在 Web 界面点击 **"停止 Broker"** 按钮

---

## 📜 方法 2: 批处理脚本

### Windows

```bash
start-mqtt-server.bat
```

按 `Ctrl+C` 停止

### PowerShell

```bash
.\scripts\test-real\start-mqtt-server.ps1
```

---

## 🔧 方法 3: 手动启动

```bash
cd remote-test-dir\test-real
rumqttd.exe -c rumqttd.toml -vv
```

按 `Ctrl+C` 停止

---

## ✅ 验证运行

### 检查端口

```bash
netstat -an | findstr "1883"
```

应该看到:
```
TCP    0.0.0.0:1883           0.0.0.0:0              LISTENING
```

### 检查进程

```bash
tasklist | findstr rumqttd
```

应该看到:
```
rumqttd.exe      12345 Console     1     25,000 K
```

---

## 🧪 测试连接

### 使用 mosquitto 客户端

```bash
# 订阅主题
mosquitto_sub -h localhost -p 1883 -t "test/#"

# 发布消息（在另一个终端）
mosquitto_pub -h localhost -p 1883 -t "test/topic" -m "Hello MQTT"
```

### 使用 MQTT Explorer

1. 下载: [mqtt-explorer.com](http://mqtt-explorer.com/)
2. 连接: `localhost:1883`
3. 订阅和发布消息

---

## 📍 配置位置

- **配置文件**: `remote-test-dir/test-real/rumqttd.toml`
- **MQTT 端口**: 1883
- **控制台端口**: 18083

---

## 🐛 常见问题

### 问题 1: "找不到 rumqttd"

**解决**:
```bash
cargo install rumqttd --version 0.20.0
```

### 问题 2: "配置文件不存在"

**解决**:
```bash
rumqttd.exe generate-config > remote-test-dir\test-real\rumqttd.toml
```

### 问题 3: 端口被占用

**解决**:
```bash
# 查找占用端口的进程
netstat -ano | findstr :1883

# 终止进程
taskkill //F //PID <PID>
```

### 问题 4: 进程立即退出

**检查配置文件**是否包含:
```toml
id = 0  # 必须在文件开头

[router]
id = 0  # 必须有 router 配置
```

---

## 📚 完整文档

- [MQTT_INTEGRATION_SUMMARY.md](MQTT_INTEGRATION_SUMMARY.md) - 完整集成总结
- [README_MQTT.md](README_MQTT.md) - 详细使用指南
- [MQTT_WEB_CONTROL.md](MQTT_WEB_CONTROL.md) - Web 界面控制文档

---

## 🎯 下一步

### 开发
1. ✅ 启动 MQTT Server
2. 启动你的应用程序
3. 配置应用连接到 `localhost:1883`

### 测试
1. 使用 mosquitto 客户端测试
2. 查看 Web 界面监控消息
3. 验证消息投递状态

### 部署
- 考虑使用系统服务（systemd）
- 配置自动重启
- 添加监控告警

---

**提示**: 首次使用推荐使用 **Web 界面控制**，方便查看状态和调试！
