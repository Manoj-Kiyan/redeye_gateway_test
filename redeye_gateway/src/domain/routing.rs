use serde::{Deserialize, Serialize};

use crate::domain::provider::ProviderKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRoutePolicy {
    pub tenant_id: String,
    pub requested_model: String,
    pub effective_model: String,
    pub provider: ProviderKind,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub provider: ProviderKind,
    pub requested_model: String,
    pub effective_model: String,
    pub tenant_id: String,
    pub upstream_api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRouteConfig {
    pub provider: ProviderKind,
    pub model: String,
    pub is_default: bool,
}
