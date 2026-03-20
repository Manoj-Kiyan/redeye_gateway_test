use deadpool_redis::redis::AsyncCommands;

use crate::{
    domain::{models::{AppState, GatewayError}, provider::ProviderKind},
};

pub async fn ensure_closed(
    state: &AppState,
    tenant_id: &str,
    provider: ProviderKind,
) -> Result<(), GatewayError> {
    let key = open_key(tenant_id, provider);
    let mut conn = state
        .redis_pool
        .get()
        .await
        .map_err(|error| GatewayError::ResponseBuild(format!("failed to get redis connection for circuit breaker: {error}")))?;

    let is_open = conn.exists::<_, bool>(&key).await.unwrap_or(false);
    if is_open {
        return Err(GatewayError::CircuitOpen(format!(
            "provider '{}' is temporarily open-circuited for tenant '{}'",
            provider.as_str(),
            tenant_id
        )));
    }

    Ok(())
}

pub async fn record_success(
    state: &AppState,
    tenant_id: &str,
    provider: ProviderKind,
) {
    if let Ok(mut conn) = state.redis_pool.get().await {
        let _: Result<(), _> = conn.del(failure_key(tenant_id, provider)).await;
        let _: Result<(), _> = conn.del(open_key(tenant_id, provider)).await;
    }
}

pub async fn record_failure(
    state: &AppState,
    tenant_id: &str,
    provider: ProviderKind,
) {
    if let Ok(mut conn) = state.redis_pool.get().await {
        let key = failure_key(tenant_id, provider);
        let open_key = open_key(tenant_id, provider);
        let current_count = conn.incr::<_, _, u32>(&key, 1).await.unwrap_or(1);
        let _: Result<(), _> = conn.expire(&key, state.config.circuit_breaker.open_window_secs as i64).await;

        if current_count >= state.config.circuit_breaker.failure_threshold {
            let _: Result<(), _> = conn.set_ex::<_, _, ()>(&open_key, "1", state.config.circuit_breaker.open_window_secs).await;
            let _: Result<(), _> = conn.del(&key).await;
        }
    }
}

fn failure_key(tenant_id: &str, provider: ProviderKind) -> String {
    format!("cb:failures:{tenant_id}:{}", provider.as_str())
}

fn open_key(tenant_id: &str, provider: ProviderKind) -> String {
    format!("cb:open:{tenant_id}:{}", provider.as_str())
}

#[cfg(test)]
mod tests {
    use crate::domain::provider::ProviderKind;

    use super::{failure_key, open_key};

    #[test]
    fn circuit_breaker_keys_are_namespaced() {
        assert_eq!(failure_key("tenant-1", ProviderKind::OpenAi), "cb:failures:tenant-1:openai");
        assert_eq!(open_key("tenant-1", ProviderKind::Gemini), "cb:open:tenant-1:gemini");
    }
}
