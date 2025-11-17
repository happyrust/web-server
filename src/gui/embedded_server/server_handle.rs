// Server handle for managing server lifecycle

use std::time::Instant;
use tokio::sync::oneshot;

/// Handle for managing the embedded web server
pub struct ServerHandle {
    /// Server task handle
    task_handle: tokio::task::JoinHandle<anyhow::Result<()>>,
    /// Shutdown signal sender
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Server address
    address: String,
    /// Start time
    started_at: Instant,
}

impl ServerHandle {
    pub fn new(
        task_handle: tokio::task::JoinHandle<anyhow::Result<()>>,
        shutdown_tx: oneshot::Sender<()>,
        address: String,
    ) -> Self {
        Self {
            task_handle,
            shutdown_tx: Some(shutdown_tx),
            address,
            started_at: Instant::now(),
        }
    }
    
    pub fn address(&self) -> &str {
        &self.address
    }
    
    pub fn uptime(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
    
    pub async fn shutdown(mut self) -> anyhow::Result<()> {
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        
        // Wait for task to complete
        match self.task_handle.await {
            Ok(result) => result,
            Err(e) => Err(anyhow::anyhow!("Task join error: {}", e)),
        }
    }
    
    /// Check if the server is still running
    pub fn is_running(&self) -> bool {
        !self.task_handle.is_finished()
    }
}
