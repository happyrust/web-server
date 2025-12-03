# Web服务器启动时的 CBA 文件初始化

## 问题描述

**用户期望**: Web服务器启动时,应该扫描指定的 dbnum,自动生成初始的 CBA 文件。

**实际情况**: 代码已经实现了这个功能,但有一些前提条件和限制。

## 当前实现

### 启动流程

**src/web_server/mod.rs:214-246**:

```rust
pub async fn start_web_server_with_config(port: u16, config_file: Option<&str>) {
    // ...初始化数据库连接...

    // 🔥 启动增量监测后台任务
    if db_option.sync_live.unwrap_or(false) {
        tokio::spawn(async move {
            // 1. 先初始化监听器,扫描并缓存所有数据库文件
            mgr_for_watch.init_watcher().await

            // 2. 然后开始持续监听文件变化
            mgr_for_watch.async_watch().await
        });
    }
}
```

### init_watcher 详细流程

**src/data_interface/increment_manager.rs:771-909**:

#### 第1步: 创建输出目录 (第773行)
```rust
fs::create_dir_all("assets/archives")?;
```

#### 第2步: 扫描数据库文件 (第780-886行)

对于每个监控目录中的文件:

1. **解析数据库基本信息**:
   ```rust
   let DbBasicInfo { db_type, ses_pgno, db_no } = parse_db_basic_info(path);
   ```

2. **过滤数据库** (根据配置):
   ```rust
   // 如果配置了 manual_db_nums,只处理指定的数据库
   if !manual_dbnums.is_empty() && !manual_dbnums.contains(&db_no) {
       continue;
   }

   // 排除 exclude_db_nums 中的数据库
   if !exclude_dbnums.is_empty() && exclude_dbnums.contains(&db_no) {
       continue;
   }
   ```

3. **读取文件和数据库的最新 sesno**:
   ```rust
   let file_latest_sesno = PdmsIO::new(&project, path, true)
       .get_latest_sesno()
       .unwrap_or_default();

   let db_latest_sesno = Self::query_latest_sesno_by_dbnum(db_no).await?;
   ```

4. **生成初始 CBA 文件** (第838-850行):
   ```rust
   #[cfg(feature = "mqtt")]  // ⚠️ 需要 mqtt feature
   {
       let input = path.to_path_buf();
       let output: PathBuf = format!("assets/archives/{}.cba", file_name).into();
       let compress_opt = CompressOptions::new(input, output, "assets/temp");
       execute_compress(compress_opt).await.expect("compress failed");
   }
   ```

5. **检测增量更新** (第863-880行):
   ```rust
   if file_latest_sesno > db_latest_sesno {
       println!("发现需要增量更新的文件: {:?}, 当前数据库属性最大sesno: {}, 文件属性对应sesno: {}",
           file_name, db_latest_sesno, file_latest_sesno);

       // 记录需要更新的会话范围
       params.insert(path, (basic_info, nearest_sesno..=file_latest_sesno));
   }
   ```

#### 第3步: 执行增量更新 (第894-904行)
```rust
if !params.is_empty() {
    match self.execute_incr_update(params).await {
        Ok(true) => println!("执行启动后的自动增量完成。"),
        Ok(false) => println!("没有发生增量更新。"),
        Err(e) => println!("Execute increment update error: {:?}", e),
    }
}
```

## 关键条件和限制

### 1. 配置要求

**DbOption.toml** 必须配置:

```toml
# 启用 SurrealDB 同步
sync_live = true

# 启用 Web服务器自动检测
enable_web_server_auto_detection = true

# 配置监控的项目路径
project_path = "D:/AVEVA/Projects/E3D2.1"
included_projects = ["AvevaMarineSample", "AvevaCatalogue"]

# 可选: 指定要处理的数据库编号
# manual_db_nums = [1112, 1113]  # 如果不配置,处理所有数据库

# 可选: 排除某些数据库
# exclude_db_nums = [9999]
```

### 2. 编译要求

**生成 CBA 文件需要 `mqtt` feature**:

```bash
# 编译时必须启用 mqtt feature
cargo build --bin web_server --features "web_server,mqtt" --release
```

如果没有启用 `mqtt` feature,第838-850行的 CBA 生成代码会被跳过！

### 3. 目录要求

运行前必须存在以下目录:

```
site-xxx/
├── assets/
│   ├── archives/  ← CBA 输出目录 (代码会自动创建)
│   └── temp/      ← 压缩临时目录
└── ...
```

虽然代码会创建 `assets/archives`,但 `assets/temp` 目录必须存在。

### 4. 数据库要求

- SurrealDB 必须运行并可连接
- 数据库中已经有初始数据 (db_latest_sesno > 0)
- 如果 `db_latest_sesno == 0`,会跳过该数据库 (第830行)

## 验证方法

### 1. 检查配置

```bash
cd D:\work\plant\web-server\site-main\config
grep -E "sync_live|enable_web_server_auto_detection" DbOption.toml
```

预期输出:
```toml
sync_live = true
enable_web_server_auto_detection = true
```

### 2. 检查日志

**启动时的日志**:

```bash
cd D:\work\plant\web-server\site-main
tail -100 logs/web_server_18080.log | grep -E "初始化|扫描|发现需要增量"
```

预期看到:
```
📋 正在初始化数据库文件监听器...
✅ 监听器初始化完成
发现需要增量更新的文件: "ams1112_0001", 当前数据库属性最大sesno: 1150, 文件属性对应sesno: 1192
初始化增量更新耗时: 16.258 s
```

### 3. 检查 CBA 文件

```bash
ls -lh site-main/assets/archives/*.cba
```

预期看到:
```
-rw-r--r-- 1 Administrator 197121 13M Nov 22 08:15 ams1112_0001.cba
-rw-r--r-- 1 Administrator 197121 5.0K Nov 22 08:15 amscom.cba
```

### 4. 检查编译 feature

如果 CBA 文件没有生成,可能是缺少 `mqtt` feature:

```bash
# 重新编译,确保包含 mqtt feature
cd D:\work\plant\web-server
cargo build --bin web_server --features "web_server,mqtt" --release

# 复制到 site-main
cp target/release/web_server.exe site-main/bin/
```

## 常见问题排查

### 问题 1: 启动时没有扫描数据库

**症状**: 日志中没有 "初始化数据库文件监听器" 消息

**原因**: `sync_live = false`

**解决**:
```toml
# DbOption.toml
sync_live = true
enable_web_server_auto_detection = true
```

### 问题 2: 扫描了但没有生成 CBA

**症状**: 看到初始化日志,但 `assets/archives/` 目录是空的

**可能原因**:

1. **缺少 `mqtt` feature** (最常见):
   ```bash
   cargo build --bin web_server --features "web_server,mqtt" --release
   ```

2. **数据库中 sesno = 0**:
   ```sql
   -- 查询数据库中的 sesno
   SELECT MAX(SESNO) FROM pe WHERE db_num = 1112;
   ```
   如果返回 0 或 NULL,代码会跳过 (第830行)

3. **文件和数据库 sesno 一致**:
   如果 `file_latest_sesno == db_latest_sesno`,不会触发增量更新

### 问题 3: 目录权限问题

**症状**: 日志中有 "Permission denied" 错误

**解决**:
```bash
# 确保有写权限
chmod -R 755 site-main/assets/
```

### 问题 4: 压缩失败

**症状**: 日志中有 "compress failed" 错误

**检查**:
1. `assets/temp` 目录是否存在
2. 磁盘空间是否充足
3. Archive 文件是否损坏

## 最佳实践

### 1. 部署前检查清单

```bash
#!/bin/bash
# check_startup_init.sh

SITE_DIR="site-main"

# 1. 检查配置
echo "1. 检查配置..."
if grep -q "sync_live = true" "$SITE_DIR/config/DbOption.toml"; then
    echo "   ✅ sync_live = true"
else
    echo "   ❌ sync_live 未启用"
fi

# 2. 检查目录
echo "2. 检查目录..."
for dir in "$SITE_DIR/assets/archives" "$SITE_DIR/assets/temp"; do
    if [ -d "$dir" ]; then
        echo "   ✅ $dir 存在"
    else
        echo "   ❌ $dir 不存在,正在创建..."
        mkdir -p "$dir"
    fi
done

# 3. 检查编译 feature (需要手动确认)
echo "3. 检查编译配置..."
echo "   ⚠️  请确认编译时使用了 --features \"web_server,mqtt\""

# 4. 检查数据库连接
echo "4. 检查数据库..."
echo "   请确认 SurrealDB 在运行: http://localhost:8020"
```

### 2. 启动脚本增强

在 `start_server.ps1` 中添加检查:

```powershell
# 启动前检查
if (-not (Test-Path (Join-Path $siteRoot "assets/archives"))) {
    Write-Warning "创建 assets/archives 目录"
    New-Item -ItemType Directory -Force -Path (Join-Path $siteRoot "assets/archives")
}

if (-not (Test-Path (Join-Path $siteRoot "assets/temp"))) {
    Write-Warning "创建 assets/temp 目录"
    New-Item -ItemType Directory -Force -Path (Join-Path $siteRoot "assets/temp")
}

# 检查配置
$configContent = Get-Content (Join-Path $configFile.DirectoryName $configFile.Name)
if ($configContent -notmatch "sync_live\s*=\s*true") {
    Write-Warning "sync_live 未启用,将不会自动生成 CBA 文件"
}
```

### 3. 监控脚本

```powershell
# monitor_cba_generation.ps1
param(
    [string]$SiteDir = "site-main"
)

$logFile = Join-Path $SiteDir "logs/web_server_18080.log"
$cbaDir = Join-Path $SiteDir "assets/archives"

Write-Host "监控 CBA 文件生成..." -ForegroundColor Cyan

# 检查日志
$initLog = Select-String -Path $logFile -Pattern "初始化数据库文件监听器" | Select-Object -Last 1
if ($initLog) {
    Write-Host "✅ 监听器已初始化: $($initLog.Line)" -ForegroundColor Green
} else {
    Write-Host "❌ 未找到监听器初始化日志" -ForegroundColor Red
}

# 检查 CBA 文件
$cbaFiles = Get-ChildItem -Path $cbaDir -Filter "*.cba" -ErrorAction SilentlyContinue
if ($cbaFiles) {
    Write-Host "✅ 找到 $($cbaFiles.Count) 个 CBA 文件:" -ForegroundColor Green
    $cbaFiles | ForEach-Object {
        Write-Host "   - $($_.Name) ($([math]::Round($_.Length / 1MB, 2)) MB)" -ForegroundColor Gray
    }
} else {
    Write-Host "❌ 未找到 CBA 文件" -ForegroundColor Red
    Write-Host "   可能原因:" -ForegroundColor Yellow
    Write-Host "   1. 缺少 mqtt feature (重新编译)" -ForegroundColor Yellow
    Write-Host "   2. 数据库中 sesno = 0" -ForegroundColor Yellow
    Write-Host "   3. 文件和数据库 sesno 一致" -ForegroundColor Yellow
}
```

## 总结

### ✅ Web服务器启动时**已经**在做:

1. 扫描所有配置的数据库文件
2. 读取文件和数据库的 sesno
3. 生成初始 CBA 文件 (如果启用了 `mqtt` feature)
4. 检测并执行增量更新

### ⚠️ 需要满足的条件:

1. **配置**: `sync_live = true`
2. **编译**: 包含 `mqtt` feature
3. **目录**: `assets/archives` 和 `assets/temp` 存在
4. **数据库**: SurrealDB 运行且有数据 (sesno > 0)

### 🔧 如果 CBA 没有生成:

1. 检查编译时是否包含 `--features "mqtt"`
2. 检查日志中的初始化信息
3. 验证数据库中的 sesno 不为 0
4. 确保目录有写权限

### 📝 建议:

考虑将 CBA 生成从 `#[cfg(feature = "mqtt")]` 中移出,改为运行时配置:

```rust
// 当前: 编译时决定
#[cfg(feature = "mqtt")]
{
    execute_compress(compress_opt).await?;
}

// 建议: 运行时配置决定
if get_db_option().enable_cba_generation.unwrap_or(true) {
    execute_compress(compress_opt).await?;
}
```

这样即使不使用 MQTT,也可以生成 CBA 文件用于 HTTP 下载。
