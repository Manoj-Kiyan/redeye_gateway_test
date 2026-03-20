use axum::{extract::{Path, State}, Json, http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE}};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{AppState, error::AppError, infrastructure::security::{hash_password, verify_password, generate_jwt, encrypt_api_key, generate_redeye_api_key, verify_jwt, generate_refresh_token}};
use uuid::Uuid;
use sqlx::Row;

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub company_name: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub id: Uuid,
    pub email: String,
    pub tenant_id: Uuid,
    pub workspace_name: String,
    pub onboarding_complete: bool,
    pub token: String,
    pub redeye_api_key: Option<String>,
    pub role: String,
}

#[derive(Serialize)]
pub struct ProviderStatusResponse {
    pub openai_configured: bool,
    pub anthropic_configured: bool,
    pub gemini_configured: bool,
    pub redeye_api_key: Option<String>,
    pub workspace_name: String,
}

#[derive(Deserialize)]
pub struct UpdateProviderCredentialsRequest {
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
}

#[derive(Serialize)]
pub struct TenantMember {
    pub id: Uuid,
    pub email: String,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct TenantMembersResponse {
    pub members: Vec<TenantMember>,
}

#[derive(Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "redeye_auth",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<SignupRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let hashed_pw = hash_password(&payload.password)?;

    let mut tx = state.db_pool.begin().await?;

    let tenant_id: Uuid = sqlx::query("INSERT INTO tenants (name) VALUES ($1) RETURNING id")
        .bind(&payload.company_name)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.constraint() == Some("tenants_name_key") {
                    return AppError::BadRequest("Company name already exists".into());
                }
            }
            AppError::from(e)
        })?
        .get("id");

    let user_role = "owner";
    let user_id: Uuid = sqlx::query("INSERT INTO users (email, password_hash, tenant_id, role) VALUES ($1, $2, $3, $4) RETURNING id")
        .bind(&payload.email)
        .bind(&hashed_pw)
        .bind(tenant_id)
        .bind(user_role)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.constraint() == Some("users_email_key") {
                    return AppError::BadRequest("Email already exists".into());
                }
            }
            AppError::from(e)
        })?
        .get("id");

    tx.commit().await?;

    let token = generate_jwt(user_id, tenant_id, user_role)?;
    let refresh_token = generate_refresh_token(user_id, tenant_id, user_role)?;

    let cookie = format!("refresh_token={}; HttpOnly; Path=/; Max-Age=604800; SameSite=Strict", refresh_token);
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());

    Ok((headers, Json(AuthResponse {
        id: user_id,
        email: payload.email,
        tenant_id,
        workspace_name: payload.company_name,
        onboarding_complete: false,
        token,
        redeye_api_key: None,
        role: user_role.to_string(),
    })))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let row = sqlx::query("SELECT u.id, u.password_hash, u.tenant_id, u.role, t.name as workspace_name, t.onboarding_status, t.redeye_api_key FROM users u JOIN tenants t ON u.tenant_id = t.id WHERE u.email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.db_pool)
        .await?;

    let user_row = match row {
        Some(r) => r,
        None => return Err(AppError::Unauthorized("Invalid email or password".into())),
    };

    let p_hash: String = user_row.get("password_hash");
    let is_valid = verify_password(&p_hash, &payload.password)?;
    if !is_valid {
        return Err(AppError::Unauthorized("Invalid email or password".into()));
    }

    let user_id: Uuid = user_row.get("id");
    let tenant_id: Uuid = user_row.get("tenant_id");
    let role: String = user_row.get("role");
    let workspace_name: String = user_row.get("workspace_name");
    let onboarding_complete: bool = user_row.get("onboarding_status");
    let redeye_api_key: Option<String> = user_row.get("redeye_api_key");

    let token = generate_jwt(user_id, tenant_id, &role)?;
    let refresh_token = generate_refresh_token(user_id, tenant_id, &role)?;

    let cookie = format!("refresh_token={}; HttpOnly; Path=/; Max-Age=604800; SameSite=Strict", refresh_token);
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());

    Ok((headers, Json(AuthResponse {
        id: user_id,
        email: payload.email,
        tenant_id,
        workspace_name,
        onboarding_complete,
        token,
        redeye_api_key,
        role,
    })))
}

pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthResponse>, AppError> {
    let claims = extract_claims_from_cookie(&headers)?;
    let user_id = Uuid::parse_str(&claims.sub).unwrap_or_default();
    let tenant_id = Uuid::parse_str(&claims.tenant_id).unwrap_or_default();
    let token = generate_jwt(user_id, tenant_id, &claims.role)?;

    let email: String = sqlx::query("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db_pool)
        .await?
        .get("email");

    let row = sqlx::query("SELECT name, onboarding_status, redeye_api_key FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(&state.db_pool)
        .await?;

    Ok(Json(AuthResponse {
        id: user_id,
        email,
        tenant_id,
        workspace_name: row.get("name"),
        onboarding_complete: row.get("onboarding_status"),
        token,
        redeye_api_key: row.get("redeye_api_key"),
        role: claims.role,
    }))
}

#[derive(Deserialize)]
pub struct OnboardRequest {
    pub openai_api_key: Option<String>,
    pub workspace_name: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
}

pub async fn onboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<OnboardRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let claims = extract_claims_from_bearer(&headers)?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|_| AppError::Internal("Invalid tenant ID in token".into()))?;
    let user_id = Uuid::parse_str(&claims.sub).unwrap_or_default();
    ensure_minimum_role(&claims.role, "admin")?;

    let mut tx = state.db_pool.begin().await?;
    let redeye_api_key = current_or_new_redeye_api_key(&state.db_pool, tenant_id).await?;

    sqlx::query("UPDATE tenants SET redeye_api_key = $1, onboarding_status = true WHERE id = $2")
        .bind(&redeye_api_key)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await?;

    let openai_configured = if let Some(api_key) = payload.openai_api_key.as_deref().filter(|value| !value.trim().is_empty()) {
        let encrypted_openai_key = encrypt_api_key(api_key)?;
        sqlx::query("UPDATE tenants SET encrypted_openai_key = $1 WHERE id = $2")
            .bind(&encrypted_openai_key)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await?;
        upsert_provider_credential(&mut tx, tenant_id, "openai", api_key).await?;
        true
    } else {
        false
    };

    let anthropic_configured = if let Some(api_key) = payload.anthropic_api_key.as_deref().filter(|value| !value.trim().is_empty()) {
        upsert_provider_credential(&mut tx, tenant_id, "anthropic", api_key).await?;
        true
    } else {
        false
    };

    let gemini_configured = if let Some(api_key) = payload.gemini_api_key.as_deref().filter(|value| !value.trim().is_empty()) {
        upsert_provider_credential(&mut tx, tenant_id, "gemini", api_key).await?;
        true
    } else {
        false
    };

    let final_workspace_name = if let Some(ws_name) = &payload.workspace_name {
        sqlx::query("UPDATE tenants SET name = $1 WHERE id = $2")
            .bind(ws_name)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await?;
        ws_name.clone()
    } else {
        sqlx::query("SELECT name FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_one(&mut *tx)
            .await?
            .get("name")
    };

    tx.commit().await?;

    write_audit_log(
        &state.db_pool,
        tenant_id,
        Some(user_id),
        "auth",
        "tenant_onboarded",
        "provider_credentials",
        json!({
            "workspace_name": final_workspace_name,
            "providers": {
                "openai": openai_configured,
                "anthropic": anthropic_configured,
                "gemini": gemini_configured,
            }
        }),
    ).await?;

    let email: String = sqlx::query("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db_pool)
        .await?
        .get("email");

    Ok(Json(AuthResponse {
        id: user_id,
        email,
        tenant_id,
        workspace_name: final_workspace_name,
        onboarding_complete: true,
        token: generate_jwt(user_id, tenant_id, &claims.role)?,
        redeye_api_key: Some(redeye_api_key),
        role: claims.role,
    }))
}

pub async fn get_provider_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ProviderStatusResponse>, AppError> {
    let claims = extract_claims_from_bearer(&headers)?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|_| AppError::Internal("Invalid tenant ID in token".into()))?;

    let tenant_row = sqlx::query("SELECT name, redeye_api_key, onboarding_status FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(&state.db_pool)
        .await?;

    let credential_rows = sqlx::query("SELECT provider FROM provider_credentials WHERE tenant_id = $1")
        .bind(tenant_id)
        .fetch_all(&state.db_pool)
        .await?;

    let mut openai_configured = false;
    let mut anthropic_configured = false;
    let mut gemini_configured = false;

    for row in credential_rows {
        let provider: String = row.get("provider");
        match provider.as_str() {
            "openai" => openai_configured = true,
            "anthropic" => anthropic_configured = true,
            "gemini" => gemini_configured = true,
            _ => {}
        }
    }

    Ok(Json(ProviderStatusResponse {
        openai_configured: openai_configured || tenant_row.get::<bool, _>("onboarding_status"),
        anthropic_configured,
        gemini_configured,
        redeye_api_key: tenant_row.get("redeye_api_key"),
        workspace_name: tenant_row.get("name"),
    }))
}

pub async fn update_provider_credentials(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateProviderCredentialsRequest>,
) -> Result<Json<ProviderStatusResponse>, AppError> {
    let claims = extract_claims_from_bearer(&headers)?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|_| AppError::Internal("Invalid tenant ID in token".into()))?;
    let actor_user_id = Uuid::parse_str(&claims.sub).ok();
    ensure_minimum_role(&claims.role, "admin")?;

    let mut tx = state.db_pool.begin().await?;
    let mut updated_providers = Vec::new();

    if let Some(api_key) = payload.openai_api_key.as_deref().filter(|value| !value.trim().is_empty()) {
        let encrypted_openai_key = encrypt_api_key(api_key)?;
        sqlx::query("UPDATE tenants SET encrypted_openai_key = $1, onboarding_status = true WHERE id = $2")
            .bind(encrypted_openai_key)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await?;
        upsert_provider_credential(&mut tx, tenant_id, "openai", api_key).await?;
        updated_providers.push("openai");
    }

    if let Some(api_key) = payload.anthropic_api_key.as_deref().filter(|value| !value.trim().is_empty()) {
        upsert_provider_credential(&mut tx, tenant_id, "anthropic", api_key).await?;
        updated_providers.push("anthropic");
    }

    if let Some(api_key) = payload.gemini_api_key.as_deref().filter(|value| !value.trim().is_empty()) {
        upsert_provider_credential(&mut tx, tenant_id, "gemini", api_key).await?;
        updated_providers.push("gemini");
    }

    tx.commit().await?;

    if !updated_providers.is_empty() {
        write_audit_log(
            &state.db_pool,
            tenant_id,
            actor_user_id,
            "auth",
            "provider_credentials_updated",
            "provider_credentials",
            json!({
                "updated_providers": updated_providers,
            }),
        ).await?;
    }

    get_provider_status(State(state), headers).await
}

pub async fn get_tenant_members(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TenantMembersResponse>, AppError> {
    let claims = extract_claims_from_bearer(&headers)?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|_| AppError::Internal("Invalid tenant ID in token".into()))?;
    ensure_minimum_role(&claims.role, "admin")?;

    let rows = sqlx::query("SELECT id, email, role, created_at FROM users WHERE tenant_id = $1 ORDER BY created_at ASC")
        .bind(tenant_id)
        .fetch_all(&state.db_pool)
        .await?;

    let members = rows.into_iter().map(|row| TenantMember {
        id: row.get("id"),
        email: row.get("email"),
        role: row.get("role"),
        created_at: row.get("created_at"),
    }).collect();

    Ok(Json(TenantMembersResponse { members }))
}

pub async fn update_member_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(member_id): Path<Uuid>,
    Json(payload): Json<UpdateMemberRoleRequest>,
) -> Result<StatusCode, AppError> {
    let claims = extract_claims_from_bearer(&headers)?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|_| AppError::Internal("Invalid tenant ID in token".into()))?;
    let actor_user_id = Uuid::parse_str(&claims.sub).ok();
    ensure_minimum_role(&claims.role, "owner")?;
    ensure_valid_role(&payload.role)?;

    sqlx::query("UPDATE users SET role = $1 WHERE id = $2 AND tenant_id = $3")
        .bind(&payload.role)
        .bind(member_id)
        .bind(tenant_id)
        .execute(&state.db_pool)
        .await?;

    write_audit_log(
        &state.db_pool,
        tenant_id,
        actor_user_id,
        "auth",
        "member_role_updated",
        "users",
        json!({
            "member_id": member_id,
            "role": payload.role,
        }),
    ).await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn upsert_provider_credential(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: Uuid,
    provider: &str,
    api_key: &str,
) -> Result<(), AppError> {
    let encrypted_key = encrypt_api_key(api_key)?;

    sqlx::query("INSERT INTO provider_credentials (tenant_id, provider, encrypted_api_key, is_primary) VALUES ($1, $2, $3, true) ON CONFLICT (tenant_id, provider) DO UPDATE SET encrypted_api_key = EXCLUDED.encrypted_api_key, updated_at = NOW(), is_primary = true")
        .bind(tenant_id)
        .bind(provider)
        .bind(encrypted_key)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

async fn current_or_new_redeye_api_key(
    db_pool: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<String, AppError> {
    let existing_key = sqlx::query("SELECT redeye_api_key FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(db_pool)
        .await?
        .get::<Option<String>, _>("redeye_api_key");

    Ok(existing_key.unwrap_or_else(generate_redeye_api_key))
}

fn extract_claims_from_bearer(headers: &HeaderMap) -> Result<crate::infrastructure::security::Claims, AppError> {
    let auth_header = headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|val| val.to_str().ok())
        .and_then(|val| val.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::Unauthorized("Missing or invalid Authorization header".into()))?;

    verify_jwt(auth_header)
}

fn extract_claims_from_cookie(headers: &HeaderMap) -> Result<crate::infrastructure::security::Claims, AppError> {
    let cookie_header = headers.get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing refresh token cookie".into()))?;

    let refresh_token = cookie_header.split(';')
        .map(|s| s.trim())
        .find(|s| s.starts_with("refresh_token="))
        .map(|s| &s["refresh_token=".len()..])
        .ok_or_else(|| AppError::Unauthorized("Refresh token cookie not found".into()))?;

    verify_jwt(refresh_token)
}

fn ensure_valid_role(role: &str) -> Result<(), AppError> {
    match role {
        "owner" | "admin" | "viewer" => Ok(()),
        _ => Err(AppError::BadRequest("Invalid role. Allowed values: owner, admin, viewer".into())),
    }
}

fn ensure_minimum_role(current_role: &str, minimum_role: &str) -> Result<(), AppError> {
    let rank = |role: &str| match role {
        "owner" => 3,
        "admin" => 2,
        "viewer" => 1,
        _ => 0,
    };

    if rank(current_role) < rank(minimum_role) {
        return Err(AppError::Unauthorized(format!("{} role required", minimum_role)));
    }

    Ok(())
}

async fn write_audit_log(
    db_pool: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_user_id: Option<Uuid>,
    service: &str,
    action: &str,
    target_type: &str,
    metadata: serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO admin_audit_logs (tenant_id, actor_user_id, service, action, target_type, metadata) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(tenant_id)
    .bind(actor_user_id)
    .bind(service)
    .bind(action)
    .bind(target_type)
    .bind(metadata)
    .execute(db_pool)
    .await?;

    Ok(())
}
