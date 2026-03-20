use axum::{
    routing::{get, post, put},
    Router,
};
use axum::http::Method;
use tower_http::cors::CorsLayer;
use super::handlers::{signup, login, onboard, refresh, get_provider_status, update_provider_credentials, get_tenant_members, update_member_role, health};
use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse::<axum::http::HeaderValue>().unwrap(),
            "http://localhost:5174".parse::<axum::http::HeaderValue>().unwrap(),
            "http://localhost:5175".parse::<axum::http::HeaderValue>().unwrap(),
            "http://127.0.0.1:5173".parse::<axum::http::HeaderValue>().unwrap(),
            "http://127.0.0.1:5174".parse::<axum::http::HeaderValue>().unwrap(),
            "http://127.0.0.1:5175".parse::<axum::http::HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true);

    Router::new()
        .route("/health", get(health))
        .route("/v1/auth/signup", post(signup))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/onboard", post(onboard))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/auth/providers", get(get_provider_status).post(update_provider_credentials))
        .route("/v1/auth/members", get(get_tenant_members))
        .route("/v1/auth/members/:member_id/role", put(update_member_role))
        .layer(cors)
        .with_state(state)
}
