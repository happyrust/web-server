# 异地协同测试环境配置指南

## 概述

本文档介绍如何在单机环境下配置并运行两个独立的测试站点（站点 1112 和站点 7000），用于模拟异地协同场景。

## 目录结构

```
remote-test-dir/test-real/
├── rumqttd.toml              # MQTT 服务器配置
├── site-1112/                # 站点 1112 的完整运行环境
│   ├── DbOption.toml         # 站点 1112 的配置文件
│   ├── deployment_sites.sqlite  # SQLite 数据库（记录远程站点信息）
│   ├── assets/               # 资源目录
│   │   └── archives/         # 压缩包存储
│   ├── output/               # 输出目录
│   │   └── remote_sync/      # 同步文件存储
│   └── surrealdb-data/       # SurrealDB 数据目录
└── site-7000/                # 站点 7000 的完整运行环境
    ├── DbOption.toml         # 站点 7000 的配置文件
    ├── deployment_sites.sqlite  # SQLite 数据库
    ├── assets/
    │   └── archives/
    ├── output/
    │   └── remote_sync/
    └── surrealdb-data/
```

## 前置要求

### 必需软件

1. **Rust 工具链**
   - 安装：https://rustup.rs/
   - 用于编译和运行项目

2. **SurrealDB**
   - 安装（Windows）：`iwr https://windows.surrealdb.com -useb | iex`
   - 安装（Cargo）：`cargo install --locked surrealdb`
   - 官网：https://surrealdb.com/install

3. **SQLite**
   - Windows 用户需要下载 sqlite3.exe
   - 官网：https://www.sqlite.org/download.html
   - 或使用包管理器：`choco install sqlite`

4. **MQTT 服务器（rumqttd）**
   - 方式 1：使用项目中的 rumqtt submodule
   - 方式 2：使用 rumqttd-server
   - 方式 3：`cargo install rumqttd`

### AVEVA 项目要求

- AVEVA E3D 2.1 安装在 `D:\AVEVA\Projects\E3D2.1`
- 包含 `AvevaMarineSample` 项目
- 包含数据库文件：
  - `AvevaMarineSample/ams000/ams1112_0001/`（数据库 1112）
  - `AvevaMarineSample/ams000/ams7000_0001/`（数据库 7000）

## 端口分配

| 服务 | 站点 1112 | 站点 7000 | 共享 |
|------|----------|----------|------|
| Web Server (HTTP) | 8081 | 8082 | - |
| SurrealDB | 8021 | 8022 | - |
| MQTT Broker | - | - | 1883 |
| MQTT Console | - | - | 18083 |

## 快速启动

### 1. 初始化环境

运行初始化脚本，创建目录结构和 SQLite 数据库：

```powershell
.\scripts\test-real\init_test_env.ps1
```

使用 `-Force` 参数可以强制重新创建数据库（会删除现有数据）：

```powershell
.\scripts\test-real\init_test_env.ps1 -Force
```

### 2. 启动服务（推荐顺序）

打开 **6 个独立的终端窗口**，分别运行以下命令：

#### 终端 1: MQTT 服务器

```powershell
.\scripts\test-real\start-mqtt-server.ps1
```

等待看到 "rumqttd started" 或类似消息。

#### 终端 2: SurrealDB（站点 1112）

```powershell
.\scripts\test-real\start-surreal-1112.ps1
```

等待看到 "Started web server on 127.0.0.1:8021"。

#### 终端 3: SurrealDB（站点 7000）

```powershell
.\scripts\test-real\start-surreal-7000.ps1
```

等待看到 "Started web server on 127.0.0.1:8022"。

#### 终端 4: 初始化数据库

```powershell
# 初始化所有站点
.\scripts\test-real\init-database.ps1

# 或只初始化特定站点
.\scripts\test-real\init-database.ps1 -Site 1112
.\scripts\test-real\init-database.ps1 -Site 7000
```

此步骤会解析 AVEVA 数据库文件并导入到 SurrealDB。

#### 终端 5: Web 服务器（站点 1112）

```powershell
.\scripts\test-real\start-site-1112.ps1
```

等待看到服务器启动消息。

#### 终端 6: Web 服务器（站点 7000）

```powershell
.\scripts\test-real\start-site-7000.ps1
```

等待看到服务器启动消息。

## 配置详解

### 站点 1112 配置 (DbOption.toml)

```toml
# 项目路径（必须存在）
project_path = "D:/AVEVA/Projects/E3D2.1"
included_projects = ["AvevaMarineSample"]
project_name = "AvevaMarineSample"
project_code = '1516'

# 站点标识
location = "SITE1112"
location_dbs = [1112]

# MQTT 配置（共享服务器）
mqtt_host = "127.0.0.1"
mqtt_port = 1883

# 文件服务器（本站点）
file_server_host = "http://127.0.0.1:8081/assets/archives"

# SQLite 数据库（相对路径）
deployment_sites_sqlite_path = "deployment_sites.sqlite"

# SurrealDB 配置
v_ip = "localhost"
v_port = 8021
v_user = "root"
v_password = "root"
```

### 站点 7000 配置 (DbOption.toml)

配置与站点 1112 类似，主要差异：

```toml
location = "SITE7000"
location_dbs = [7000]
file_server_host = "http://127.0.0.1:8082/assets/archives"
v_port = 8022
```

### SQLite 数据库内容

#### 站点 1112 的 deployment_sites.sqlite

```sql
-- 环境记录（本站点）
INSERT INTO deployment_environments (name, location, location_dbs)
VALUES ('测试环境-1112', 'SITE1112', '1112');

-- 远程站点记录（对端站点 7000）
INSERT INTO deployment_sites (environment_id, name, location, http_host, dbnums, enabled)
VALUES (1, '站点-7000', 'SITE7000', 'http://127.0.0.1:8082/files', '7000', 1);
```

#### 站点 7000 的 deployment_sites.sqlite

```sql
-- 环境记录（本站点）
INSERT INTO deployment_environments (name, location, location_dbs)
VALUES ('测试环境-7000', 'SITE7000', '7000');

-- 远程站点记录（对端站点 1112）
INSERT INTO deployment_sites (environment_id, name, location, http_host, dbnums, enabled)
VALUES (1, '站点-1112', 'SITE1112', 'http://127.0.0.1:8081/files', '1112', 1);
```

## 测试流程

### 1. 验证服务状态

检查所有服务是否正常运行：

```powershell
# 检查 MQTT（应该看到连接信息）
# 查看 MQTT 服务器终端输出

# 检查 SurrealDB
Invoke-WebRequest -Uri http://127.0.0.1:8021/health
Invoke-WebRequest -Uri http://127.0.0.1:8022/health

# 检查 Web 服务器
Invoke-WebRequest -Uri http://127.0.0.1:8081/health
Invoke-WebRequest -Uri http://127.0.0.1:8082/health
```

### 2. 模拟文件更新

#### 方式 1: 修改 AVEVA 数据库文件

1. 修改 `D:\AVEVA\Projects\E3D2.1\AvevaMarineSample\ams000\ams1112_0001\` 下的文件
2. 站点 1112 检测到变化后会：
   - 生成增量压缩包到 `site-1112/assets/archives/`
   - 通过 HTTP PUT 发送到 `http://127.0.0.1:8082/files/.../UPLOAD/...`
3. 站点 7000 接收并存储到 `site-7000/output/remote_sync/`

#### 方式 2: 手动触发同步

使用 Web API 手动触发（根据实际 API 实现）：

```powershell
# 示例：触发站点 1112 的同步
Invoke-WebRequest -Uri http://127.0.0.1:8081/api/sync/trigger -Method POST
```

### 3. 查看日志

观察终端输出，查找：
- 文件变化检测日志
- 增量生成日志
- HTTP 传输日志
- MQTT 事件日志

### 4. 验证同步结果

检查文件是否已同步：

```powershell
# 检查站点 7000 的接收目录
Get-ChildItem -Path "remote-test-dir\test-real\site-7000\output\remote_sync\" -Recurse

# 检查站点 1112 的发送记录
Get-ChildItem -Path "remote-test-dir\test-real\site-1112\assets\archives\" -Recurse
```

## 故障排查

### 问题 1: 端口已被占用

**症状**：启动服务时报错 "Address already in use"

**解决**：

```powershell
# 查看占用端口的进程
netstat -ano | findstr "8081"
netstat -ano | findstr "8082"
netstat -ano | findstr "1883"
netstat -ano | findstr "8021"
netstat -ano | findstr "8022"

# 终止进程（替换 PID）
taskkill /PID <进程ID> /F
```

### 问题 2: SQLite 数据库初始化失败

**症状**：init_test_env.ps1 执行失败，找不到 sqlite3 命令

**解决**：

1. 下载 SQLite 工具：https://www.sqlite.org/download.html
2. 解压并添加到 PATH
3. 或使用 Chocolatey：`choco install sqlite`

### 问题 3: AVEVA 项目路径不存在

**症状**：启动 Web 服务器时报错，找不到项目文件

**解决**：

1. 确认 AVEVA 安装路径
2. 修改 DbOption.toml 中的 `project_path`
3. 或创建软链接：
   ```powershell
   New-Item -ItemType SymbolicLink -Path "D:\AVEVA\Projects\E3D2.1" -Target "实际路径"
   ```

### 问题 4: SurrealDB 连接失败

**症状**：Web 服务器启动时无法连接到 SurrealDB

**解决**：

1. 确认 SurrealDB 已启动并监听正确端口
2. 检查防火墙设置
3. 查看 SurrealDB 日志输出

### 问题 5: MQTT 连接失败

**症状**：无法连接到 MQTT broker

**解决**：

1. 确认 rumqttd 已启动
2. 检查 rumqttd.toml 配置
3. 使用 MQTT 客户端工具测试（如 MQTT Explorer）

### 问题 6: 文件同步不触发

**症状**：修改文件后没有触发同步

**解决**：

1. 检查文件监控是否启用（查看配置和日志）
2. 确认修改的文件属于配置的 location_dbs
3. 检查 MQTT 连接状态
4. 查看 Web 服务器日志是否有错误

## 清理环境

### 停止所有服务

在每个终端窗口按 `Ctrl+C` 停止服务。

### 清理数据

```powershell
# 删除 SurrealDB 数据
Remove-Item -Recurse -Force "remote-test-dir\test-real\site-1112\surrealdb-data"
Remove-Item -Recurse -Force "remote-test-dir\test-real\site-7000\surrealdb-data"

# 删除 SQLite 数据库
Remove-Item -Force "remote-test-dir\test-real\site-1112\deployment_sites.sqlite"
Remove-Item -Force "remote-test-dir\test-real\site-7000\deployment_sites.sqlite"

# 删除同步文件
Remove-Item -Recurse -Force "remote-test-dir\test-real\site-1112\output\remote_sync\*"
Remove-Item -Recurse -Force "remote-test-dir\test-real\site-7000\output\remote_sync\*"
Remove-Item -Recurse -Force "remote-test-dir\test-real\site-1112\assets\archives\*"
Remove-Item -Recurse -Force "remote-test-dir\test-real\site-7000\assets\archives\*"
```

### 重新初始化

```powershell
.\scripts\test-real\init_test_env.ps1 -Force
```

## 高级配置

### 修改数据库编号

如果需要使用不同的数据库编号：

1. 修改 DbOption.toml 中的 `location_dbs`
2. 修改 SQLite 数据库中的记录
3. 确保 AVEVA 项目中有对应的数据库文件

### 添加更多站点

1. 复制 site-1112 或 site-7000 目录
2. 修改新站点的 DbOption.toml
3. 分配新的端口号
4. 创建启动脚本
5. 在其他站点的 SQLite 数据库中添加新站点记录

### 自定义 MQTT 配置

编辑 `remote-test-dir/test-real/rumqttd.toml`：

```toml
[v4.1]
name = "v4-1"
listen = "0.0.0.0:1883"
next_connection_delay_ms = 1

    [v4.1.connections]
    connection_timeout_ms = 60000
    max_payload_size = 20480        # 调整最大消息大小
    max_inflight_count = 100        # 调整并发消息数
    max_inflight_size = 1024        # 调整并发消息总大小
```

## 参考资料

- [异地协同测试环境计划](.cursor/plans/配置异地协同测试环境（单机多节点模拟）.plan.md)
- [远程同步 API 文档](REMOTE_SYNC_API_COMPLETE.md)
- [远程同步开发指南](REMOTE_SYNC_DEVELOPMENT_GUIDE.md)
- [远程同步测试策略](REMOTE_SYNC_TEST_STRATEGY.md)
- [SurrealDB 文档](https://surrealdb.com/docs)
- [rumqttd 文档](https://github.com/bytebeamio/rumqtt)

## 常见问题 (FAQ)

### Q: 是否需要真实的异地网络环境？

A: 不需要。本测试环境在单机上模拟多站点，使用 localhost 和不同端口。

### Q: 可以在生产环境使用这套配置吗？

A: 不可以。这是测试配置，生产环境需要：
- 真实的网络地址和域名
- SSL/TLS 加密
- 认证和授权机制
- 负载均衡和高可用配置

### Q: 如何调试同步问题？

A:
1. 启用详细日志（修改 DbOption.toml 中的 `enable_log = true`）
2. 使用 MQTT 客户端监控消息
3. 检查 HTTP 请求日志
4. 查看数据库变更记录

### Q: 支持 Windows 以外的操作系统吗？

A: 支持，但需要：
- 修改脚本为 Shell 脚本（.sh）
- 调整路径格式（使用 `/` 而不是 `\`）
- 确保有对应平台的 AVEVA E3D 安装

## 联系支持

如有问题，请：
1. 查看日志文件
2. 检查本文档的故障排查章节
3. 提交 Issue 到项目仓库
