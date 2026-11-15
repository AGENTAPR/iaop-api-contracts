use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use std::{fs, sync::Arc};
use tower_http::cors::{Any, CorsLayer};

use crate::{
    auth,
    catalog::{build_dual_catalog, build_scalar_config, ServiceCatalog},
    config::AppConfig,
    errors::PortalError,
    proxy::proxy_router,
    security::{sanitize_path, security_headers_middleware},
};

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub catalog: Arc<ServiceCatalog>,
    pub auth_store: auth::SharedStore,
}

// ---------------------------------------------------------------------------
// Router assembly
// ---------------------------------------------------------------------------

pub fn build_router(state: AppState) -> Router {
    // CORS: explicit allowlist when configured; wildcard as dev fallback only.
    let cors = build_cors(&state.config.allowed_cors_origins);

    // Public routes: health probes and the OIDC login flow only.
    let public = Router::new()
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/auth/login", get(auth::login))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/session", get(auth::session_info))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout));

    // Protected routes: everything that exposes catalog, documentation,
    // specs, or the Scalar UI requires an authenticated portal session.
    let protected = Router::new()
        // Landing page
        .route("/", get(landing_page))
        // Scalar API Reference views
        .route("/reference/frontend", get(frontend_index))
        .route("/reference/frontend/{id}", get(frontend_service))
        .route("/reference/backend", get(backend_index))
        .route("/reference/backend/{id}", get(backend_service))
        // Backward-compat redirect from old Swagger UI URLs
        .route("/swagger-ui", get(swagger_compat_redirect))
        .route("/swagger-ui/", get(swagger_compat_redirect))
        // Catalog API
        .route("/api/v1/catalog", get(get_dual_catalog))
        .route("/api/v1/catalog/frontend", get(get_frontend_catalog))
        .route("/api/v1/catalog/backend", get(get_backend_catalog))
        // Per-service meta + spec redirect
        .route("/api/v1/services/{id}", get(get_service_meta))
        .route("/api/v1/services/{id}/spec", get(redirect_service_spec))
        // Spec files (path-traversal protected)
        .route("/specs/{*spec_path}", get(serve_spec_file))
        // Bundled Scalar JS (served from filesystem, immutable cache)
        .route("/static/scalar.min.js", get(serve_scalar_js))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_session,
        ));

    // API-testing proxy: requires session AND the portal-api-tester role.
    let proxy = proxy_router().route_layer(middleware::from_fn_with_state(
        state.clone(),
        auth::require_api_tester,
    ));

    Router::new()
        .merge(public)
        .merge(protected)
        .merge(proxy)
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(cors)
        .with_state(state)
}

// ---------------------------------------------------------------------------
// CORS helpers
// ---------------------------------------------------------------------------

fn build_cors(origins: &[String]) -> CorsLayer {
    let non_empty: Vec<&String> = origins.iter().filter(|o| !o.is_empty()).collect();
    if non_empty.is_empty() {
        // Development fallback — allow any origin
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    } else {
        let parsed: Vec<HeaderValue> = non_empty
            .iter()
            .filter_map(|o| o.parse::<HeaderValue>().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(parsed)
            .allow_methods([Method::GET, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    }
}

// ---------------------------------------------------------------------------
// Static assets
// ---------------------------------------------------------------------------

async fn serve_scalar_js(State(state): State<AppState>) -> Result<Response, PortalError> {
    let content = fs::read(&state.config.scalar_js_path).map_err(|e| {
        PortalError::Internal(format!(
            "Scalar JS bundle not found at {:?}: {}",
            state.config.scalar_js_path, e
        ))
    })?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/javascript; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );

    Ok((StatusCode::OK, headers, content).into_response())
}

// ---------------------------------------------------------------------------
// Health probes
// ---------------------------------------------------------------------------

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
        "schema_version": state.catalog.schema_version,
        "environment": state.config.environment,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// ---------------------------------------------------------------------------
// Backward-compat redirect
// ---------------------------------------------------------------------------

async fn swagger_compat_redirect() -> impl IntoResponse {
    Redirect::temporary("/reference/frontend")
}

// ---------------------------------------------------------------------------
// Landing page (/)
// ---------------------------------------------------------------------------

async fn landing_page() -> impl IntoResponse {
    Html(LANDING_PAGE_HTML)
}

// ---------------------------------------------------------------------------
// Scalar view: /reference/frontend  (index — no specific service)
// ---------------------------------------------------------------------------

async fn frontend_index(State(state): State<AppState>) -> impl IntoResponse {
    // Find the first enabled frontend service and redirect to its view, or show landing.
    let first = state.catalog.services.iter().find(|s| {
        s.enabled
            && s.environment
                .eq_ignore_ascii_case(&state.config.environment)
            && s.catalog.as_deref() == Some("frontend")
    });

    if let Some(svc) = first {
        Redirect::temporary(&format!("/reference/frontend/{}", svc.id)).into_response()
    } else {
        Html(LANDING_PAGE_HTML).into_response()
    }
}

// ---------------------------------------------------------------------------
// Scalar view: /reference/frontend/{id}
// ---------------------------------------------------------------------------

async fn frontend_service(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, PortalError> {
    render_scalar_page(&state, &id, "frontend").map(|html| html.into_response())
}

// ---------------------------------------------------------------------------
// Scalar view: /reference/backend  (index — redirect to first backend service)
// ---------------------------------------------------------------------------

async fn backend_index(State(state): State<AppState>) -> impl IntoResponse {
    let first = state.catalog.services.iter().find(|s| {
        s.enabled
            && s.environment
                .eq_ignore_ascii_case(&state.config.environment)
            && s.catalog.as_deref().unwrap_or("backend") == "backend"
    });

    if let Some(svc) = first {
        Redirect::temporary(&format!("/reference/backend/{}", svc.id)).into_response()
    } else {
        Html(LANDING_PAGE_HTML).into_response()
    }
}

// ---------------------------------------------------------------------------
// Scalar view: /reference/backend/{id}
// ---------------------------------------------------------------------------

async fn backend_service(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, PortalError> {
    render_scalar_page(&state, &id, "backend").map(|html| html.into_response())
}

// ---------------------------------------------------------------------------
// Shared Scalar page renderer
// ---------------------------------------------------------------------------

fn render_scalar_page(
    state: &AppState,
    service_id: &str,
    expected_catalog: &str,
) -> Result<Html<String>, PortalError> {
    let cfg = build_scalar_config(&state.catalog, service_id, &state.config.environment)
        .ok_or_else(|| {
            PortalError::NotFound(format!("Service '{}' not found in catalog", service_id))
        })?;

    if cfg.service_meta.catalog != expected_catalog {
        // Redirect cross-catalog lookups to the correct section instead of 404.
        return Err(PortalError::NotFound(format!(
            "Service '{}' belongs to '{}' catalog, not '{}'",
            service_id, cfg.service_meta.catalog, expected_catalog
        )));
    }

    let html = scalar_html(&cfg.service_meta.name, &cfg.spec.url);
    Ok(Html(html))
}

/// Builds the Scalar API Reference HTML page for a given service.
fn scalar_html(service_name: &str, spec_url: &str) -> String {
    // Escape to prevent XSS via catalog data
    let safe_name = html_escape(service_name);
    let safe_url = html_escape(spec_url);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta http-equiv="X-Content-Type-Options" content="nosniff" />
  <title>{name} — IAOP Developer Portal</title>
  <style>
    body {{ margin: 0; }}
  </style>
</head>
<body>
  <script id="api-reference" data-url="{url}"></script>
  <script src="/static/scalar.min.js"></script>
</body>
</html>"#,
        name = safe_name,
        url = safe_url,
    )
}

/// Minimal HTML attribute/content escaping to prevent injection via catalog strings.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// ---------------------------------------------------------------------------
// Catalog API endpoints
// ---------------------------------------------------------------------------

async fn get_dual_catalog(State(state): State<AppState>) -> impl IntoResponse {
    let dual = build_dual_catalog(&state.catalog, &state.config.environment);
    Json(dual)
}

async fn get_frontend_catalog(State(state): State<AppState>) -> impl IntoResponse {
    let dual = build_dual_catalog(&state.catalog, &state.config.environment);
    Json(dual.frontend)
}

async fn get_backend_catalog(State(state): State<AppState>) -> impl IntoResponse {
    let dual = build_dual_catalog(&state.catalog, &state.config.environment);
    Json(dual.backend)
}

async fn get_service_meta(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, PortalError> {
    let cfg = build_scalar_config(&state.catalog, &id, &state.config.environment)
        .ok_or_else(|| PortalError::NotFound(format!("Service '{}' not found", id)))?;
    Ok(Json(cfg))
}

async fn redirect_service_spec(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, PortalError> {
    let entry = state
        .catalog
        .services
        .iter()
        .find(|s| s.id == id && s.enabled)
        .ok_or_else(|| PortalError::NotFound(format!("Service '{}' not found", id)))?;

    Ok(Redirect::temporary(&format!("/{}", entry.spec_path)))
}

// ---------------------------------------------------------------------------
// Spec file serving (path-traversal protected)
// ---------------------------------------------------------------------------

async fn serve_spec_file(
    State(state): State<AppState>,
    Path(spec_path): Path<String>,
) -> Result<Response, PortalError> {
    let req_path = std::path::Path::new(&spec_path);
    let resolved_path = sanitize_path(&state.config.specs_dir, req_path)?;

    let raw = fs::read(&resolved_path)
        .map_err(|e| PortalError::Internal(format!("Failed to read spec file: {}", e)))?;

    // Rewrite server URLs to the controlled DEV proxy and inject the
    // Keycloak OAuth2 (auth-code + PKCE) scheme for backend APIs so Scalar
    // can execute real authorized requests without manual token handling.
    let content = transform_spec_for_testing(&state, &spec_path, &raw);

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

// ---------------------------------------------------------------------------
// Spec transformation for API testing (server rewrite + OAuth injection)
// ---------------------------------------------------------------------------

/// Maps a catalog service id to its DEV audience client-scope (`aud-*`).
fn audience_scope_for(service_id: &str) -> Option<&'static str> {
    match service_id {
        "agent-service" => Some("aud-agent"),
        "agency-service" => Some("aud-agency"),
        "eligibility-service" => Some("aud-eligibility"),
        _ => None,
    }
}

/// Returns the portal-relative proxy base for a service, but only when the
/// corresponding upstream is configured (empty upstream = testing disabled).
fn proxy_base_for<'a>(state: &'a AppState, service_id: &str) -> Option<String> {
    let portal = state.config.public_base_url.as_deref()?.trim_end_matches('/');
    match service_id {
        "agent-bff" if state.config.upstream_agent_bff.is_some() => {
            Some(format!("{portal}/agent-bff/api/v1"))
        }
        "agent-service" if state.config.upstream_agent_service.is_some() => {
            Some(format!("{portal}/dev-api/agent"))
        }
        "agency-service" if state.config.upstream_agency_service.is_some() => {
            Some(format!("{portal}/dev-api/agency"))
        }
        "eligibility-service" if state.config.upstream_eligibility_service.is_some() => {
            Some(format!("{portal}/dev-api/eligibility"))
        }
        _ => None,
    }
}

/// Rewrites `servers` to the controlled proxy and injects a Keycloak
/// OAuth2 authorization-code + PKCE security scheme for backend specs.
/// Returns the original bytes unchanged for YAML specs, unknown services,
/// or when no portal base URL is configured.
fn transform_spec_for_testing(state: &AppState, spec_path: &str, raw: &[u8]) -> Vec<u8> {
    if spec_path.ends_with(".yaml") || spec_path.ends_with(".yml") {
        return raw.to_vec();
    }
    let catalog_path = format!("specs/{spec_path}");
    let Some(entry) = state
        .catalog
        .services
        .iter()
        .find(|s| s.spec_path == catalog_path && s.enabled)
    else {
        return raw.to_vec();
    };
    let Some(portal) = state
        .config
        .public_base_url
        .as_deref()
        .map(|u| u.trim_end_matches('/').to_string())
    else {
        return raw.to_vec();
    };
    let Ok(mut doc) = serde_json::from_slice::<serde_json::Value>(raw) else {
        return raw.to_vec();
    };

    // 1. Server rewrite: browser must reach upstreams through the portal
    //    proxy (domain services have no public ports).
    let is_backend = entry.catalog.as_deref().unwrap_or("backend") == "backend";
    if let Some(base) = proxy_base_for(state, &entry.id) {
        doc["servers"] = serde_json::json!([{ "url": base }]);
    }

    // 2. OAuth injection for backend APIs only. Frontend BFF specs use the
    //    server-managed session cookie + CSRF model instead.
    if is_backend {
        if let Some(issuer) = &state.config.oidc_issuer {
            let issuer = issuer.trim_end_matches('/');
            let mut scopes = serde_json::Map::new();
            for (k, v) in [
                ("openid", "OpenID Connect"),
                ("profile", "User profile"),
                ("email", "Email address"),
            ] {
                scopes.insert(k.into(), serde_json::Value::String(v.into()));
            }
            let mut selected = vec!["openid".to_string()];
            if let Some(aud) = audience_scope_for(&entry.id) {
                scopes.insert(aud.into(), serde_json::Value::String(format!("Audience: {aud}")));
                selected.push(aud.to_string());
            }
            let redirect = format!("{portal}/reference/backend/{}", entry.id);
            let scheme = serde_json::json!({
                "type": "oauth2",
                "description": "Keycloak Authorization Code + PKCE (public client iaop-scalar-dev)",
                "flows": {
                    "authorizationCode": {
                        "authorizationUrl": format!("{issuer}/protocol/openid-connect/auth"),
                        "tokenUrl": format!("{issuer}/protocol/openid-connect/token"),
                        "x-usePkce": "SHA-256",
                        "x-scalar-client-id": "iaop-scalar-dev",
                        "x-scalar-redirect-uri": redirect,
                        "x-scalar-credentials-location": "body",
                        "scopes": scopes,
                        "selectedScopes": selected,
                    }
                }
            });
            doc["components"]["securitySchemes"]["keycloakOAuth"] = scheme;
            doc["security"] = serde_json::json!([{ "keycloakOAuth": [] }]);
        }
    }

    serde_json::to_vec(&doc).unwrap_or_else(|_| raw.to_vec())
}

// ---------------------------------------------------------------------------
// Landing page HTML (embedded string literal — no external file dependency)
// ---------------------------------------------------------------------------

static LANDING_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>IAOP Developer Portal</title>
  <style>
    * { box-sizing: border-box; }
    body { margin: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; background: #0f1117; color: #e2e8f0; }
    .header { padding: 32px 48px; border-bottom: 1px solid #1e2433; display: flex; align-items: center; gap: 16px; }
    .header h1 { margin: 0; font-size: 20px; font-weight: 600; color: #f8fafc; }
    .header .badge { font-size: 11px; padding: 2px 8px; background: #1e3a5f; color: #60a5fa; border-radius: 4px; font-weight: 500; }
    .container { max-width: 1200px; margin: 0 auto; padding: 48px; }
    .section-title { font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; color: #64748b; margin-bottom: 20px; margin-top: 40px; }
    .section-title:first-child { margin-top: 0; }
    .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 16px; }
    .card { background: #161b27; border: 1px solid #1e2433; border-radius: 8px; padding: 20px; text-decoration: none; color: inherit; display: block; transition: border-color 0.15s, background 0.15s; }
    .card:hover { border-color: #3b82f6; background: #1a2035; }
    .card-name { font-size: 15px; font-weight: 600; color: #f1f5f9; margin-bottom: 6px; }
    .card-desc { font-size: 13px; color: #94a3b8; line-height: 1.5; margin-bottom: 12px; }
    .card-meta { display: flex; gap: 8px; flex-wrap: wrap; }
    .tag { font-size: 11px; padding: 2px 8px; border-radius: 4px; font-weight: 500; }
    .tag-active { background: #14532d; color: #4ade80; }
    .tag-proposed { background: #1c1917; color: #a8a29e; border: 1px solid #292524; }
    .tag-frontend { background: #1e1b4b; color: #818cf8; }
    .tag-backend { background: #0c1a2e; color: #38bdf8; }
    .group-label { font-size: 13px; font-weight: 500; color: #64748b; margin-top: 32px; margin-bottom: 12px; padding-left: 10px; border-left: 2px solid #1e2433; }
  </style>
</head>
<body>
  <div class="header">
    <h1>IAOP Developer Portal</h1>
    <span class="badge">DEV</span>
  </div>
  <div class="container">
    <div class="section-title">Frontend APIs</div>
    <div class="grid" id="frontend-grid"></div>
    <div class="section-title">Backend APIs</div>
    <div id="backend-sections"></div>
  </div>
  <script>
    fetch('/api/v1/catalog').then(r => r.json()).then(catalog => {
      const fg = document.getElementById('frontend-grid');
      (catalog.frontend || []).forEach(group => {
        group.services.forEach(svc => {
          const a = document.createElement('a');
          a.href = '/reference/frontend/' + svc.id;
          a.className = 'card';
          const lc = svc.lifecycle || 'proposed';
          a.innerHTML = '<div class="card-name">' + svc.name + '</div>'
            + '<div class="card-desc">' + (svc.description || '').substring(0, 120) + '</div>'
            + '<div class="card-meta">'
            + '<span class="tag tag-frontend">Frontend</span>'
            + '<span class="tag ' + (lc === 'active' ? 'tag-active' : 'tag-proposed') + '">' + lc + '</span>'
            + '<span class="tag" style="background:#1e2433;color:#94a3b8">v' + svc.apiVersion + '</span>'
            + '</div>';
          fg.appendChild(a);
        });
      });
      const bs = document.getElementById('backend-sections');
      (catalog.backend || []).forEach(group => {
        const gl = document.createElement('div');
        gl.className = 'group-label';
        gl.textContent = group.label;
        bs.appendChild(gl);
        const grid = document.createElement('div');
        grid.className = 'grid';
        group.services.forEach(svc => {
          const a = document.createElement('a');
          a.href = '/reference/backend/' + svc.id;
          a.className = 'card';
          const lc = svc.lifecycle || 'proposed';
          a.innerHTML = '<div class="card-name">' + svc.name + '</div>'
            + '<div class="card-desc">' + (svc.description || '').substring(0, 120) + '</div>'
            + '<div class="card-meta">'
            + '<span class="tag tag-backend">Backend</span>'
            + '<span class="tag ' + (lc === 'active' ? 'tag-active' : 'tag-proposed') + '">' + lc + '</span>'
            + '<span class="tag" style="background:#1e2433;color:#94a3b8">v' + svc.apiVersion + '</span>'
            + '</div>';
          grid.appendChild(a);
        });
        bs.appendChild(grid);
      });
    });
  </script>
</body>
</html>"#;
