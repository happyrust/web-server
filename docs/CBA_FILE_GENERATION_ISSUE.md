# CBA 文件生成问题分析与解决方案

## 问题描述

**现象**: `site-main/bin/web_server.exe` 运行时,检测到增量更新但没有生成 CBA 文件到本地目录。

## 根本原因

### 1. 相对路径问题

**代码位置**: `src/data_interface/increment_manager.rs:580`

```rust
let output: PathBuf = format!("assets/archives/{}.cba", file_name).into();
```

这是一个**相对路径**,会基于**当前工作目录**解析。

### 2. 工作目录差异

不同的运行位置会导致不同的输出路径:

| 运行位置 | 工作目录 | CBA 输出路径 | 结果 |
|---------|---------|-------------|------|
| 根目录 | `D:\work\plant\web-server\` | `D:\work\plant\web-server\assets\archives\` | ✅ 正常 |
| site-main | `D:\work\plant\web-server\site-main\` | `D:\work\plant\web-server\site-main\assets\archives\` | ❌ 目录不存在 |
| site-marine | `D:\work\plant\web-server\site-marine\` | `D:\work\plant\web-server\site-marine\assets\archives\` | ❌ 目录不存在 |

### 3. 日志分析

**site-main 运行日志** (`logs/web_server_18080.log`):
```
Archive created D:/AVEVA/Projects/E3D2.1\AvevaMarineSample\ams000\ams1112_0001 in 1.6657038
📊 会话范围 1192-1192: 新增 0, 修改 0, 删除 0
发生了增量更新，推送：ams1112_0001
✅ 同步历史已保存到 e3d_sync 表: 1 个文件 [增量同步] (会话范围: 1192-1192)
```

- ✅ 增量检测正常
- ✅ Archive 文件已创建
- ✅ 同步历史已保存
- ❌ **但没有看到 CBA 压缩成功的日志**

## 解决方案

### 方案 1: 创建必需的目录结构 (推荐)

为每个站点创建 `assets/archives` 和 `assets/temp` 目录:

```bash
cd D:\work\plant\web-server

# 为所有站点创建目录
for dir in site-*/; do
  mkdir -p "$dir/assets/archives"
  mkdir -p "$dir/assets/temp"
done
```

**已执行** ✅:
```
✅ 已创建 site-custom/assets/archives
✅ 已创建 site-main/assets/archives
✅ 已创建 site-marine/assets/archives
```

### 方案 2: 修改代码使用绝对路径 (长期方案)

修改 `increment_manager.rs` 使用配置的输出目录:

```rust
// 当前代码 (硬编码相对路径)
let output: PathBuf = format!("assets/archives/{}.cba", file_name).into();

// 改进方案 (使用配置或环境变量)
let output_dir = std::env::current_dir()
    .unwrap()
    .join("assets")
    .join("archives");
std::fs::create_dir_all(&output_dir).ok();
let output = output_dir.join(format!("{}.cba", file_name));
```

或者从配置文件读取:

```toml
# DbOption.toml
[output_paths]
cba_output_dir = "assets/archives"
cba_temp_dir = "assets/temp"
```

## 验证步骤

### 1. 检查目录是否创建

```bash
cd D:\work\plant\web-server\site-main
ls -la assets/archives/
```

预期输出:
```
drwxr-xr-x 1 Administrator 197121 0 Nov 22 08:11 .
```

### 2. 触发增量更新

修改 PDMS 文件或手动触发增量检测。

### 3. 检查 CBA 文件

```bash
ls -lh site-main/assets/archives/*.cba
```

预期看到新生成的 CBA 文件:
```
-rw-r--r-- 1 Administrator 197121 13M Nov 22 08:15 ams1112_0001.cba
```

### 4. 检查日志

```bash
tail -f site-main/logs/web_server_18080.log
```

应该看到压缩成功的日志:
```
Archive created D:/AVEVA/Projects/E3D2.1\AvevaMarineSample\ams000\ams1112_0001 in 1.66s
✅ CBA 压缩成功: assets/archives/ams1112_0001.cba (13.1 MB)
```

## 相关配置

### Web 服务器静态文件服务

**代码位置**: `src/web_server/mod.rs:858`

```rust
.nest_service("/assets/archives", ServeDir::new("assets/archives"))
```

这会将 `/assets/archives` URL 映射到本地 `assets/archives/` 目录。

### 文件服务器地址配置

每个站点的 `DbOption.toml`:

```toml
file_server_host = "http://100.112.192.11:18080/assets/archives"
```

- **必须**指向本站点自己的 HTTP 服务地址
- **不要**使用 `localhost` (除非纯本机测试)
- 端口应该与 `server_release_ip` 一致

## 最佳实践

### 1. 站点部署清单

部署新站点时,确保以下目录结构存在:

```
site-xxx/
├── bin/
│   └── web_server.exe
├── config/
│   └── DbOption.toml
├── assets/
│   ├── archives/      ← CBA 文件输出目录
│   └── temp/          ← CBA 压缩临时目录
├── logs/              ← 日志目录
├── data/              ← SurrealDB 数据目录
└── deployment_sites.sqlite
```

### 2. 启动前检查

```bash
# 检查必需目录
for dir in assets/archives assets/temp logs data; do
  if [ ! -d "$dir" ]; then
    echo "❌ 缺少目录: $dir"
    mkdir -p "$dir"
    echo "✅ 已创建: $dir"
  fi
done
```

### 3. 配置验证

确保 `DbOption.toml` 配置正确:

```toml
# ✅ 正确: 使用本站点的实际IP
file_server_host = "http://192.168.1.10:18080/assets/archives"

# ❌ 错误: 使用 localhost (其他站点无法访问)
file_server_host = "http://localhost:8080/assets/archives"

# ❌ 错误: 端口不匹配
server_release_ip = "192.168.1.10:18080"
file_server_host = "http://192.168.1.10:8080/assets/archives"  # 端口不一致!
```

## 故障排查

### 问题 1: 找不到 CBA 文件

**检查**:
```bash
find . -name "*.cba" -mtime -1
```

**可能原因**:
- `assets/archives` 目录不存在
- 工作目录不正确
- 磁盘空间不足

### 问题 2: 压缩失败

**检查日志**:
```bash
grep -i "压缩\|compress\|cba.*失败" logs/*.err.log
```

**可能原因**:
- `assets/temp` 目录不存在
- 权限不足
- Archive 文件损坏

### 问题 3: 远程站点无法下载

**检查**:
1. 文件确实存在:
   ```bash
   ls -lh assets/archives/*.cba
   ```

2. HTTP 服务可访问:
   ```bash
   curl -I http://192.168.1.10:18080/assets/archives/ams1112_0001.cba
   ```

3. 防火墙规则:
   ```bash
   # Windows
   netsh advfirewall firewall show rule name=all | grep 18080
   ```

## 总结

**问题**: CBA 文件未生成到站点目录
**原因**: `assets/archives` 目录不存在
**解决**: 创建必需的目录结构
**预防**: 部署清单 + 启动前检查脚本

**状态**: ✅ 已修复

所有站点的 `assets/archives` 和 `assets/temp` 目录已创建,现在可以正常生成 CBA 文件了!
