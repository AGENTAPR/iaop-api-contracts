use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use std::{fs, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::{
    catalog::{build_swagger_config_dto, build_swagger_urls, ServiceCatalog, SwaggerConfigDto},
    config::AppConfig,
    errors::PortalError,
    security::{sanitize_path, security_headers_middleware},
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub catalog: Arc<ServiceCatalog>,
}

/// Builds the complete Axum router for the Central Developer Portal.
pub fn build_router(state: AppState) -> Router {
    let swagger_urls = build_swagger_urls(
        &state.catalog,
        &state.config.environment,
        &state.config.allowed_visibility,
    );

    let swagger_cfg = Config::new(swagger_urls)
        .validator_url("none")
        .query_config_enabled(false)
        .supported_submit_methods::<[&str; 0], &str>([]);

    let swagger_ui = SwaggerUi::new("/swagger-ui").config(swagger_cfg);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(root_redirect))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/api/v1/catalog", get(get_catalog))
        .route("/api/v1/swagger-config", get(get_swagger_config))
        .route("/specs/{*spec_path}", get(serve_spec_file))
        .merge(swagger_ui)
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(cors)
        .with_state(state)
}

async fn root_redirect() -> impl IntoResponse {
    Redirect::temporary("/swagger-ui/")
}

async fn health_live() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn health_ready(State(state): State<AppState>) -> impl IntoResponse {
    let service_count = state.catalog.services.len();
    Json(serde_json::json!({
        "status": "ready",
        "services_count": service_count,
        "environment": state.config.environment,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn get_catalog(State(state): State<AppState>) -> impl IntoResponse {
    Json((*state.catalog).clone())
}

async fn get_swagger_config(State(state): State<AppState>) -> impl IntoResponse {
    let dto: SwaggerConfigDto = build_swagger_config_dto(
        &state.catalog,
        &state.config.environment,
        &state.config.allowed_visibility,
    );
    Json(dto)
}

async fn serve_spec_file(
    State(state): State<AppState>,
    Path(spec_path): Path<String>,
) -> Result<Response, PortalError> {
    let req_path = std::path::Path::new(&spec_path);
    let resolved_path = sanitize_path(&state.config.specs_dir, req_path)?;

    let content = fs::read(&resolved_path)
        .map_err(|e| PortalError::Internal(format!("Failed to read spec file: {}", e)))?;

    let content_type = if spec_path.ends_with(".yaml") || spec_path.ends_with(".yml") {
        "application/yaml"
    } else {
        "application/json"
    };

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300, must-revalidate"),
    );

    Ok((StatusCode::OK, headers, content).into_response())
}
