// Server metrics collection

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;

/// Server metrics for monitoring
#[derive(Default)]
pub struct ServerMetrics {
    /// Total requests
    pub total_requests: AtomicU64,
    /// Successful requests
    pub success_requests: AtomicU64,
    /// Failed requests
    pub error_requests: AtomicU64,
    /// Active connections
    pub active_connections: AtomicU32,
    /// Peak connections
    pub peak_connections: AtomicU32,
    /// Response time history (last 1000 records)
    pub response_times: Arc<RwLock<VecDeque<ResponseTimeRecord>>>,
    /// Error logs (last 100 records)
    pub error_logs: Arc<RwLock<VecDeque<ErrorLog>>>,
}

/// Response time record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTimeRecord {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
    pub path: String,
}

/// Error log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLog {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub path: String,
    pub method: String,
    pub status_code: u16,
    pub error_message: String,
}

impl ServerMetrics {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn increment_total_requests(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_success_requests(&self) {
        self.success_requests.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_error_requests(&self) {
        self.error_requests.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn increment_active_connections(&self) {
        let current = self.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        // Update peak
        self.peak_connections.fetch_max(current, Ordering::Relaxed);
    }
    
    pub fn decrement_active_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn record_response_time(&self, path: String, duration: std::time::Duration) {
        let mut times = self.response_times.write();
        times.push_back(ResponseTimeRecord {
            timestamp: chrono::Utc::now(),
            duration_ms: duration.as_millis() as u64,
            path,
        });
        // Keep last 1000 records
        if times.len() > 1000 {
            times.pop_front();
        }
    }
    
    pub fn record_error(&self, path: String, method: String, status_code: u16, error_message: String) {
        let mut errors = self.error_logs.write();
        errors.push_back(ErrorLog {
            timestamp: chrono::Utc::now(),
            path,
            method,
            status_code,
            error_message,
        });
        // Keep last 100 records
        if errors.len() > 100 {
            errors.pop_front();
        }
    }
    
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            success_requests: self.success_requests.load(Ordering::Relaxed),
            error_requests: self.error_requests.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            peak_connections: self.peak_connections.load(Ordering::Relaxed),
            avg_response_time_ms: self.calculate_avg_response_time(),
        }
    }
    
    fn calculate_avg_response_time(&self) -> u64 {
        let times = self.response_times.read();
        if times.is_empty() {
            return 0;
        }
        let sum: u64 = times.iter().map(|r| r.duration_ms).sum();
        sum / times.len() as u64
    }
    
    pub fn get_recent_errors(&self, limit: usize) -> Vec<ErrorLog> {
        let errors = self.error_logs.read();
        errors.iter().rev().take(limit).cloned().collect()
    }
}

/// Metrics snapshot for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub success_requests: u64,
    pub error_requests: u64,
    pub active_connections: u32,
    pub peak_connections: u32,
    pub avg_response_time_ms: u64,
}
