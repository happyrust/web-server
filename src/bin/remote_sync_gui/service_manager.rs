// Service process management

use super::types::*;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};
use chrono::Utc;

pub struct ServiceManager {
    services: HashMap<String, ServiceState>,
    project_root: PathBuf,
}

struct ServiceState {
    info: ServiceInfo,
    process: Option<Child>,
}

impl ServiceManager {
    pub fn new() -> Self {
        let project_root = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."));

        let mut services = HashMap::new();

        // Define all services
        let service_configs = vec![
            ("mqtt", "MQTT", None, "start-mqtt-server.ps1", "mqtt.log"),
            ("surreal-1112", "SurrealDB-1112", Some(8021), "start-surreal-1112.ps1", "surreal-1112.log"),
            ("surreal-7000", "SurrealDB-7000", Some(8022), "start-surreal-7000.ps1", "surreal-7000.log"),
            ("web-1112", "WebServer-1112", Some(8081), "start-site-1112.ps1", "web-1112.log"),
            ("web-7000", "WebServer-7000", Some(8082), "start-site-7000.ps1", "web-7000.log"),
        ];

        for (name, display_name, port, script, log_file) in service_configs {
            let script_path = project_root.join("scripts").join("test-real").join(script);

            services.insert(
                name.to_string(),
                ServiceState {
                    info: ServiceInfo {
                        name: name.to_string(),
                        display_name: display_name.to_string(),
                        status: ServiceStatus::Stopped,
                        port,
                        script_path,
                        log_file: log_file.to_string(),
                        started_at: None,
                        process_id: None,
                        cpu_usage: 0.0,
                        memory_mb: 0,
                    },
                    process: None,
                },
            );
        }

        Self {
            services,
            project_root,
        }
    }

    pub async fn start_service(&mut self, name: &str) -> Result<()> {
        let state = self.services.get_mut(name)
            .ok_or_else(|| anyhow!("Service not found: {}", name))?;

        if state.info.status.is_running() {
            return Ok(());
        }

        state.info.status = ServiceStatus::Starting;

        let log_path = self.project_root.join("remote-test-dir/test-real/logs")
            .join(&state.info.log_file);

        // Ensure log directory exists
        if let Some(parent) = log_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Start the service process
        let mut command = Command::new("powershell");
        command
            .arg("-ExecutionPolicy").arg("Bypass")
            .arg("-File").arg(&state.info.script_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let child = command.spawn()?;

        let process_id = child.id();

        state.process = Some(child);
        state.info.status = ServiceStatus::Running;
        state.info.started_at = Some(Utc::now());
        state.info.process_id = process_id;

        Ok(())
    }

    pub async fn stop_service(&mut self, name: &str) -> Result<()> {
        let state = self.services.get_mut(name)
            .ok_or_else(|| anyhow!("Service not found: {}", name))?;

        if let Some(mut process) = state.process.take() {
            state.info.status = ServiceStatus::Stopping;

            // Try to kill the process
            process.kill().await.ok();

            state.info.status = ServiceStatus::Stopped;
            state.info.started_at = None;
            state.info.process_id = None;
        }

        Ok(())
    }

    pub async fn restart_service(&mut self, name: &str) -> Result<()> {
        self.stop_service(name).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        self.start_service(name).await?;
        Ok(())
    }

    pub async fn start_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.services.keys().cloned().collect();

        for name in names {
            if let Err(e) = self.start_service(&name).await {
                eprintln!("Failed to start {}: {}", name, e);
            }
            // Small delay between starting services
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Ok(())
    }

    pub async fn stop_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.services.keys().cloned().collect();

        for name in names {
            if let Err(e) = self.stop_service(&name).await {
                eprintln!("Failed to stop {}: {}", name, e);
            }
        }

        Ok(())
    }

    pub fn get_service(&self, name: &str) -> Option<&ServiceInfo> {
        self.services.get(name).map(|s| &s.info)
    }

    pub fn get_all_services(&self) -> Vec<&ServiceInfo> {
        self.services.values().map(|s| &s.info).collect()
    }

    pub fn running_count(&self) -> usize {
        self.services.values()
            .filter(|s| s.info.status.is_running())
            .count()
    }

    pub async fn check_health(&mut self, name: &str) -> HealthStatus {
        let state = match self.services.get(name) {
            Some(s) => s,
            None => return HealthStatus::Unknown,
        };

        if !state.info.status.is_running() {
            return HealthStatus::Unhealthy;
        }

        // Check if port is accessible (for services with ports)
        if let Some(port) = state.info.port {
            match self.check_port_health(port).await {
                Ok(true) => HealthStatus::Healthy,
                Ok(false) => HealthStatus::Degraded,
                Err(_) => HealthStatus::Unhealthy,
            }
        } else {
            // For services without HTTP endpoints (like MQTT), check process
            match &state.info.process_id {
                Some(_) => HealthStatus::Healthy,
                None => HealthStatus::Unhealthy,
            }
        }
    }

    async fn check_port_health(&self, port: u16) -> Result<bool> {
        let url = format!("http://127.0.0.1:{}/health", port);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()?;

        match client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub async fn update_stats(&mut self) {
        // Update CPU and memory stats for running services
        // This is a simplified version - in production you'd want to use sysinfo crate
        for state in self.services.values_mut() {
            if let Some(pid) = state.info.process_id {
                // TODO: Implement actual CPU/memory monitoring
                // For now, just placeholder values
                state.info.cpu_usage = 0.0;
                state.info.memory_mb = 0;
            }
        }
    }
}
