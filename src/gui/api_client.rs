// HTTP API client for communicating with the backend

use crate::gui::state::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub running: bool,
    pub paused: bool,
    pub queue_size: usize,
    pub active_tasks: usize,
    pub mqtt_connected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogFilters {
    pub env_id: Option<String>,
    pub site_id: Option<String>,
    pub status: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub limit: Option<usize>,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
    
    // Environment management API
    pub async fn get_environments(&self) -> Result<Vec<RemoteSyncEnv>> {
        let url = format!("{}/api/remote-sync/envs", self.base_url);
        let response = self.client.get(&url).send().await?;
        let data: Vec<RemoteSyncEnv> = response.json().await?;
        Ok(data)
    }
    
    pub async fn create_environment(&self, env: &RemoteSyncEnv) -> Result<String> {
        let url = format!("{}/api/remote-sync/envs", self.base_url);
        let response = self.client.post(&url).json(env).send().await?;
        let result: serde_json::Value = response.json().await?;
        Ok(result["id"].as_str().unwrap_or("").to_string())
    }
    
    pub async fn update_environment(&self, id: &str, env: &RemoteSyncEnv) -> Result<()> {
        let url = format!("{}/api/remote-sync/envs/{}", self.base_url, id);
        self.client.put(&url).json(env).send().await?;
        Ok(())
    }
    
    pub async fn delete_environment(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/remote-sync/envs/{}", self.base_url, id);
        self.client.delete(&url).send().await?;
        Ok(())
    }
    
    pub async fn activate_environment(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/remote-sync/envs/{}/activate", self.base_url, id);
        self.client.post(&url).send().await?;
        Ok(())
    }
    
    // Site management API
    pub async fn get_sites(&self) -> Result<Vec<RemoteSyncSite>> {
        let url = format!("{}/api/remote-sync/sites", self.base_url);
        let response = self.client.get(&url).send().await?;
        let data: Vec<RemoteSyncSite> = response.json().await?;
        Ok(data)
    }
    
    pub async fn create_site(&self, site: &RemoteSyncSite) -> Result<String> {
        let url = format!("{}/api/remote-sync/sites", self.base_url);
        let response = self.client.post(&url).json(site).send().await?;
        let result: serde_json::Value = response.json().await?;
        Ok(result["id"].as_str().unwrap_or("").to_string())
    }
    
    pub async fn test_site_connection(&self, id: &str) -> Result<TestResult> {
        let url = format!("{}/api/remote-sync/sites/{}/test", self.base_url, id);
        let response = self.client.post(&url).send().await?;
        let result: TestResult = response.json().await?;
        Ok(result)
    }
    
    // Sync control API
    pub async fn start_sync(&self) -> Result<()> {
        let url = format!("{}/api/sync/start", self.base_url);
        self.client.post(&url).send().await?;
        Ok(())
    }
    
    pub async fn stop_sync(&self) -> Result<()> {
        let url = format!("{}/api/sync/stop", self.base_url);
        self.client.post(&url).send().await?;
        Ok(())
    }
    
    pub async fn pause_sync(&self) -> Result<()> {
        let url = format!("{}/api/sync/pause", self.base_url);
        self.client.post(&url).send().await?;
        Ok(())
    }
    
    pub async fn resume_sync(&self) -> Result<()> {
        let url = format!("{}/api/sync/resume", self.base_url);
        self.client.post(&url).send().await?;
        Ok(())
    }
    
    pub async fn clear_queue(&self) -> Result<()> {
        let url = format!("{}/api/sync/queue/clear", self.base_url);
        self.client.post(&url).send().await?;
        Ok(())
    }
    
    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let url = format!("{}/api/sync/status", self.base_url);
        let response = self.client.get(&url).send().await?;
        let status: SyncStatus = response.json().await?;
        Ok(status)
    }
    
    // Log query API
    pub async fn query_logs(&self, filters: &LogFilters) -> Result<Vec<SyncLog>> {
        let url = format!("{}/api/remote-sync/logs", self.base_url);
        let response = self.client.get(&url).query(filters).send().await?;
        let logs: Vec<SyncLog> = response.json().await?;
        Ok(logs)
    }
    
    // Topology configuration API
    pub async fn save_topology(&self, topology: &TopologyData) -> Result<()> {
        let url = format!("{}/api/topology/save", self.base_url);
        self.client.post(&url).json(topology).send().await?;
        Ok(())
    }
    
    pub async fn load_topology(&self) -> Result<TopologyData> {
        let url = format!("{}/api/topology/load", self.base_url);
        let response = self.client.get(&url).send().await?;
        let topology: TopologyData = response.json().await?;
        Ok(topology)
    }
}
