use std::sync::Arc;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use crate::domain::models::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub tenant_id: String,
    pub exp: usize,
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut token_opt: Option<(String, bool)> = None;

    if let Some(auth_header) = req.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                token_opt = Some((token.to_string(), token.starts_with("re-sk-")));
            }
        }
    }

    if token_opt.is_none() {
        if let Some(api_key_header) = req.headers().get("x-api-key") {
            if let Ok(token) = api_key_header.to_str() {
                if token.starts_with("re-sk-") {
                    token_opt = Some((token.to_string(), true));
                }
            }
        }
    }

    match token_opt {
        Some((token, true)) => handle_api_key(&state, &token, req, next).await,
        Some((token, false)) => handle_jwt(&token, req, next).await,
        None => {
            tracing::warn!("Missing or invalid authentication credentials");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

async fn handle_jwt(
    token: &str,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());

    let token_data = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(error) => {
            tracing::warn!("Invalid JWT: {error}");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let tenant_id = Uuid::parse_str(&token_data.claims.tenant_id).unwrap_or_default();
    let user_id = Uuid::parse_str(&token_data.claims.sub).unwrap_or_default();
    req.headers_mut().insert("x-tenant-id", tenant_id.to_string().parse().unwrap());
    req.headers_mut().insert("x-user-id", user_id.to_string().parse().unwrap());

    Ok(next.run(req).await)
}

async fn handle_api_key(
    state: &Arc<AppState>,
    api_key: &str,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let row = sqlx::query("SELECT id FROM tenants WHERE redeye_api_key = $1 AND is_active = true")
        .bind(api_key)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(|error| {
            tracing::error!("DB error during api key lookup: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let tenant_row = match row {
        Some(row) => row,
        None => {
            tracing::warn!("Invalid RedEye API Key");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let tenant_id: Uuid = tenant_row.get("id");
    req.headers_mut().insert("x-tenant-id", tenant_id.to_string().parse().unwrap());

    Ok(next.run(req).await)
}
