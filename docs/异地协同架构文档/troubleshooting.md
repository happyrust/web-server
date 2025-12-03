# 异地协同故障排查指南

## 目录

1. [常见问题速查表](#常见问题速查表)
2. [MQTT 连接问题](#mqtt-连接问题)
3. [文件同步问题](#文件同步问题)
4. [性能问题](#性能问题)
5. [数据一致性问题](#数据一致性问题)
6. [日志分析](#日志分析)

---

## 常见问题速查表

| 症状 | 可能原因 | 解决方案 |
|------|---------|---------|
| MQTT 连接循环失败 | Broker 未启动 | 先启动 MQTT Broker |
| 事件处理两次 | 多个 watcher 实例 | 检查是否重复调用 start_runtime |
| CBA 下载失败 | 网络或地址错误 | 检查 file_server_host 配置 |
| 文件变化未同步 | location_dbs 过滤 | 检查数据库编号是否在列表中 |
| 从节点看不到主节点 | 配置错误 | 检查 master_mqtt_host/port |

---

## MQTT 连接问题

### 问题 1: 连接循环失败

**症状:**
```
[Publisher] Connected to MQTT broker.
[Publisher] MQTT Connection error: Connection closed by peer abruptly
[Publisher] Connected to MQTT broker.
[Publisher] MQTT Connection error: Connection closed by peer abruptly
... (每秒重复)
```

**原因:**
- MQTT Broker 未启动
- Client ID 冲突 (多个客户端使用相同 ID)
- 网络防火墙阻断

**解决方案:**

```
┌─────────────────────────────────────────────────────────────────┐
│                    MQTT 连接故障排查流程                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐                                           │
│  │ MQTT 连接失败    │                                           │
│  └────────┬────────┘                                           │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────┐    否    ┌─────────────────┐              │
│  │ 是否为主节点?    │────────►│ 检查主节点地址   │              │
│  └────────┬────────┘         │ master_mqtt_host │              │
│           │ 是               │ 是否正确         │              │
│           ▼                   └─────────────────┘              │
│  ┌─────────────────┐                                           │
│  │ Broker 是否启动? │                                           │
│  └────────┬────────┘                                           │
│           │                                                     │
│     ┌─────┴─────┐                                              │
│     │           │                                              │
│     ▼           ▼                                              │
│  ┌─────┐    ┌─────────────────┐                                │
│  │ 是  │    │ 否 → 点击       │                                │
│  └──┬──┘    │ "启动 Broker"   │                                │
│     │       └─────────────────┘                                │
│     ▼                                                           │
│  ┌─────────────────┐                                           │
│  │ 检查端口 1883    │                                           │
│  │ netstat -an |   │                                           │
│  │ findstr 1883    │                                           │
│  └────────┬────────┘                                           │
│           │                                                     │
│     ┌─────┴─────┐                                              │
│     │           │                                              │
│     ▼           ▼                                              │
│  ┌─────┐    ┌─────────────────┐                                │
│  │监听中│    │ 未监听 → 检查   │                                │
│  └──┬──┘    │ rumqttd 进程    │                                │
│     │       └─────────────────┘                                │
│     ▼                                                           │
│  ┌─────────────────┐                                           │
│  │ 检查防火墙规则   │                                           │
│  │ 是否放行 1883   │                                           │
│  └─────────────────┘                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**检查命令:**
```bash
# 检查 MQTT Broker 是否监听
netstat -an | findstr 1883

# 检查 rumqttd 进程
tasklist | findstr rumqttd

# 测试 MQTT 连接 (需要安装 mosquitto 客户端)
mosquitto_sub -h localhost -p 1883 -t "test" -v
```

### 问题 2: 从节点无法连接主节点

**症状:**
```
从节点模式：连接到主节点MQTT服务器 192.168.1.100:1883
[Subscriber] Connection error: ...
```

**检查清单:**
1. 主节点 IP 地址是否正确
2. 主节点防火墙是否开放 1883 端口
3. 主节点 MQTT Broker 是否已启动
4. 网络是否可达 (ping 测试)

**解决方案:**
```bash
# 在从节点测试网络连通性
ping 192.168.1.100

# 测试端口连通性 (PowerShell)
Test-NetConnection -ComputerName 192.168.1.100 -Port 1883

# 检查从节点配置
grep -E "master_mqtt" DbOption.toml
```

---

## 文件同步问题

### 问题 3: 文件变化未触发同步

**症状:**
- 修改 PDMS 文件后，日志没有输出
- 其他节点没有收到同步消息

**排查流程:**

```
┌─────────────────────────────────────────────────────────────────┐
│                    文件同步故障排查流程                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐                                           │
│  │ 文件变化未同步   │                                           │
│  └────────┬────────┘                                           │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────┐    否    ┌─────────────────┐              │
│  │ sync_live=true? │────────►│ 修改配置并重启   │              │
│  └────────┬────────┘         └─────────────────┘              │
│           │ 是                                                  │
│           ▼                                                     │
│  ┌─────────────────┐    否    ┌─────────────────┐              │
│  │ 日志有 "changed"│────────►│ 检查文件路径是否 │              │
│  │ 输出?           │         │ 在监听范围内     │              │
│  └────────┬────────┘         └─────────────────┘              │
│           │ 是                                                  │
│           ▼                                                     │
│  ┌─────────────────┐    否    ┌─────────────────┐              │
│  │ dbnum 在        │────────►│ 将 dbnum 添加到  │              │
│  │ location_dbs 中?│         │ location_dbs 列表│              │
│  └────────┬────────┘         └─────────────────┘              │
│           │ 是                                                  │
│           ▼                                                     │
│  ┌─────────────────┐    否    ┌─────────────────┐              │
│  │ db_type 在      │────────►│ 添加到           │              │
│  │ sync_push_db_   │         │ sync_push_db_    │              │
│  │ types 中?       │         │ types 列表       │              │
│  └────────┬────────┘         └─────────────────┘              │
│           │ 是                                                  │
│           ▼                                                     │
│  ┌─────────────────┐    否    ┌─────────────────┐              │
│  │ MQTT Publisher  │────────►│ 点击"启动订阅"   │              │
│  │ 已初始化?       │         │ 按钮             │              │
│  └────────┬────────┘         └─────────────────┘              │
│           │ 是                                                  │
│           ▼                                                     │
│  ┌─────────────────┐                                           │
│  │ 检查 MQTT 连接   │                                           │
│  │ 状态和网络      │                                           │
│  └─────────────────┘                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**配置检查:**
```toml
# DbOption.toml 关键配置
sync_live = true                    # 必须为 true
location_dbs = [1112, 1113]         # 必须包含目标数据库编号
sync_push_db_types = ["DESI"]       # 必须包含数据库类型
```

### 问题 4: CBA 文件下载失败

**症状:**
```
开始下载 CBA: http://192.168.1.100:8080/assets/archives/d001112_1213.cba
下载失败: Connection refused
```

**检查清单:**
1. `file_server_host` 配置是否正确
2. 主节点 Web Server 是否运行
3. CBA 文件是否存在于 `assets/archives/` 目录
4. HTTP 端口是否可访问

**解决方案:**
```bash
# 检查主节点 CBA 文件
ls assets/archives/

# 测试 HTTP 访问
curl -I http://192.168.1.100:8080/assets/archives/d001112_1213.cba

# 检查从节点配置
grep "file_server_host" DbOption.toml
```

### 问题 5: 事件被处理两次

**症状:**
```
统计每个会话的增删改数量...
统计每个会话的增删改数量...
保存到SurrealDB完成
保存到SurrealDB完成
```

**原因:**
两个 `async_watch()` 实例同时运行，通常是因为：
1. `web_server/mod.rs` 启动了一个 watcher
2. `start_runtime()` 又启动了另一个 watcher

**解决方案:**
确保 `remote_runtime.rs` 中的 `start_runtime()` 不再启动 `async_watch()`。
这个问题在最新版本中已修复。

---

## 性能问题

### 问题 6: 大文件同步缓慢

**症状:**
- CBA 文件生成很慢
- 下载大文件时超时

**优化建议:**
1. 增加网络带宽
2. 启用压缩传输
3. 分批同步大型数据库

### 问题 7: 内存占用过高

**症状:**
- 长时间运行后内存持续增长

**原因:**
- 消息监控表未清理
- 临时文件堆积

**解决方案:**
```bash
# 清理临时文件
rm -rf assets/temp/*

# 重启服务释放内存
taskkill /F /IM web_server.exe
cargo run --bin web_server --features web_server
```

---

## 数据一致性问题

### 问题 8: 哈希校验失败

**症状:**
```
SHA256 校验失败: expected abc123..., got def456...
跳过克隆
```

**原因:**
- 传输过程中文件损坏
- 发送端和接收端文件不一致

**解决方案:**
1. 重新触发同步 (修改文件再保存)
2. 检查网络稳定性
3. 手动复制 CBA 文件

### 问题 9: 数据库版本冲突

**症状:**
- 同一数据库在多个节点被修改
- 同步后数据不一致

**预防措施:**
- 使用 `location_dbs` 确保每个数据库只由一个节点负责
- 避免在多个节点同时修改同一数据库

---

## 日志分析

### 关键日志模式

**正常启动:**
```
🔍 启动增量监测后台任务...
ℹ️  MQTT 功能由前端控制，请通过 MQTT 监控页面启动订阅
📋 正在初始化数据库文件监听器...
✅ 监听器初始化完成
📡 增量监测任务已启动，正在监听文件变化...
```

**MQTT 启动成功:**
```
启动 MQTT 发布器 EventLoop...
[Publisher] Connected to MQTT broker.
[Publisher] MQTT client stored in global variable
[Subscriber] Connected to MQTT broker
```

**文件变化检测:**
```
changed: Event { kind: Modify(Data(Any)), paths: ["D:\\...\\d001112"] }
开始扫描数据库头部信息，路径: ["D:\\...\\d001112"]
成功扫描到 1 个数据库头部
正在处理路径: "D:\\...\\d001112"
检测到增量更新: 1212 -> 1213
```

**同步成功:**
```
✅ CBA 文件已生成: assets/archives/d001112_1213.cba
✅ MQTT 消息已发布到 Sync/E3d
```

### 错误日志分析

| 日志模式 | 含义 | 解决方案 |
|---------|------|---------|
| `Connection closed by peer` | MQTT 连接被断开 | 检查 Broker 状态 |
| `MQTT Publisher 未初始化` | Publisher 未启动 | 点击"启动订阅" |
| `location_dbs 过滤` | 数据库不在推送列表 | 检查配置 |
| `哈希校验失败` | 文件损坏 | 重新同步 |
| `下载失败` | HTTP 连接问题 | 检查网络和地址 |

### 日志级别调整

```bash
# 启用详细日志
RUST_LOG=debug cargo run --bin web_server --features web_server

# 只看 MQTT 相关日志
RUST_LOG=rumqttc=debug cargo run --bin web_server --features web_server
```

---

## 诊断工具

### 系统状态检查脚本

```powershell
# check_sync_status.ps1
Write-Host "=== 异地协同状态检查 ===" -ForegroundColor Green

# 检查 Web Server
$webServer = Get-Process web_server -ErrorAction SilentlyContinue
if ($webServer) {
    Write-Host "[OK] Web Server 运行中 (PID: $($webServer.Id))" -ForegroundColor Green
} else {
    Write-Host "[FAIL] Web Server 未运行" -ForegroundColor Red
}

# 检查 MQTT Broker
$mqtt = Test-NetConnection -ComputerName localhost -Port 1883 -WarningAction SilentlyContinue
if ($mqtt.TcpTestSucceeded) {
    Write-Host "[OK] MQTT Broker 监听中 (端口 1883)" -ForegroundColor Green
} else {
    Write-Host "[FAIL] MQTT Broker 未监听" -ForegroundColor Red
}

# 检查 HTTP 服务
try {
    $http = Invoke-WebRequest -Uri "http://localhost:8080/api/mqtt/subscription/status" -UseBasicParsing
    Write-Host "[OK] HTTP API 可访问" -ForegroundColor Green
} catch {
    Write-Host "[FAIL] HTTP API 不可访问" -ForegroundColor Red
}

# 检查 CBA 目录
$cbaCount = (Get-ChildItem assets/archives/*.cba -ErrorAction SilentlyContinue).Count
Write-Host "[INFO] CBA 文件数量: $cbaCount" -ForegroundColor Cyan
```

### API 状态检查

```bash
# 检查所有关键 API
curl -s http://localhost:8080/api/mqtt/subscription/status | jq .
curl -s http://localhost:8080/api/mqtt/nodes | jq .
curl -s http://localhost:8080/api/sync/mqtt/status | jq .
```
