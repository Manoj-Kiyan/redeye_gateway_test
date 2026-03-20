use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::domain::models::GatewayError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub service: String,
    pub action: String,
    pub target_type: String,
    pub metadata: Value,
    pub actor_user_id: Option<String>,
    pub created_at: String,
}

pub async fn insert_audit_log(
    db_pool: &sqlx::PgPool,
    tenant_id: &str,
    actor_user_id: Option<&str>,
    service: &str,
    action: &str,
    target_type: &str,
    metadata: Value,
) -> Result<(), GatewayError> {
    let tenant_uuid = Uuid::parse_str(tenant_id)
        .map_err(|_| GatewayError::Routing(format!("invalid tenant id '{tenant_id}'")))?;
    let actor_uuid = actor_user_id.and_then(|value| Uuid::parse_str(value).ok());

    sqlx::query(
        "INSERT INTO admin_audit_logs (tenant_id, actor_user_id, service, action, target_type, metadata) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(tenant_uuid)
    .bind(actor_uuid)
    .bind(service)
    .bind(action)
    .bind(target_type)
    .bind(metadata)
    .execute(db_pool)
    .await
    .map_err(|error| GatewayError::ResponseBuild(format!("failed to write audit log: {error}")))?;

    Ok(())
}

pub async fn fetch_tenant_audit_logs(
    db_pool: &sqlx::PgPool,
    tenant_id: &str,
    limit: i64,
) -> Result<Vec<AuditLogEntry>, GatewayError> {
    let tenant_uuid = Uuid::parse_str(tenant_id)
        .map_err(|_| GatewayError::Routing(format!("invalid tenant id '{tenant_id}'")))?;

    let rows = sqlx::query(
        "SELECT id, service, action, target_type, metadata, actor_user_id, created_at
         FROM admin_audit_logs
         WHERE tenant_id = $1
         ORDER BY created_at DESC
         LIMIT $2"
    )
    .bind(tenant_uuid)
    .bind(limit)
    .fetch_all(db_pool)
    .await
    .map_err(|error| GatewayError::ResponseBuild(format!("failed to load audit logs: {error}")))?;

    Ok(rows
        .into_iter()
        .map(|row| AuditLogEntry {
            id: row.get::<Uuid, _>("id").to_string(),
            service: row.get("service"),
            action: row.get("action"),
            target_type: row.get("target_type"),
            metadata: row.get("metadata"),
            actor_user_id: row
                .get::<Option<Uuid>, _>("actor_user_id")
                .map(|value| value.to_string()),
            created_at: row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        })
        .collect())
}
