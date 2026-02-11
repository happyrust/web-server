use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
};
use tokio::fs;

/// GET /files/output/{project}/instances/{file}
pub async fn get_project_instances_file(
    Path((project, file)): Path<(String, String)>,
) -> impl IntoResponse {
    let path = std::path::Path::new("output")
        .join(&project)
        .join("instances_cache_for_index")
        .join(&file);
    match fs::read(&path).await {
        Ok(data) => (StatusCode::OK, data).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// GET /files/output/instances/{file}
pub async fn get_root_instances_file(
    Path(file): Path<String>,
) -> impl IntoResponse {
    let path = std::path::Path::new("output")
        .join("instances_cache_for_index")
        .join(&file);
    match fs::read(&path).await {
        Ok(data) => (StatusCode::OK, data).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
