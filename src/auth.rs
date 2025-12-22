use axum::http::{HeaderMap, header};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

pub fn check_auth(headers: &HeaderMap) -> bool {
    headers.get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(|s| s.contains("auth_session=true"))
        .unwrap_or(false)
}