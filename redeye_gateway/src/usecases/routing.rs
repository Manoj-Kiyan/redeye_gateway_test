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
