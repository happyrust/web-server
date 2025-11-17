// System performance monitoring

use sysinfo::System;
use std::time::Instant;

/// System monitor for performance metrics
pub struct SystemMonitor {
    system: System,
    last_update: Instant,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self {
            system,
            last_update: Instant::now(),
        }
    }
    
    pub fn get_info(&mut self) -> SystemInfo {
        // Update every second
        if self.last_update.elapsed() > std::time::Duration::from_secs(1) {
            self.system.refresh_cpu_all();
            self.system.refresh_memory();
            self.last_update = Instant::now();
        }
        
        let cpu_usage = self.system.global_cpu_usage();
        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let memory_used_mb = used_memory / 1024 / 1024;
        let memory_total_mb = total_memory / 1024 / 1024;
        
        SystemInfo {
            cpu_usage,
            memory_used_mb,
            memory_total_mb,
            memory_usage_percent: (used_memory as f32 / total_memory as f32) * 100.0,
        }
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// System information snapshot
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub cpu_usage: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub memory_usage_percent: f32,
}
