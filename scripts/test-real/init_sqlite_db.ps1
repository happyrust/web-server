# 初始化 SQLite 数据库脚本
# 用于创建站点 1112 和 7000 的 deployment_sites.sqlite

param(
    [Parameter(Mandatory=$true)]
    [string]$SiteDir,
    
    [Parameter(Mandatory=$true)]
    [string]$EnvName,
    
    [Parameter(Mandatory=$true)]
    [string]$Location,
    
    [Parameter(Mandatory=$true)]
    [string]$LocationDbs,
    
    [Parameter(Mandatory=$true)]
    [string]$RemoteSiteName,
    
    [Parameter(Mandatory=$true)]
    [string]$RemoteLocation,
    
    [Parameter(Mandatory=$true)]
    [string]$RemoteHttpHost,
    
    [Parameter(Mandatory=$true)]
    [string]$RemoteDbnums
)

$ErrorActionPreference = "Stop"

# 切换到站点目录
Push-Location $SiteDir

try {
    $dbPath = "deployment_sites.sqlite"
    
    # 如果数据库已存在，删除它
    if (Test-Path $dbPath) {
        Remove-Item $dbPath -Force
        Write-Host "已删除现有数据库: $dbPath"
    }
    
    # 使用 sqlite3 创建数据库和表
    $sql = @"
-- 创建环境表
CREATE TABLE IF NOT EXISTS remote_sync_envs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    mqtt_host TEXT,
    mqtt_port INTEGER,
    file_server_host TEXT,
    location TEXT,
    location_dbs TEXT,
    reconnect_initial_ms INTEGER,
    reconnect_max_ms INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 创建站点表
CREATE TABLE IF NOT EXISTS remote_sync_sites (
    id TEXT PRIMARY KEY,
    env_id TEXT NOT NULL,
    name TEXT NOT NULL,
    location TEXT,
    http_host TEXT,
    dbnums TEXT,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(env_id) REFERENCES remote_sync_envs(id) ON DELETE CASCADE
);

-- 创建日志表
CREATE TABLE IF NOT EXISTS remote_sync_logs (
    id TEXT PRIMARY KEY,
    task_id TEXT,
    env_id TEXT,
    source_env TEXT,
    target_site TEXT,
    site_id TEXT,
    direction TEXT,
    file_path TEXT,
    file_size INTEGER,
    record_count INTEGER,
    status TEXT,
    error_message TEXT,
    notes TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 插入环境记录
INSERT INTO remote_sync_envs (
    id, name, mqtt_host, mqtt_port, file_server_host, location, location_dbs,
    reconnect_initial_ms, reconnect_max_ms, created_at, updated_at
) VALUES (
    '$(New-Guid)', '$EnvName', '127.0.0.1', 1883, 'http://127.0.0.1:8081/assets/archives', '$Location', '$LocationDbs',
    NULL, NULL, datetime('now'), datetime('now')
);

-- 插入远程站点记录（需要先获取环境ID）
-- 注意：这里使用子查询获取刚插入的环境ID
INSERT INTO remote_sync_sites (
    id, env_id, name, location, http_host, dbnums, notes, created_at, updated_at
) VALUES (
    '$(New-Guid)',
    (SELECT id FROM remote_sync_envs WHERE location = '$Location' LIMIT 1),
    '$RemoteSiteName',
    '$RemoteLocation',
    '$RemoteHttpHost',
    '$RemoteDbnums',
    '测试环境远程站点',
    datetime('now'),
    datetime('now')
);
"@
    
    # 检查是否有 sqlite3 命令
    if (Get-Command sqlite3 -ErrorAction SilentlyContinue) {
        $sql | sqlite3 $dbPath
        Write-Host "✓ SQLite 数据库已创建: $dbPath"
    } else {
        Write-Warning "未找到 sqlite3 命令，将使用 Rust 工具创建数据库"
        Write-Host "请运行: cargo run --bin init_test_db -- $SiteDir $EnvName $Location $LocationDbs $RemoteSiteName $RemoteLocation $RemoteHttpHost $RemoteDbnums"
    }
} finally {
    Pop-Location
}








