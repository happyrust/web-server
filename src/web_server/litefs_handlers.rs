use axum::{Json, http::StatusCode};
use serde_json::{Value, json};

/// 健康检查 API - 只检查数据库状态
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    use crate::web_server::remote_sync_handlers::open_sqlite;

    let db_status = match open_sqlite() {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    let overall_status = if db_status == "healthy" {
        "ok"
    } else {
        "unhealthy"
    };

    Ok(Json(json!({
        "status": overall_status,
        "database": db_status,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
