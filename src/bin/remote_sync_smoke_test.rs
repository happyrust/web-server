//! Lightweight remote-sync smoke test.
//!
//! This binary runs a minimal end-to-end verification of the remote
//! sync pipeline in-process and cross‑platform:
//!
//! 1. Creates a temporary working directory and SQLite database
//!    (`deployment_sites.sqlite`) with one env and one site.
//! 2. Starts a tiny HTTP file receiver to simulate a remote site.
//! 3. Generates a fake `.cba` archive locally.
//! 4. Uses `SyncControlCenter` + `process_sync_task` to resolve the
//!    destination and push the file via HTTP to the remote receiver.
//! 5. Asserts that the remote file exists and prints a short summary.
//!
//! It does **not** require SurrealDB or PDMS files; only filesystem,
//! SQLite and HTTP client/server are used.

#![allow(clippy::unused_io_amount)]

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::Context;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, put};
use rusqlite::Connection;
use serde_json::json;
use tokio::net::TcpListener;

use aios_database::web_server::sync_control_center::{
    NewSyncTaskParams, SYNC_CONTROL_CENTER, SyncTask, process_sync_task_for_test,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("[remote-sync-smoke] starting lightweight remote sync test...");

    // 1. Prepare local working directory and switch CWD into it so all
    //    relative paths (DbOption.toml, assets, output, SQLite) are local.
    let work_root = PathBuf::from("remote-test-dir");
    std::fs::create_dir_all(&work_root).context("create remote-test-dir failed")?;
    std::env::set_current_dir(&work_root)?;

    println!("[remote-sync-smoke] work root: {}", work_root.display());

    // 2. Write minimal DbOption.toml so remote_sync_handlers::open_sqlite
    //    and sync_control_center can locate the deployment_sites.sqlite.
    //    这里使用相对路径，指向当前工作目录下的文件，避免出现
    //    "remote-test-dir/remote-test-dir/deployment_sites.sqlite" 这种嵌套情况。
    let sqlite_path = PathBuf::from("deployment_sites.sqlite");
    if sqlite_path.exists() {
        std::fs::remove_file(&sqlite_path)?;
    }
    write_minimal_dboption(&sqlite_path)?;

    // 3. Initialize deployment_sites.sqlite with one env + one site.
    let (env_id, env_name, site_id, site_name, sjz_port) =
        init_remote_sync_topology(&sqlite_path).context("init topology failed")?;

    // 4. Start remote HTTP file receiver (simulating SJZ node).
    let receiver_root = work_root.join("sjz_files");
    tokio::fs::create_dir_all(&receiver_root).await?;
    let receiver_handle = start_file_receiver(sjz_port, receiver_root.clone()).await?;

    // 5. Create a fake .cba archive under assets/archives.
    let file_name = format!("smoke_{}.cba", rand::random::<u32>());
    let local_archive = create_fake_archive(&file_name).await?;

    // 6. Enqueue a sync task through SyncControlCenter and process it
    //    directly via process_sync_task, without starting the full
    //    runtime / watcher / MQTT.
    run_single_sync_task(&env_id, &env_name, &site_id, &site_name, &local_archive).await?;

    // 7. Verify that the remote file was received.
    let expected_remote = find_received_file(&receiver_root, &file_name).await?;

    // 8. Verify that a completed log entry exists for this file.
    verify_remote_sync_logs(&sqlite_path, &file_name)?;

    // 9. Verify that the remote file is accessible via HTTP from the receiver.
    verify_remote_file_http(sjz_port, &env_name, &site_name, "UPLOAD", &file_name).await?;

    println!("[remote-sync-smoke] SUCCESS");
    println!("  env:  {}", env_name);
    println!("  site: {}", site_name);
    println!("  local archive: {}", local_archive.display());
    println!("  remote received: {}", expected_remote.display());
    println!("[remote-sync-smoke] RESULT: ok");

    // Shut down HTTP server task.
    receiver_handle.abort();

    Ok(())
}

fn write_minimal_dboption(sqlite_path: &Path) -> anyhow::Result<()> {
    let content = format!(
        "deployment_sites_sqlite_path = \"{}\"\n",
        sqlite_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid sqlite path"))?
            .replace('\\', "/"),
    );
    std::fs::write("DbOption.toml", content)?;
    Ok(())
}

#[allow(clippy::type_complexity)]
fn init_remote_sync_topology(
    sqlite_path: &Path,
) -> anyhow::Result<(String, String, String, String, u16)> {
    let mut conn = Connection::open(sqlite_path)?;

    // Ensure tables exist (same schema as remote_sync_handlers::open_sqlite).
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

    let now = chrono::Utc::now().to_rfc3339();
    let env_id = uuid::Uuid::new_v4().to_string();
    let env_name = "bj-env".to_string();

    conn.execute(
        "INSERT INTO remote_sync_envs (
            id, name, mqtt_host, mqtt_port, file_server_host, location, location_dbs,
            reconnect_initial_ms, reconnect_max_ms, created_at, updated_at
        ) VALUES (?1, ?2, NULL, NULL, NULL, ?3, NULL, NULL, NULL, ?4, ?4)",
        rusqlite::params![env_id, env_name, "bj", now],
    )?;

    let site_id = uuid::Uuid::new_v4().to_string();
    let site_name = "sjz-site".to_string();

    // Choose a random port in an unprivileged range for the file receiver.
    let sjz_port: u16 = rand::random_range(18000u16..19000u16);
    let http_host = format!("http://127.0.0.1:{}/files", sjz_port);

    conn.execute(
        "INSERT INTO remote_sync_sites (
            id, env_id, name, location, http_host, dbnums, notes, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?7)",
        rusqlite::params![
            site_id,
            env_id,
            site_name,
            "sjz",
            http_host,
            "smoke-test site",
            now,
        ],
    )?;

    Ok((env_id, env_name, site_id, site_name, sjz_port))
}

async fn start_file_receiver(
    port: u16,
    base_dir: PathBuf,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    #[derive(Clone)]
    struct ReceiverState {
        root: PathBuf,
    }

    async fn handle_put(
        AxumPath(wildcard): AxumPath<String>,
        State(state): State<ReceiverState>,
        body: Bytes,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        let mut full = state.root.clone();
        for segment in wildcard.split('/') {
            if segment.is_empty() {
                continue;
            }
            full.push(segment);
        }

        if let Some(parent) = full.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }

        tokio::fs::write(&full, &body)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(json!({
            "status": "ok",
            "saved": full.to_string_lossy().to_string(),
        })))
    }

    async fn handle_get(
        AxumPath(wildcard): AxumPath<String>,
        State(state): State<ReceiverState>,
    ) -> Result<(StatusCode, Bytes), StatusCode> {
        let mut full = state.root.clone();
        for segment in wildcard.split('/') {
            if segment.is_empty() {
                continue;
            }
            full.push(segment);
        }

        match tokio::fs::read(&full).await {
            Ok(data) => Ok((StatusCode::OK, Bytes::from(data))),
            Err(_) => Err(StatusCode::NOT_FOUND),
        }
    }

    let state = ReceiverState { root: base_dir };
    let app = Router::new()
        .route("/files/{*wildcard}", get(handle_get).put(handle_put))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind receiver on {} failed", addr))?;

    println!("[remote-sync-smoke] file receiver listening on {addr}");

    let handle = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("[remote-sync-smoke] receiver error: {err}");
        }
    });

    Ok(handle)
}

async fn create_fake_archive(file_name: &str) -> anyhow::Result<PathBuf> {
    let archives_dir = PathBuf::from("assets").join("archives");
    tokio::fs::create_dir_all(&archives_dir).await?;
    let archive_path = archives_dir.join(file_name);
    tokio::fs::write(&archive_path, b"remote_sync_smoke_test").await?;
    Ok(archive_path)
}

async fn run_single_sync_task(
    env_id: &str,
    env_name: &str,
    site_id: &str,
    site_name: &str,
    archive_path: &Path,
) -> anyhow::Result<()> {
    let meta = tokio::fs::metadata(archive_path).await?;
    let file_size = meta.len();

    let file_name = archive_path
        .file_name()
        .and_then(|os| os.to_str())
        .unwrap_or("smoke.cba")
        .to_string();

    let file_path_str = archive_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("invalid archive path"))?
        .to_string();

    // 1. Enqueue a task into the global SyncControlCenter.
    let center_arc = SYNC_CONTROL_CENTER.clone();
    let task_id = {
        let mut center = center_arc.write().await;
        center.add_task(NewSyncTaskParams {
            file_path: file_path_str.clone(),
            file_size,
            priority: 5,
            file_name: Some(file_name.clone()),
            file_hash: None,
            record_count: None,
            env_id: Some(env_id.to_string()),
            source_env: Some("smoke-test".to_string()),
            target_site: Some(site_id.to_string()),
            direction: Some("UPLOAD".to_string()),
            notes: Some(format!("smoke-test sync to {} ({})", site_name, env_name)),
        })
    };

    println!("[remote-sync-smoke] enqueued task: {}", task_id);

    // 2. Take the next task and process it directly.
    let task: SyncTask = {
        let mut center = center_arc.write().await;
        center
            .get_next_task()
            .ok_or_else(|| anyhow::anyhow!("no pending task found"))?
    };

    // Sanity check: IDs must match.
    assert_eq!(task.id, task_id);

    if let Err(err) = process_sync_task_for_test(&task).await {
        let mut center = center_arc.write().await;
        let msg: String = err.to_string();
        center.complete_task(&task.id, false, Some(msg.clone()));
        anyhow::bail!("process_sync_task failed: {msg}");
    } else {
        let mut center = center_arc.write().await;
        center.complete_task(&task.id, true, None);
    }

    Ok(())
}

async fn find_received_file(root: &Path, file_name: &str) -> anyhow::Result<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .and_then(|os| os.to_str())
                .map(|name| name == file_name)
                .unwrap_or(false)
            {
                return Ok(path);
            }
        }
    }
    anyhow::bail!("remote file {file_name} not found under {}", root.display());
}

fn verify_remote_sync_logs(sqlite_path: &Path, file_name: &str) -> anyhow::Result<()> {
    let conn = Connection::open(sqlite_path)?;

    let mut stmt = conn.prepare(
        "SELECT id, status, file_path FROM remote_sync_logs ORDER BY created_at DESC LIMIT 100",
    )?;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let status: String = row.get(1)?;
        if status != "completed" {
            continue;
        }
        let path: String = row.get(2)?;
        if path.contains(file_name) {
            return Ok(());
        }
    }

    anyhow::bail!(
        "no completed remote_sync_logs entry found for file {file_name} in {:?}",
        sqlite_path
    );
}

async fn verify_remote_file_http(
    sjz_port: u16,
    env_name: &str,
    site_name: &str,
    direction: &str,
    file_name: &str,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "http://127.0.0.1:{}/files/{}/{}/{}/{}",
        sjz_port,
        urlencoding::encode(env_name),
        urlencoding::encode(site_name),
        urlencoding::encode(direction),
        urlencoding::encode(file_name),
    );

    for _ in 0..10 {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                return Ok(());
            }
            _ => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }

    anyhow::bail!("remote file not accessible via HTTP: {}", url);
}
