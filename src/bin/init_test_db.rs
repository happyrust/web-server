//! 初始化测试环境的 SQLite 数据库工具
//!
//! 用法:
//!   cargo run --bin init_test_db -- <site_dir> <env_name> <location> <location_dbs> <remote_site_name> <remote_location> <remote_http_host> <remote_dbnums>

use chrono::Utc;
use rusqlite::Connection;
use std::path::PathBuf;
use uuid::Uuid;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 9 {
        eprintln!(
            "用法: {} <site_dir> <env_name> <location> <location_dbs> <remote_site_name> <remote_location> <remote_http_host> <remote_dbnums>",
            args[0]
        );
        std::process::exit(1);
    }

    let site_dir = PathBuf::from(&args[1]);
    let env_name = &args[2];
    let location = &args[3];
    let location_dbs = &args[4];
    let remote_site_name = &args[5];
    let remote_location = &args[6];
    let remote_http_host = &args[7];
    let remote_dbnums = &args[8];

    let db_path = site_dir.join("deployment_sites.sqlite");

    // 如果数据库已存在，删除它
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
        println!("已删除现有数据库: {}", db_path.display());
    }

    let mut conn = Connection::open(&db_path)?;

    // 创建表结构
    conn.execute(
        "CREATE TABLE IF NOT EXISTS remote_sync_envs (
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
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS remote_sync_sites (
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
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS remote_sync_logs (
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
        )",
        [],
    )?;

    // 插入环境记录
    let now = Utc::now().to_rfc3339();
    let env_id = Uuid::new_v4().to_string();

    // 确定文件服务器地址（根据 location 判断端口）
    let file_server_host = if location == "SITE1112" {
        "http://127.0.0.1:8081/assets/archives"
    } else {
        "http://127.0.0.1:8082/assets/archives"
    };

    conn.execute(
        "INSERT INTO remote_sync_envs (
            id, name, mqtt_host, mqtt_port, file_server_host, location, location_dbs,
            reconnect_initial_ms, reconnect_max_ms, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, ?8, ?8)",
        rusqlite::params![
            env_id,
            env_name,
            "127.0.0.1",
            1883,
            file_server_host,
            location,
            location_dbs,
            now
        ],
    )?;

    // 插入远程站点记录
    let site_id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO remote_sync_sites (
            id, env_id, name, location, http_host, dbnums, notes, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        rusqlite::params![
            site_id,
            env_id,
            remote_site_name,
            remote_location,
            remote_http_host,
            remote_dbnums,
            "测试环境远程站点",
            now,
        ],
    )?;

    println!("✓ SQLite 数据库已创建: {}", db_path.display());
    println!("  环境: {} ({})", env_name, location);
    println!("  远程站点: {} ({})", remote_site_name, remote_location);

    Ok(())
}
