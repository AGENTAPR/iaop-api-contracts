use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use std::{fs, sync::Arc};
use tower_http::cors::CorsLayer;

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
        // Portal UI assets (embedded at compile time — same-origin, CSP-safe)
        .route("/static/portal.css", get(serve_portal_css))
        .route("/static/portal.js", get(serve_portal_js))
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
        // No CORS origins configured: same-origin only (browser default).
        // Wildcard CORS is never permitted for authenticated routes because
        // it cannot be combined with Allow-Credentials and would expose
        // session-gated content to cross-origin scripts.
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    } else {
        // Explicit allowlist from configuration — validate at startup.
        let parsed: Vec<HeaderValue> = non_empty
            .iter()
            .filter_map(|o| {
                o.parse::<HeaderValue>()
                    .map_err(|e| tracing::warn!("Invalid CORS origin {o:?}: {e}"))
                    .ok()
            })
            .collect();
        CorsLayer::new()
            .allow_origin(parsed)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
    }
}

// ---------------------------------------------------------------------------
// Static assets
// ---------------------------------------------------------------------------

async fn serve_portal_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        [(
            header::CACHE_CONTROL,
            "public, max-age=300, must-revalidate",
        )],
        PORTAL_CSS,
    )
        .into_response()
}

async fn serve_portal_js() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        [(
            header::CACHE_CONTROL,
            "public, max-age=300, must-revalidate",
        )],
        PORTAL_JS,
    )
        .into_response()
}

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

    let html = scalar_html(&cfg.service_meta, &cfg.spec.url);
    Ok(Html(html))
}

/// Builds the Scalar API Reference page wrapped in the portal shell.
/// The shell provides catalog navigation, service identity, environment and
/// auth-model context; Scalar itself renders the spec unchanged.
fn scalar_html(meta: &crate::catalog::ServiceMeta, spec_url: &str) -> String {
    let safe_name = html_escape(&meta.name);
    let safe_url = html_escape(spec_url);
    let catalog_kind = meta.catalog.as_str();
    let section = if catalog_kind == "frontend" {
        "Frontend"
    } else {
        "Backend"
    };
    let ref_base = if catalog_kind == "frontend" {
        "/reference/frontend"
    } else {
        "/reference/backend"
    };
    let auth_note = if catalog_kind == "frontend" {
        "Testing via BFF session cookie + CSRF — connect through the BFF login flow."
    } else {
        "Testing via OAuth2 Authorization Code + PKCE (client iaop-scalar-dev)."
    };
    let safe_section = html_escape(section);
    let safe_auth_note = html_escape(auth_note);
    let safe_repo = html_escape(&meta.repository);
    let status_bits: Vec<String> = [
        meta.implementation_status.as_deref(),
        meta.deployment_status.as_deref(),
        meta.verification_status.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(|s| format!("<span class=\"tag tag-proposed\">{}</span>", html_escape(s)))
    .collect();
    let status_html = status_bits.join(" ");
    let api_version = html_escape(&format!("v{}", meta.api_version));
    let description = meta
        .description
        .as_deref()
        .map(|d| format!("<div class=\"ref-desc\">{}</div>", html_escape(d)))
        .unwrap_or_default();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta http-equiv="X-Content-Type-Options" content="nosniff" />
  <title>{name} — IAOP Developer Portal</title>
  <link rel="stylesheet" href="/static/portal.css" />
</head>
<body>
<div class="ref-shell">
  <div class="ref-bar">
    <a href="/">← Catalog</a>
    <span class="sep">/</span>
    <a href="{ref_base}">{section}</a>
    <span class="sep">/</span>
    <span class="svc-name">{name}</span>
    <span class="tag tag-version">{api_version}</span>
    {status_html}
    <span class="auth-note">{auth_note}</span>
  </div>
  {description}
  <div class="ref-body">
    <script id="api-reference" data-url="{url}"></script>
    <script src="/static/scalar.min.js"></script>
  </div>
</div>
</body>
</html>"#,
        name = safe_name,
        url = safe_url,
        ref_base = ref_base,
        section = safe_section,
        api_version = api_version,
        status_html = status_html,
        auth_note = safe_auth_note,
        description = description,
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
    let mut dual = build_dual_catalog(&state.catalog, &state.config.environment);
    enrich_operation_counts(&state, &mut dual);
    Json(dual)
}

/// Counts OpenAPI operations per service by reading its published spec.
/// Only JSON specs are counted; missing/unparseable specs yield `None`.
fn enrich_operation_counts(state: &AppState, dual: &mut crate::catalog::DualCatalogResponse) {
    let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for group in dual.frontend.iter_mut().chain(dual.backend.iter_mut()) {
        for svc in group.services.iter_mut() {
            let count = counts
                .entry(svc.id.clone())
                .or_insert_with(|| count_spec_operations(&state.config.specs_dir, &svc.spec_path));
            svc.operation_count = Some(*count);
        }
    }
}

fn count_spec_operations(specs_dir: &std::path::Path, spec_path: &str) -> u32 {
    // spec_path looks like "specs/development/<svc>/openapi.json"
    let rel = spec_path.strip_prefix("specs/").unwrap_or(spec_path);
    let file = specs_dir.join(rel);
    let Ok(raw) = fs::read(&file) else { return 0 };
    let Ok(doc) = serde_json::from_slice::<serde_json::Value>(&raw) else {
        return 0;
    };
    doc["paths"]
        .as_object()
        .map(|paths| {
            paths
                .values()
                .map(|ops| {
                    ops.as_object()
                        .map(|m| {
                            m.keys()
                                .filter(|k| {
                                    matches!(
                                        k.to_ascii_lowercase().as_str(),
                                        "get"
                                            | "post"
                                            | "put"
                                            | "patch"
                                            | "delete"
                                            | "head"
                                            | "options"
                                            | "trace"
                                    )
                                })
                                .count() as u32
                        })
                        .unwrap_or(0)
                })
                .sum()
        })
        .unwrap_or(0)
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
    let portal = state
        .config
        .public_base_url
        .as_deref()?
        .trim_end_matches('/');
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
                scopes.insert(
                    aud.into(),
                    serde_json::Value::String(format!("Audience: {aud}")),
                );
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
// Portal UI assets (embedded at compile time — served same-origin so the
// strict CSP `script-src 'self'` is preserved with no 'unsafe-inline').
// ---------------------------------------------------------------------------

static PORTAL_CSS: &str = include_str!("../static/portal.css");
static PORTAL_JS: &str = include_str!("../static/portal.js");

// ---------------------------------------------------------------------------
// Landing page HTML shell — all catalog rendering happens in portal.js,
// which builds DOM nodes only (no innerHTML) from /api/v1/catalog.
// ---------------------------------------------------------------------------

static LANDING_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta http-equiv="X-Content-Type-Options" content="nosniff" />
  <title>IAOP Developer Portal</title>
  <link rel="stylesheet" href="/static/portal.css" />
</head>
<body>
  <div class="portal-header">
    <a class="brand" href="/">IAOP Developer Portal</a>
    <span class="env-badge">DEV</span>
    <nav>
      <span class="session-state" id="session-note">Signed in</span>
    </nav>
  </div>
  <main class="portal-main">
    <div class="page-intro">
      <h1>API Catalog</h1>
      <p>Frontend BFF contracts and backend domain-service contracts for the IAOP platform. Select a service to open its Scalar API reference.</p>
    </div>

    <div class="section-title">Frontend APIs <span class="count" id="frontend-count"></span></div>
    <div id="frontend-groups" aria-live="polite"></div>

    <div class="section-title">Backend APIs <span class="count" id="backend-count"></span></div>
    <div id="backend-sections" aria-live="polite"></div>
  </main>
  <noscript>
    <div class="portal-main"><div class="state-box">JavaScript is required to render the API catalog.</div></div>
  </noscript>
  <script src="/static/portal.js" defer></script>
</body>
</html>"#;
