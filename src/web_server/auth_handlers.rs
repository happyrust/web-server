use axum::{Json, http::StatusCode};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub project: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub expires_in: i64, // seconds
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // username
    pub project: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

/// Generate a JWT token for the given user, project, and role.
///
/// The token is valid for 24 hours.
/// The secret key is read from the AUTH_SECRET environment variable,
/// defaulting to "default_secret_key_change_me" if not set.
pub async fn generate_token(
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    // In a real production environment, ensure AUTH_SECRET is set and strong.
    let secret =
        env::var("AUTH_SECRET").unwrap_or_else(|_| "default_secret_key_change_me".to_string());

    let now = Utc::now();
    let duration_hours = 24;
    let expiration = now
        .checked_add_signed(Duration::hours(duration_hours))
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .timestamp();

    let claims = Claims {
        sub: payload.username,
        project: payload.project,
        role: payload.role,
        exp: expiration as usize,
        iat: now.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| {
        eprintln!("Token generation error: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(AuthResponse {
        token,
        expires_in: duration_hours * 3600,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{DecodingKey, Validation, decode};

    #[tokio::test]
    async fn test_generate_token() {
        let req = AuthRequest {
            username: "testuser".to_string(),
            project: "testproj".to_string(),
            role: "admin".to_string(),
        };

        let response = generate_token(Json(req))
            .await
            .expect("Handler should succeed");
        let AuthResponse { token, expires_in } = response.0;

        assert_eq!(expires_in, 3600 * 24);

        let secret =
            env::var("AUTH_SECRET").unwrap_or_else(|_| "default_secret_key_change_me".to_string());
        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .expect("Token should be valid");

        assert_eq!(token_data.claims.sub, "testuser");
        assert_eq!(token_data.claims.project, "testproj");
        assert_eq!(token_data.claims.role, "admin");
    }
}
