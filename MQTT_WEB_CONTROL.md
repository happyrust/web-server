# MQTT Server Web 界面控制功能

## 功能概述

前端 Vue 界面已集成 MQTT Server 启动/停止控制功能，后端已实现通过 Web API 控制 `rumqttd` 进程的启动和停止。

## ✅ 已实现的功能

### 1. 前端界面 (Vue)

**位置**: [frontend/src/components/views/MqttNodeMonitor.vue](frontend/src/components/views/MqttNodeMonitor.vue)

**功能**:
- ✅ 显示 MQTT Server 运行状态（仅主节点可见）
- ✅ 启动 MQTT Broker 按钮
- ✅ 停止 MQTT Broker 按钮
- ✅ 实时状态指示器（运行中/未启动）
- ✅ 节点角色切换（主节点/从节点）

**界面元素**:
```vue
<!-- MQTT Server 状态（仅主节点） -->
<div v-if="mqttStatus.is_master_node">
  <div class="status-indicator">
    {{ mqttStatus.is_server_running ? 'Broker运行中' : 'Broker未启动' }}
  </div>
  <button @click="startMqttServer">启动Broker</button>
  <button @click="stopMqttServer">停止Broker</button>
</div>
```

### 2. 后端 API

**API 路由**:
- `POST /api/sync/mqtt/start` - 启动 MQTT 服务器
- `POST /api/sync/mqtt/stop` - 停止 MQTT 服务器
- `GET /api/sync/mqtt/status` - 查询 MQTT 服务器状态

**实现位置**:
- 路由注册: [src/web_server/mod.rs:459-467](src/web_server/mod.rs#L459-L467)
- Handler: [src/web_server/sync_control_handlers.rs:432-465](src/web_server/sync_control_handlers.rs#L432-L465)
- 核心逻辑: [src/web_server/sync_control_center.rs:1237-1353](src/web_server/sync_control_center.rs#L1237-L1353)

### 3. MQTT Server 进程管理

**全局状态管理**:
```rust
/// MQTT 服务器进程句柄（用于启动/停止控制）
pub static MQTT_SERVER_PROCESS: Lazy<Arc<RwLock<Option<tokio::process::Child>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));
```

**启动流程**:
1. 检查 `rumqttd` 是否已在运行
2. 定位配置文件 `remote-test-dir/test-real/rumqttd.toml`
3. 使用 `tokio::process::Command` 启动 `rumqttd.exe`
4. 验证进程启动成功（等待 500ms 后检查）
5. 保存进程句柄到全局变量
6. 更新 `SyncControlCenter` 状态

**停止流程**:
1. 从全局变量获取进程句柄
2. 调用 `child.kill()` 终止进程
3. 等待进程完全退出
4. 清理全局状态和 `SyncControlCenter` 状态

## 使用方法

### 1. 启动 Web 服务器

```bash
cargo run --bin web_server --features web_server
```

### 2. 访问 MQTT 监控界面

1. 在浏览器中打开: `http://localhost:8080`
2. 导航到 **MQTT 节点监控** 页面
3. 如果当前不是主节点，点击 "设为主节点" 按钮
4. 主节点界面会显示 "MQTT Server 状态" 区域

### 3. 启动 MQTT Server

点击 **"启动 Broker"** 按钮：

**前端请求**:
```javascript
async function startMqttServer() {
  const res = await fetch('/api/sync/mqtt/start', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ port: 1883 })
  });
  const data = await res.json();
  // 显示结果: data.status, data.message
}
```

**后端响应**:
```json
{
  "status": "success",
  "message": "MQTT服务器已启动在端口 1883",
  "port": 1883
}
```

**日志输出**:
```
✅ MQTT 服务器已启动在端口 1883
```

### 4. 停止 MQTT Server

点击 **"停止 Broker"** 按钮：

**前端请求**:
```javascript
async function stopMqttServer() {
  if (!confirm('确定要停止 MQTT Broker 吗？')) return;

  const res = await fetch('/api/sync/mqtt/stop', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' }
  });
  const data = await res.json();
}
```

**后端响应**:
```json
{
  "status": "success",
  "message": "MQTT服务器已停止"
}
```

**日志输出**:
```
✅ MQTT 服务器进程已终止
✅ MQTT 服务器已停止
```

## 配置要求

### 1. 安装 rumqttd

```bash
cargo install rumqttd --version 0.20.0
```

### 2. 配置文件位置

MQTT 配置文件必须存在于：
```
<项目根目录>/remote-test-dir/test-real/rumqttd.toml
```

### 3. 配置文件内容

参考 [remote-test-dir/test-real/rumqttd.toml](remote-test-dir/test-real/rumqttd.toml):

```toml
id = 0

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

## 错误处理

### 常见错误及解决方案

#### 1. "无法启动 rumqttd，请确保已安装"

**原因**: 系统 PATH 中找不到 `rumqttd.exe`

**解决**:
```bash
# 安装 rumqttd
cargo install rumqttd --version 0.20.0

# 验证安装
rumqttd.exe --version
```

#### 2. "MQTT 配置文件不存在"

**原因**: 配置文件路径错误或文件缺失

**解决**:
```bash
# 检查文件是否存在
ls remote-test-dir/test-real/rumqttd.toml

# 如果不存在，生成默认配置
rumqttd.exe generate-config > remote-test-dir/test-real/rumqttd.toml
```

#### 3. "MQTT 服务器已经在运行中"

**原因**: 进程句柄未清理或外部启动了 rumqttd

**解决**:
```bash
# Windows - 查找并终止进程
tasklist | findstr rumqttd
taskkill /F /IM rumqttd.exe

# Linux - 查找并终止进程
ps aux | grep rumqttd
kill -9 <PID>
```

#### 4. "MQTT 服务器启动失败，退出状态: 101"

**原因**: 配置文件格式错误（如缺少 `id = 0` 字段）

**解决**: 检查配置文件是否包含必需字段：
```toml
id = 0  # 必须在文件开头

[router]
id = 0  # 必须有 router 配置
```

## 测试步骤

### 完整测试流程

1. **启动 Web 服务器**:
   ```bash
   cargo run --bin web_server --features web_server
   ```

2. **打开浏览器**: `http://localhost:8080`

3. **导航到 MQTT 监控页面**

4. **设置为主节点**:
   - 如果显示 "从节点"，点击 👑 按钮设为主节点
   - 确认对话框，等待角色切换完成

5. **启动 MQTT Broker**:
   - 点击 "启动 Broker" 按钮
   - 等待 500ms，应显示 "Broker运行中"
   - 检查控制台日志: `✅ MQTT 服务器已启动在端口 1883`

6. **验证 MQTT 服务器**:
   ```bash
   # 检查端口监听
   netstat -an | findstr "1883"

   # 应该看到类似输出：
   # TCP    0.0.0.0:1883           0.0.0.0:0              LISTENING
   ```

7. **测试 MQTT 连接** (可选):
   ```bash
   # 使用 mosquitto 客户端测试
   mosquitto_sub -h localhost -p 1883 -t "test/#"
   ```

8. **停止 MQTT Broker**:
   - 点击 "停止 Broker" 按钮
   - 确认对话框
   - 应显示 "Broker未启动"
   - 检查控制台日志: `✅ MQTT 服务器已停止`

9. **验证进程已终止**:
   ```bash
   tasklist | findstr rumqttd
   # 应该没有输出
   ```

## 技术架构

### 前后端通信流程

```
前端 Vue 组件 (MqttNodeMonitor.vue)
    ↓ HTTP POST
API Handler (sync_control_handlers.rs)
    ↓ 调用
核心逻辑 (sync_control_center.rs)
    ↓ 启动
tokio::process::Command
    ↓ 执行
rumqttd.exe 进程
    ↓ 监听
0.0.0.0:1883 (MQTT 端口)
```

### 状态管理

```rust
// 全局进程句柄
MQTT_SERVER_PROCESS: RwLock<Option<Child>>

// 同步控制中心状态
SyncControlCenter {
    mqtt_server: Option<MqttServerState> {
        is_running: bool,
        port: u16,
        started_at: Option<SystemTime>,
        ...
    }
}
```

## 扩展功能（待实现）

### 可能的改进

1. **进程输出日志**:
   - 捕获 `rumqttd` 的 stdout/stderr
   - 在 Web 界面实时显示日志

2. **健康检查**:
   - 定期检查 MQTT 服务器是否响应
   - 自动重启失败的服务器

3. **多实例管理**:
   - 支持启动多个 MQTT broker（不同端口）
   - 动态配置生成

4. **配置编辑**:
   - 在 Web 界面直接编辑 `rumqttd.toml`
   - 实时生效配置更改

## 相关文档

- [README_MQTT.md](README_MQTT.md) - MQTT Server 使用指南
- [MQTT_MASTER_CLIENT_ROLE.md](MQTT_MASTER_CLIENT_ROLE.md) - MQTT 主从节点角色
- [MQTT_NODE_MONITORING.md](frontend/MQTT_NODE_MONITORING.md) - 节点监控功能文档

## 总结

✅ **功能已完整实现**:
- 前端 Vue 界面完成
- 后端 API 完成
- MQTT 进程管理完成
- 启动/停止逻辑完成

🎯 **核心改进**:
- 将之前的 TODO 实现替换为真正的进程启动/停止逻辑
- 使用 `tokio::process::Command` 管理 `rumqttd` 子进程
- 全局状态管理确保进程生命周期可控

📝 **使用建议**:
- 确保 `rumqttd` 已正确安装
- 检查配置文件格式正确
- 在生产环境考虑使用系统服务（systemd）而非 Web 控制
