use deadpool_redis::redis::AsyncCommands;
use sqlx::Row;
use uuid::Uuid;

use crate::domain::{models::{AppState, GatewayError}, provider::ProviderKind};

pub async fn fetch_provider_api_key(
    state: &AppState,
    tenant_id: &str,
    provider: ProviderKind,
) -> Result<String, GatewayError> {
    if tenant_id == "anonymous" {
        return fallback_service_key(state, provider);
    }

    let cache_key = format!("tenant_provider_key:{tenant_id}:{}", provider.as_str());

    if let Ok(mut conn) = state.redis_pool.get().await {
        if let Ok(cached_key) = conn.get::<_, String>(&cache_key).await {
            return Ok(cached_key);
        }
    }

    let tenant_uuid = Uuid::parse_str(tenant_id)
        .map_err(|_| GatewayError::Routing(format!("invalid tenant id '{tenant_id}'")))?;

    let row = sqlx::query(
        "SELECT encrypted_api_key FROM provider_credentials WHERE tenant_id = $1 AND provider = $2 AND is_primary = true"
    )
    .bind(tenant_uuid)
    .bind(provider.as_str())
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| GatewayError::ResponseBuild(format!("failed to load provider credential: {error}")))?;

    let api_key = if let Some(row) = row {
        let encrypted_data: Vec<u8> = row.get("encrypted_api_key");
        decrypt_api_key(&encrypted_data)?
    } else if provider == ProviderKind::OpenAi {
        fetch_openai_fallback_from_tenants(state, tenant_uuid).await?
    } else {
        fallback_service_key(state, provider)?
    };

    if let Ok(mut conn) = state.redis_pool.get().await {
        let _: Result<(), _> = conn.set_ex(&cache_key, &api_key, 300).await;
    }

    Ok(api_key)
}

async fn fetch_openai_fallback_from_tenants(
    state: &AppState,
    tenant_id: Uuid,
) -> Result<String, GatewayError> {
    let row = sqlx::query(
        "SELECT encrypted_openai_key FROM tenants WHERE id = $1 AND onboarding_status = true"
    )
    .bind(tenant_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|error| GatewayError::ResponseBuild(format!("failed to load legacy openai credential: {error}")))?;

    let encrypted_data: Vec<u8> = row
        .and_then(|row| row.get::<Option<Vec<u8>>, _>("encrypted_openai_key"))
        .ok_or_else(|| GatewayError::Routing("tenant provider credential not found".to_string()))?;

    decrypt_api_key(&encrypted_data)
}

fn fallback_service_key(state: &AppState, provider: ProviderKind) -> Result<String, GatewayError> {
    match provider {
        ProviderKind::OpenAi => Ok(state.config.providers.openai_api_key.clone()),
        ProviderKind::Anthropic => state
            .config
            .providers
            .anthropic_api_key
            .clone()
            .ok_or_else(|| GatewayError::Routing("ANTHROPIC_API_KEY is not configured".to_string())),
        ProviderKind::Gemini => state
            .config
            .providers
            .gemini_api_key
            .clone()
            .ok_or_else(|| GatewayError::Routing("GEMINI_API_KEY is not configured".to_string())),
    }
}

fn decrypt_api_key(encrypted_data: &[u8]) -> Result<String, GatewayError> {
    use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};

    let master_key = std::env::var("AES_MASTER_KEY")
        .map_err(|_| GatewayError::ResponseBuild("AES_MASTER_KEY missing".to_string()))?;

    if master_key.len() != 32 || encrypted_data.len() < 12 {
        return Err(GatewayError::ResponseBuild("invalid encrypted provider credential".to_string()));
    }

    let key = Key::<Aes256Gcm>::from_slice(master_key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| GatewayError::ResponseBuild("failed to decrypt provider credential".to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|_| GatewayError::ResponseBuild("provider credential is not valid utf-8".to_string()))
}
