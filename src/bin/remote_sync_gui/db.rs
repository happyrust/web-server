// Database operations using SQLite

use super::types::*;
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub struct Database {
    conn: Connection,
    base_path: PathBuf,
}

impl Database {
    pub async fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        // Try to find an existing database
        let db_paths = [
            base_path.join("site-1112/deployment_sites.sqlite"),
            base_path.join("site-7000/deployment_sites.sqlite"),
        ];

        let db_path = db_paths.iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| base_path.join("site-1112/deployment_sites.sqlite"));

        let conn = Connection::open(&db_path)?;

        Ok(Self {
            conn,
            base_path,
        })
    }

    pub async fn load_environment(&self) -> Result<EnvironmentConfig> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, mqtt_host, mqtt_port, file_server_host, location, location_dbs,
                    reconnect_initial_ms, reconnect_max_ms, created_at, updated_at
             FROM remote_sync_envs
             LIMIT 1"
        )?;

        let env = stmt.query_row([], |row| {
            let location_dbs_str: String = row.get(6)?;
            let location_dbs: Vec<i32> = location_dbs_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();

            Ok(EnvironmentConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                mqtt_host: row.get(2)?,
                mqtt_port: row.get::<_, i32>(3)? as u16,
                file_server_host: row.get(4)?,
                location: row.get(5)?,
                location_dbs,
                reconnect_initial_ms: row.get(7)?,
                reconnect_max_ms: row.get(8)?,
                created_at: parse_timestamp(row.get::<_, String>(9)?),
                updated_at: parse_timestamp(row.get::<_, String>(10)?),
            })
        })?;

        Ok(env)
    }

    pub async fn save_environment(&self, env: &EnvironmentConfig) -> Result<()> {
        let location_dbs = env.location_dbs.iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",");

        self.conn.execute(
            "INSERT OR REPLACE INTO remote_sync_envs
             (id, name, mqtt_host, mqtt_port, file_server_host, location, location_dbs,
              reconnect_initial_ms, reconnect_max_ms, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &env.id,
                &env.name,
                &env.mqtt_host,
                env.mqtt_port as i32,
                &env.file_server_host,
                &env.location,
                &location_dbs,
                env.reconnect_initial_ms,
                env.reconnect_max_ms,
                env.created_at.to_rfc3339(),
                env.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub async fn load_remote_sites(&self) -> Result<Vec<RemoteSite>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, env_id, name, location, http_host, dbnums, notes, created_at, updated_at
             FROM remote_sync_sites"
        )?;

        let sites = stmt.query_map([], |row| {
            let dbnums_str: String = row.get(5)?;
            let dbnums: Vec<i32> = dbnums_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();

            Ok(RemoteSite {
                id: row.get(0)?,
                env_id: row.get(1)?,
                name: row.get(2)?,
                location: row.get(3)?,
                http_host: row.get(4)?,
                dbnums,
                notes: row.get(6)?,
                enabled: true,
                health: HealthStatus::Unknown,
                last_check: None,
                created_at: parse_timestamp(row.get::<_, String>(7)?),
                updated_at: parse_timestamp(row.get::<_, String>(8)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(sites)
    }

    pub async fn save_remote_site(&self, site: &RemoteSite) -> Result<()> {
        let dbnums = site.dbnums.iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",");

        self.conn.execute(
            "INSERT OR REPLACE INTO remote_sync_sites
             (id, env_id, name, location, http_host, dbnums, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &site.id,
                &site.env_id,
                &site.name,
                &site.location,
                &site.http_host,
                &dbnums,
                &site.notes,
                site.created_at.to_rfc3339(),
                site.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    pub async fn delete_remote_site(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM remote_sync_sites WHERE id = ?1",
            params![id],
        )?;

        Ok(())
    }

    pub async fn load_sync_logs(&self, limit: usize) -> Result<Vec<SyncRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_env, target_site, file_path, file_size, record_count,
                    status, started_at, completed_at, error_message, created_at
             FROM remote_sync_logs
             ORDER BY created_at DESC
             LIMIT ?1"
        )?;

        let logs = stmt.query_map(params![limit], |row| {
            let started_at: Option<String> = row.get(7)?;
            let completed_at: Option<String> = row.get(8)?;
            let created_at: String = row.get(10)?;

            let timestamp = parse_timestamp(created_at);
            let duration_ms = if let (Some(start), Some(end)) = (started_at, completed_at) {
                let start_time = parse_timestamp(start);
                let end_time = parse_timestamp(end);
                (end_time - start_time).num_milliseconds().max(0) as u64
            } else {
                0
            };

            let status_str: String = row.get(6)?;
            let status = match status_str.as_str() {
                "completed" => SyncStatus::Completed,
                "failed" => SyncStatus::Failed,
                "in_progress" => SyncStatus::InProgress,
                _ => SyncStatus::Pending,
            };

            Ok(SyncRecord {
                id: row.get(0)?,
                timestamp,
                source: row.get::<_, String>(1)?.to_string(),
                target: row.get::<_, String>(2)?.to_string(),
                file_path: row.get(3)?,
                file_size: row.get::<_, i64>(4)? as u64,
                record_count: row.get::<_, Option<i64>>(5)?.map(|n| n as u64),
                status,
                duration_ms,
                error_message: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(logs)
    }
}

fn parse_timestamp(s: String) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}
