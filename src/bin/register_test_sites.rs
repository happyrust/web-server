// 注册测试站点到deployment_sites.sqlite的工具
use rusqlite::{Connection, Result};

fn main() -> Result<()> {
    let db_path = "deployment_sites.sqlite";

    if !std::path::Path::new(db_path).exists() {
        eprintln!("错误: deployment_sites.sqlite 不存在");
        return Ok(());
    }

    let conn = Connection::open(db_path)?;

    println!("正在注册测试站点...");

    // 1. 创建测试环境
    conn.execute(
        "INSERT OR REPLACE INTO remote_sync_envs (
            id, name, mqtt_host, mqtt_port, file_server_host, location, location_dbs,
            reconnect_initial_ms, reconnect_max_ms, created_at, updated_at
        ) VALUES (
            'test-env-001', 'Test Environment - Beijing', 'localhost', 1883,
            'http://localhost:8080', 'beijing', '1112', 5000, 60000,
            datetime('now'), datetime('now')
        )",
        [],
    )?;
    println!("✓ 测试环境已注册: test-env-001");

    // 2. 创建上海测试站点
    conn.execute(
        "INSERT OR REPLACE INTO remote_sync_sites (
            id, env_id, name, location, http_host, dbnums, notes, created_at, updated_at
        ) VALUES (
            'test-site-shanghai', 'test-env-001', 'Shanghai Test Site', 'shanghai',
            'http://localhost:9090', '1112', 'Test site for simulating remote synchronization',
            datetime('now'), datetime('now')
        )",
        [],
    )?;
    println!("✓ 上海测试站点已注册: test-site-shanghai");

    // 3. 创建深圳测试站点
    conn.execute(
        "INSERT OR REPLACE INTO remote_sync_sites (
            id, env_id, name, location, http_host, dbnums, notes, created_at, updated_at
        ) VALUES (
            'test-site-shenzhen', 'test-env-001', 'Shenzhen Test Site', 'shenzhen',
            'http://localhost:9091', '1112', 'Another test site for multi-site synchronization',
            datetime('now'), datetime('now')
        )",
        [],
    )?;
    println!("✓ 深圳测试站点已注册: test-site-shenzhen");

    // 验证插入
    println!("\n验证注册结果:");
    let mut stmt = conn.prepare(
        "SELECT id, name, location FROM remote_sync_sites WHERE env_id = 'test-env-001'",
    )?;
    let sites: Vec<_> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect();

    for site_result in sites {
        let (id, name, location) = site_result?;
        println!("  - {} ({}) at {}", name, id, location);
    }

    println!("\n✓ 测试站点注册完成!");
    Ok(())
}
