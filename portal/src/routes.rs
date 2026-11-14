use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::{fs, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use utoipa_swagger_ui::{oauth, Config, SwaggerUi};

use crate::{
    auth::{self, SharedStore},
    catalog::{build_swagger_config_dto, build_swagger_urls, ServiceCatalog, SwaggerConfigDto},
    config::AppConfig,
    errors::PortalError,
    proxy::proxy_router,
    security::{sanitize_path, security_headers_middleware},
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub catalog: Arc<ServiceCatalog>,
    pub auth_store: SharedStore,
}

/// Services whose OpenAPI docs get an injected Keycloak OAuth2 PKCE
/// security scheme for Scalar/Swagger "Try it out" testing.
/// BFF specs are excluded: they use server-side session cookies + CSRF.
fn is_backend_api(spec_path: &str) -> bool {
    let p = spec_path.to_lowercase();
    p.contains("-service") || p.contains("integration-service") || p.contains("notification-service")
}

fn audience_scope_for(spec_path: &str) -> Option<&'static str> {
    let p = spec_path.to_lowercase();
    if p.contains("agent-service") {
        Some("aud-agent")
    } else if p.contains("agency-service") {
        Some("aud-agency")
    } else if p.contains("eligibility-service") {
        Some("aud-eligibility")
    } else {
        None
    }
}

/// Inject a Keycloak OAuth2 (authorization code + PKCE) security scheme into
/// a backend OpenAPI document so Swagger UI exposes an Authorize flow.
fn inject_oauth_security(spec: &[u8], spec_path: &str, issuer: &str) -> Vec<u8> {
    if !is_backend_api(spec_path) {
        return spec.to_vec();
    }
    let Ok(mut doc) = serde_json::from_slice::<serde_json::Value>(spec) else {
        return spec.to_vec();
    };

    let mut scopes = serde_json::Map::new();
    scopes.insert("openid".into(), serde_json::json!("OpenID Connect"));
    if let Some(aud) = audience_scope_for(spec_path) {
        scopes.insert(aud.into(), serde_json::json!(format!("Token audience for {aud}")));
    }

    let scheme = serde_json::json!({
        "type": "oauth2",
        "description": "Keycloak Authorization Code + PKCE (public client iaop-scalar-dev)",
        "flows": {
            "authorizationCode": {
                "authorizationUrl": format!("{issuer}/protocol/openid-connect/auth"),
                "tokenUrl": format!("{issuer}/protocol/openid-connect/token"),
                "scopes": scopes,
            }
        }
    });

    if let Some(obj) = doc.as_object_mut() {
        let comps = obj
            .entry("components")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(c) = comps.as_object_mut() {
            let schemes = c
                .entry("securitySchemes")
                .or_insert_with(|| serde_json::json!({}));
            if let Some(s) = schemes.as_object_mut() {
                s.insert("keycloakOAuth".into(), scheme);
            }
        }
        obj.insert(
            "security".into(),
            serde_json::json!([{ "keycloakOAuth": [] }]),
        );
    }
    serde_json::to_vec(&doc).unwrap_or_else(|_| spec.to_vec())
}

/// Builds the complete Axum router for the Central Developer Portal.
pub fn build_router(state: AppState) -> Router {
    let swagger_urls = build_swagger_urls(
        &state.catalog,
        &state.config.environment,
        &state.config.allowed_visibility,
    );

    let oauth_cfg = oauth::Config::new()
        .client_id("iaop-scalar-dev")
        .use_pkce_with_authorization_code_grant(true)
        .scope_separator(" ")
        .scopes(vec!["openid".to_string()]);

    let swagger_cfg = Config::new(swagger_urls)
        .validator_url("none")
        .query_config_enabled(false)
        .supported_submit_methods::<[&str; 0], &str>([]);

    let swagger_ui = SwaggerUi::new("/swagger-ui")
        .config(swagger_cfg)
        .oauth(oauth_cfg);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes: health, OIDC flow, proxies (they enforce their own auth).
    let public = Router::new()
        .route("/", get(root_page))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/auth/login", get(auth::login))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/session", get(auth::session_info))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout));

    // Protected routes: catalog + specs require a portal session.
    let protected = Router::new()
        .route("/api/v1/catalog", get(get_catalog))
        .route("/api/v1/swagger-config", get(get_swagger_config))
        .route("/specs/{*spec_path}", get(serve_spec_file))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_session,
        ));

    Router::new()
        .merge(public)
        .merge(protected)
        .merge(proxy_router())
        .merge(swagger_ui)
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(cors)
        .with_state(state)
}

async fn root_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authed = auth::session_from_headers(&state, &headers).is_some();
    let oidc_cfgured = state.config.oidc_issuer.is_some();
    let login = if authed {
        String::new()
    } else if oidc_cfgured {
        r#"<a class="btn" href="/auth/login">Login with Keycloak</a>"#.to_string()
    } else {
        r#"<span class="muted">SSO not configured</span>"#.to_string()
    };
    let logout = if authed {
        r#"<form method="post" action="/auth/logout" style="display:inline"><button class="btn ghost" type="submit">Logout</button></form>"#.to_string()
    } else {
        String::new()
    };
    let html = format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>IAOP Developer Portal</title>
<style>body{{font-family:system-ui,sans-serif;max-width:900px;margin:3rem auto;padding:0 1rem;color:#1a1a2e}}
.card{{border:1px solid #ddd;border-radius:8px;padding:1.5rem;margin:1rem 0}}
a{{color:#0b5fff}} .btn{{background:#0b5fff;color:#fff;padding:.5rem 1rem;border-radius:6px;text-decoration:none;border:0;cursor:pointer}}
.ghost{{background:#eee;color:#333}} .muted{{color:#888}} code{{background:#f4f4f4;padding:.1rem .3rem;border-radius:4px}}</style></head><body>
<h1>IAOP Developer Portal</h1>
<div class="card"><strong>Session:</strong> <span id="who">{}</span> {} {}</div>
<div class="card"><h3>API Access</h3>
<p><a href="/swagger-ui/">Backend API Catalog (Swagger UI)</a> — OAuth2 + PKCE via Keycloak client <code>iaop-scalar-dev</code></p>
<p><strong>Frontend APIs</strong> — Agent BFF session testing via same-origin proxy <code>/agent-bff/*</code> (connect at <code>/agent-bff/auth/login</code>)</p>
<p><strong>Domain APIs</strong> — bearer-token testing via <code>/dev-api/&lt;service&gt;/*</code> with audience-scoped tokens</p>
</div>
<script>
fetch('/auth/session').then(r=>r.json()).then(s=>{{
  document.getElementById('who').textContent = s.authenticated ? ('Signed in as '+(s.displayName||s.subject)) : 'Anonymous';
}}).catch(()=>{{}});
</script></body></html>"#,
        if authed { "authenticated" } else { "anonymous" },
        login, logout
    );
    Html(html).into_response()
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

    let is_yaml = spec_path.ends_with(".yaml") || spec_path.ends_with(".yml");
    let content = if !is_yaml {
        if let Some(issuer) = &state.config.oidc_issuer {
            inject_oauth_security(&content, &spec_path, issuer)
        } else {
            content
        }
    } else {
        content
    };

    let content_type = if is_yaml {
        "application/yaml"
    } else {
        "application/json"
    };

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=60, must-revalidate"),
    );

    Ok((StatusCode::OK, headers, content).into_response())
}
