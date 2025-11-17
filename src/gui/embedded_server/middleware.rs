// Middleware for metrics collection and logging

use super::metrics::ServerMetrics;
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

/// Request log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub client_ip: String,
}

/// Request logger
pub struct RequestLogger {
    logs: Arc<RwLock<VecDeque<RequestLog>>>,
    max_logs: usize,
}

impl RequestLogger {
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Arc::new(RwLock::new(VecDeque::new())),
            max_logs,
        }
    }
    
    pub fn log(&self, log: RequestLog) {
        let mut logs = self.logs.write();
        logs.push_back(log);
        if logs.len() > self.max_logs {
            logs.pop_front();
        }
    }
    
    pub fn get_logs(&self) -> Vec<RequestLog> {
        self.logs.read().iter().cloned().collect()
    }
    
    pub fn clear(&self) {
        self.logs.write().clear();
    }
}

/// Metrics middleware
pub async fn metrics_middleware(
    req: Request,
    next: Next,
    metrics: Arc<ServerMetrics>,
) -> Response {
    metrics.increment_total_requests();
    metrics.increment_active_connections();
    
    let path = req.uri().path().to_string();
    let start = Instant::now();
    
    let response = next.run(req).await;
    
    let duration = start.elapsed();
    metrics.decrement_active_connections();
    
    // Record response time
    metrics.record_response_time(path.clone(), duration);
    
    // Update success/error counters
    if response.status().is_success() {
        metrics.increment_success_requests();
    } else {
        metrics.increment_error_requests();
        metrics.record_error(
            path,
            "GET".to_string(), // Simplified - should extract from request
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown").to_string(),
        );
    }
    
    response
}

/// Logging middleware
pub async fn logging_middleware(
    req: Request,
    next: Next,
    logger: Arc<RequestLogger>,
) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    
    let start = Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed();
    
    logger.log(RequestLog {
        timestamp: chrono::Utc::now(),
        method,
        path,
        status_code: response.status().as_u16(),
        duration_ms: duration.as_millis() as u64,
        client_ip,
    });
    
    response
}
