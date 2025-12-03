# 端口和 Host 配置说明

## 修改启动端口的方法

Web 服务器支持三种方式配置端口，优先级从高到低：

### 1. 命令行参数（推荐）⭐

使用 `--port` 或 `-p` 参数指定端口：

```bash
# Windows
web_server.exe --port 18080
web_server.exe -p 18080

# Linux
./web_server --port 18080
./web_server -p 18080
```

**示例**：
```bash
# 启动在端口 18080
web_server.exe --config DbOption_marine --port 18080

# 启动在端口 9000
web_server.exe -p 9000
```

### 2. 环境变量

通过设置 `PORT` 环境变量：

**Windows PowerShell**：
```powershell
$env:PORT = 18080
web_server.exe
```

**Windows CMD**：
```cmd
set PORT=18080
web_server.exe
```

**Linux/Mac**：
```bash
export PORT=18080
./web_server
```

### 3. 默认值

如果未指定命令行参数和环境变量，默认使用端口 **8080**。

## 站点启动脚本

站点启动脚本（如 `site-marine/scripts/start_server.ps1`）通过参数设置端口：

```powershell
# 默认端口 18082
.\scripts\start_server.ps1

# 指定端口
.\scripts\start_server.ps1 -Port 18080
```

脚本内部会设置环境变量：
```powershell
$env:PORT = $Port
```

## 端口优先级总结

```
命令行参数 (--port/-p)
    ↓ (如果未指定)
环境变量 (PORT)
    ↓ (如果未设置)
默认值 (8080)
```

## 示例场景

### 场景1：直接运行可执行文件
```bash
# 使用命令行参数
web_server.exe --port 18080

# 或使用环境变量
$env:PORT = 18080
web_server.exe
```

### 场景2：使用启动脚本
```powershell
# site-marine 默认端口 18082
cd site-marine
.\scripts\start_server.ps1

# 自定义端口
.\scripts\start_server.ps1 -Port 18080
```

### 场景3：开发环境
```bash
# 从项目根目录运行，使用默认端口 8080
cargo run --bin web_server --features web_server

# 或指定端口
PORT=18080 cargo run --bin web_server --features web_server
```

## 修改启动 Host 的方法

Web 服务器支持三种方式配置 host 地址，优先级从高到低：

### 1. 命令行参数（推荐）⭐

使用 `--host` 参数指定 host 地址：

```bash
# Windows - 使用 localhost
web_server.exe --host localhost

# Windows - 使用 127.0.0.1
web_server.exe --host 127.0.0.1

# Linux
./web_server --host localhost
```

**示例**：
```bash
# 本地开发，只监听 localhost
web_server.exe --host localhost --port 8080

# 监听所有网络接口（默认行为）
web_server.exe --host 0.0.0.0

# 结合配置文件使用
web_server.exe --config DbOption_marine --host localhost --port 18080
```

### 2. 环境变量

通过设置 `WEB_SERVER_HOST` 环境变量：

**Windows PowerShell**：
```powershell
$env:WEB_SERVER_HOST = "localhost"
web_server.exe
```

**Windows CMD**：
```cmd
set WEB_SERVER_HOST=localhost
web_server.exe
```

**Linux/Mac**：
```bash
export WEB_SERVER_HOST=localhost
./web_server
```

### 3. 配置文件

在 `DbOption.toml` 或 `DbOption_*.toml` 中设置 `web_server_host`：

```toml
web_server_host = "localhost"
web_server_port = 8080
```

### 4. 默认值

如果未指定命令行参数、环境变量和配置文件，默认使用 `0.0.0.0`（监听所有网络接口）。

## Host 优先级总结

```
命令行参数 (--host)
    ↓ (如果未指定)
环境变量 (WEB_SERVER_HOST)
    ↓ (如果未设置)
配置文件 (web_server_host)
    ↓ (如果未配置)
默认值 (0.0.0.0)
```

## 常见使用场景

### 场景1：本地开发（只允许本机访问）
```bash
# 使用 localhost，只能通过 127.0.0.1 或 localhost 访问
cargo run --bin web_server --features web_server -- --host localhost

# 或使用可执行文件
web_server.exe --host localhost --port 8080
```

### 场景2：局域网访问（允许其他设备访问）
```bash
# 使用 0.0.0.0，允许所有网络接口访问
web_server.exe --host 0.0.0.0 --port 8080

# 或指定本机 IP 地址
web_server.exe --host 192.168.1.100 --port 8080
```

### 场景3：同时指定 host 和 port
```bash
# 本地开发环境
web_server.exe --host localhost --port 8080

# 生产环境
web_server.exe --host 0.0.0.0 --port 18080
```

## 相关文件

- `src/bin/web_server.rs` - 端口和 host 配置逻辑
- `site-*/scripts/start_server.ps1` - 站点启动脚本

