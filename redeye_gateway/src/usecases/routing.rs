use axum::http::HeaderMap;
use serde_json::Value;

use crate::{
    config::GatewayConfig,
    domain::{
        models::{AppState, GatewayError},
        provider::ProviderKind,
        routing::{RoutingDecision, TenantRoutePolicy},
    },
    infrastructure::{credential_repository, routing_repository},
};

pub async fn resolve_routing_decision(
    state: &AppState,
    headers: &HeaderMap,
    body: &Value,
) -> Result<RoutingDecision, GatewayError> {
    let requested_model = body
        .get("model")
        .and_then(|value| value.as_str())
        .ok_or_else(|| GatewayError::InvalidRequest("model is required".to_string()))?
        .to_string();

    let tenant_id = headers
        .get("x-tenant-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();

    let tenant_routes = load_tenant_routes(state, &tenant_id).await?;
    let selected_policy = select_route_policy(&state.config, &tenant_id, &requested_model, &tenant_routes)?;
    let upstream_api_key = credential_repository::fetch_provider_api_key(state, &tenant_id, selected_policy.provider).await?;

    Ok(RoutingDecision {
        provider: selected_policy.provider,
        requested_model,
        effective_model: selected_policy.effective_model,
        tenant_id,
        upstream_api_key,
    })
}

pub async fn resolve_fallback_routing_decision(
    state: &AppState,
    tenant_id: &str,
    failed_provider: ProviderKind,
) -> Result<Option<RoutingDecision>, GatewayError> {
    if tenant_id == "anonymous" {
        return Ok(None);
    }

    let default_route = routing_repository::fetch_default_tenant_route(&state.db_pool, tenant_id).await?;
    let Some(default_route) = default_route else {
        return Ok(None);
    };

    if default_route.provider == failed_provider {
        return Ok(None);
    }

    let upstream_api_key = match credential_repository::fetch_provider_api_key(state, tenant_id, default_route.provider).await {
        Ok(api_key) => api_key,
        Err(_) => return Ok(None),
    };

    Ok(Some(RoutingDecision {
        provider: default_route.provider,
        requested_model: default_route.requested_model.clone(),
        effective_model: default_route.effective_model,
        tenant_id: tenant_id.to_string(),
        upstream_api_key,
    }))
}

async fn load_tenant_routes(state: &AppState, tenant_id: &str) -> Result<Vec<TenantRoutePolicy>, GatewayError> {
    if tenant_id == "anonymous" {
        return Ok(Vec::new());
    }

    routing_repository::fetch_tenant_routes(&state.db_pool, tenant_id).await
}

fn select_route_policy(
    config: &GatewayConfig,
    tenant_id: &str,
    requested_model: &str,
    tenant_routes: &[TenantRoutePolicy],
) -> Result<TenantRoutePolicy, GatewayError> {
    if let Some(route) = tenant_routes.iter().find(|route| route.requested_model == requested_model) {
        return Ok(route.clone());
    }

    if tenant_routes.is_empty() {
        let provider = infer_provider(requested_model).unwrap_or(config.providers.default_provider);

        if !provider.supports_model(requested_model) {
            return Err(GatewayError::Routing(format!(
                "model '{requested_model}' is not supported by provider '{}'",
                provider.as_str()
            )));
        }

        return Ok(TenantRoutePolicy {
            tenant_id: tenant_id.to_string(),
            requested_model: requested_model.to_string(),
            effective_model: requested_model.to_string(),
            provider,
            is_default: true,
        });
    }

    let allowed_models = tenant_routes
        .iter()
        .map(|route| route.requested_model.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    Err(GatewayError::Routing(format!(
        "model '{requested_model}' is not configured for tenant '{tenant_id}'. Allowed models: {allowed_models}"
    )))
}

fn infer_provider(model: &str) -> Option<ProviderKind> {
    [ProviderKind::OpenAi, ProviderKind::Anthropic, ProviderKind::Gemini]
        .into_iter()
        .find(|provider| provider.supports_model(model))
}

#[cfg(test)]
mod tests {
    use super::{infer_provider, select_route_policy};
    use crate::{
        config::{CircuitBreakerConfig, GatewayConfig, ProviderRegistry, RateLimitConfig, ServiceUrls},
        domain::{provider::ProviderKind, routing::TenantRoutePolicy},
    };

    fn test_config() -> GatewayConfig {
        GatewayConfig {
            port: 8080,
            database_url: "postgres://local".to_string(),
            redis_url: "redis://local".to_string(),
            service_urls: ServiceUrls {
                cache_url: "http://localhost:8081".to_string(),
                clickhouse_url: "http://localhost:8123".to_string(),
                tracer_url: "http://localhost:8082".to_string(),
            },
            rate_limit: RateLimitConfig {
                max_requests: 60,
                window_secs: 60,
            },
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 3,
                open_window_secs: 30,
            },
            providers: ProviderRegistry {
                default_provider: ProviderKind::OpenAi,
                openai_api_key: "dummy".to_string(),
                anthropic_api_key: None,
                gemini_api_key: None,
            },
        }
    }

    #[test]
    fn infer_provider_uses_model_prefixes() {
        assert_eq!(infer_provider("gpt-4o"), Some(ProviderKind::OpenAi));
        assert_eq!(infer_provider("claude-3-5-sonnet-latest"), Some(ProviderKind::Anthropic));
        assert_eq!(infer_provider("gemini-1.5-pro"), Some(ProviderKind::Gemini));
        assert_eq!(infer_provider("unknown-model"), None);
    }

    #[test]
    fn select_route_policy_prefers_explicit_tenant_route() {
        let config = test_config();
        let routes = vec![TenantRoutePolicy {
            tenant_id: "tenant-1".to_string(),
            requested_model: "gpt-4o-mini".to_string(),
            effective_model: "gpt-4o-mini".to_string(),
            provider: ProviderKind::OpenAi,
            is_default: true,
        }];

        let selected = select_route_policy(&config, "tenant-1", "gpt-4o-mini", &routes).unwrap();
        assert_eq!(selected.provider, ProviderKind::OpenAi);
        assert_eq!(selected.requested_model, "gpt-4o-mini");
    }

    #[test]
    fn select_route_policy_rejects_unconfigured_tenant_model() {
        let config = test_config();
        let routes = vec![TenantRoutePolicy {
            tenant_id: "tenant-1".to_string(),
            requested_model: "gpt-4o-mini".to_string(),
            effective_model: "gpt-4o-mini".to_string(),
            provider: ProviderKind::OpenAi,
            is_default: true,
        }];

        let error = select_route_policy(&config, "tenant-1", "claude-3-5-sonnet-latest", &routes)
            .expect_err("expected unconfigured model to fail");

        match error {
            crate::domain::models::GatewayError::Routing(message) => {
                assert!(message.contains("Allowed models"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn select_route_policy_uses_inferred_provider_when_no_routes_exist() {
        let config = test_config();
        let selected = select_route_policy(&config, "tenant-1", "gemini-1.5-pro", &[]).unwrap();
        assert_eq!(selected.provider, ProviderKind::Gemini);
        assert!(selected.is_default);
    }
}
