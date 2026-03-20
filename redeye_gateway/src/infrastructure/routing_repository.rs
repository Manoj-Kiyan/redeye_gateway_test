use sqlx::Row;
use uuid::Uuid;

use crate::domain::{models::GatewayError, provider::ProviderKind, routing::{TenantRouteConfig, TenantRoutePolicy}};

pub async fn fetch_tenant_routes(
    db_pool: &sqlx::PgPool,
    tenant_id: &str,
) -> Result<Vec<TenantRoutePolicy>, GatewayError> {
    let tenant_uuid = Uuid::parse_str(tenant_id)
        .map_err(|_| GatewayError::Routing(format!("invalid tenant id '{tenant_id}'")))?;

    let rows = sqlx::query(
        "SELECT tenant_id, provider, model, is_default FROM llm_routes WHERE tenant_id = $1 ORDER BY is_default DESC, created_at ASC"
    )
    .bind(tenant_uuid)
    .fetch_all(db_pool)
    .await
    .map_err(|error| GatewayError::ResponseBuild(format!("failed to load tenant routes: {error}")))?;

    rows.into_iter().map(map_row_to_policy).collect()
}

pub async fn fetch_default_tenant_route(
    db_pool: &sqlx::PgPool,
    tenant_id: &str,
) -> Result<Option<TenantRoutePolicy>, GatewayError> {
    let routes = fetch_tenant_routes(db_pool, tenant_id).await?;
    let default_route = routes.into_iter().find(|route| route.is_default);
    Ok(default_route)
}

pub async fn replace_tenant_routes(
    db_pool: &sqlx::PgPool,
    tenant_id: &str,
    routes: &[TenantRouteConfig],
) -> Result<Vec<TenantRoutePolicy>, GatewayError> {
    let tenant_uuid = Uuid::parse_str(tenant_id)
        .map_err(|_| GatewayError::Routing(format!("invalid tenant id '{tenant_id}'")))?;

    let mut tx = db_pool
        .begin()
        .await
        .map_err(|error| GatewayError::ResponseBuild(format!("failed to begin route update transaction: {error}")))?;

    sqlx::query("DELETE FROM llm_routes WHERE tenant_id = $1")
        .bind(tenant_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|error| GatewayError::ResponseBuild(format!("failed to clear tenant routes: {error}")))?;

    for route in routes {
        sqlx::query("INSERT INTO llm_routes (tenant_id, provider, model, is_default) VALUES ($1, $2, $3, $4)")
            .bind(tenant_uuid)
            .bind(route.provider.as_str())
            .bind(&route.model)
            .bind(route.is_default)
            .execute(&mut *tx)
            .await
            .map_err(|error| GatewayError::ResponseBuild(format!("failed to insert tenant route: {error}")))?;
    }

    tx.commit()
        .await
        .map_err(|error| GatewayError::ResponseBuild(format!("failed to commit route update transaction: {error}")))?;

    fetch_tenant_routes(db_pool, tenant_id).await
}

fn map_row_to_policy(row: sqlx::postgres::PgRow) -> Result<TenantRoutePolicy, GatewayError> {
    let provider_str: String = row.get("provider");
    let provider = ProviderKind::from_db_value(&provider_str)
        .ok_or_else(|| GatewayError::Routing(format!("unsupported provider '{provider_str}' in llm_routes")))?;

    Ok(TenantRoutePolicy {
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        requested_model: row.get("model"),
        effective_model: row.get("model"),
        provider,
        is_default: row.get("is_default"),
    })
}
