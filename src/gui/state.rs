// Global application state management

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSyncEnv {
    pub id: String,
    pub name: String,
    pub mqtt_host: Option<String>,
    pub mqtt_port: Option<u16>,
    pub file_server_host: Option<String>,
    pub location: String,
    pub location_dbs: Option<String>,
    pub reconnect_initial_ms: Option<u64>,
    pub reconnect_max_ms: Option<u64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSyncSite {
    pub id: String,
    pub env_id: String,
    pub name: String,
    pub location: String,
    pub http_host: Option<String>,
    pub dbnums: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTask {
    pub id: String,
    pub file_name: String,
    pub source_env: String,
    pub target_site: String,
    pub status: String,
    pub progress: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncLog {
    pub id: String,
    pub task_id: String,
    pub file_path: String,
    pub file_size: u64,
    pub record_count: u32,
    pub source_env: String,
    pub target_site: String,
    pub status: String,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyData {
    pub environments: Vec<RemoteSyncEnv>,
    pub sites: Vec<RemoteSyncSite>,
    pub connections: Vec<TopologyConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyConnection {
    pub env_id: String,
    pub site_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running { address: String },
    Stopping,
    Error(String),
}

pub struct AppState {
    pub environments: Vec<RemoteSyncEnv>,
    pub sites: Vec<RemoteSyncSite>,
    pub sync_tasks: Vec<SyncTask>,
    pub sync_logs: Vec<SyncLog>,
    pub web_server_status: ServerStatus,
    pub topology: TopologyData,
    pub loading: bool,
    pub error: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            environments: Vec::new(),
            sites: Vec::new(),
            sync_tasks: Vec::new(),
            sync_logs: Vec::new(),
            web_server_status: ServerStatus::Stopped,
            topology: TopologyData {
                environments: Vec::new(),
                sites: Vec::new(),
                connections: Vec::new(),
            },
            loading: false,
            error: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
