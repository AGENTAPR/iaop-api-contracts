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
        .route("/auth/logout", post(auth::logout))
        .route("/favicon.ico", get(favicon));

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
        // Portal UI assets (embedded at compile time — same-origin, CSP-safe).
        // Allowlist of ADL stylesheets and scripts; anything else is 404.
        .route("/static/{*asset}", get(serve_static_asset))
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

/// Embedded ADL/portal assets keyed by their public /static/ path.
/// Only allowlisted files are served — the wildcard route cannot escape
/// this map, so there is no path-traversal surface.
fn static_asset(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "styles/adl-tokens.css" => Some(("text/css; charset=utf-8", ADL_TOKENS_CSS)),
        "styles/portal.css" => Some(("text/css; charset=utf-8", PORTAL_CSS)),
        "styles/scalar-overrides.css" => {
            Some(("text/css; charset=utf-8", SCALAR_OVERRIDES_CSS))
        }
        "scripts/theme-init.js" => {
            Some(("application/javascript; charset=utf-8", THEME_INIT_JS))
        }
        "scripts/theme.js" => Some(("application/javascript; charset=utf-8", THEME_JS)),
        "scripts/theme-sync.js" => {
            Some(("application/javascript; charset=utf-8", THEME_SYNC_JS))
        }
        "scripts/portal.js" => Some(("application/javascript; charset=utf-8", PORTAL_JS)),
        _ => None,
    }
}

async fn serve_static_asset(
    Path(asset): Path<String>,
) -> Result<impl IntoResponse, PortalError> {
    let (content_type, body) = static_asset(&asset)
        .ok_or_else(|| PortalError::NotFound(format!("Unknown static asset '{asset}'")))?;
    Ok((
        [(header::CONTENT_TYPE, content_type)],
        [(
            header::CACHE_CONTROL,
            "public, max-age=300, must-revalidate",
        )],
        body,
    )
        .into_response())
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

async fn favicon() -> impl IntoResponse {
    StatusCode::NO_CONTENT
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
    Html(landing_html())
}

/// Landing shell with the shared ADL header injected server-side.
fn landing_html() -> String {
    LANDING_PAGE_HTML.replacen("{header}", &portal_header_html("overview"), 1)
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
        Html(landing_html()).into_response()
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
        Html(landing_html()).into_response()
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
        "Frontend APIs"
    } else {
        "Backend APIs"
    };
    let ref_base = if catalog_kind == "frontend" {
        "/reference/frontend"
    } else {
        "/reference/backend"
    };
    let kind_label = if catalog_kind == "frontend" {
        "Application-facing BFF contract"
    } else {
        "Domain service contract"
    };
    let auth_note = if catalog_kind == "frontend" {
        "API testing: BFF session cookie + CSRF via the BFF login flow"
    } else {
        "API testing: OAuth2 Authorization Code + PKCE (client iaop-scalar-dev)"
    };
    let safe_section = html_escape(section);
    let safe_kind_label = html_escape(kind_label);
    let safe_auth_note = html_escape(auth_note);
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
        .map(|d| format!("<span class=\"ref-desc\">{}</span>", html_escape(d)))
        .unwrap_or_default();

    // Scalar bootstrap config. theme-sync.js rewrites darkMode from the
    // portal theme before scalar.min.js executes; "auto" is the no-JS
    // fallback. hideDarkModeToggle keeps a single theme control (ours).
    let scalar_config = serde_json::json!({
        "theme": "default",
        "darkMode": "auto",
        "hideDarkModeToggle": true,
        "hideClientButton": true,
    });
    let safe_config = html_escape(&scalar_config.to_string());
    let title = format!("{} — IAOP Developer Portal", meta.name);

    format!(
        r#"{head}
<body>
{header}
  <div class="ref-shell">
    <div class="ref-context">
      <a href="/">Catalog</a>
      <span class="sep">/</span>
      <a href="{ref_base}">{section}</a>
      <span class="sep">/</span>
      <span class="ref-title">{name}</span>
      <span class="tag tag-version">{api_version}</span>
      {status_html}
      <span class="ref-kind">{kind_label}</span>
      {description}
      <span class="auth-note">{auth_note}</span>
    </div>
    <div class="ref-body">
      <script id="api-reference" data-url="{url}" data-configuration='{config}'></script>
      <script src="/static/scripts/theme-sync.js"></script>
      <script src="/static/scripts/theme.js" defer></script>
      <script src="/static/scalar.min.js"></script>
    </div>
  </div>
</body></html>"#,
        head = html_head(&title),
        header = portal_header_html("reference"),
        name = safe_name,
        url = safe_url,
        ref_base = ref_base,
        section = safe_section,
        api_version = api_version,
        status_html = status_html,
        kind_label = safe_kind_label,
        auth_note = safe_auth_note,
        description = description,
        config = safe_config,
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

static ADL_TOKENS_CSS: &str = include_str!("../static/styles/adl-tokens.css");
static PORTAL_CSS: &str = include_str!("../static/styles/portal.css");
static SCALAR_OVERRIDES_CSS: &str = include_str!("../static/styles/scalar-overrides.css");
static THEME_INIT_JS: &str = include_str!("../static/scripts/theme-init.js");
static THEME_JS: &str = include_str!("../static/scripts/theme.js");
static THEME_SYNC_JS: &str = include_str!("../static/scripts/theme-sync.js");
static PORTAL_JS: &str = include_str!("../static/scripts/portal.js");

// ---------------------------------------------------------------------------
// Shared ADL page chrome — header/nav/session controls used by both the
// catalog landing page and every Scalar reference page.
// ---------------------------------------------------------------------------

/// Renders the sticky portal header. `active` is "overview" | "frontend" |
/// "backend" | "reference" and controls aria-current on the nav links.
fn portal_header_html(active: &str) -> String {
    let nav_link = |href: &str, label: &str, key: &str| -> String {
        let current = if active == key { " aria-current=\"page\"" } else { "" };
        format!(
            "<a href=\"{href}\"{current}>{label}</a>",
            href = html_escape(href),
            label = html_escape(label),
            current = current,
        )
    };
    format!(
        r#"<header class="portal-header">
  <div class="portal-header-inner">
    <a class="brand" href="/">
      <span class="brand-mark" aria-hidden="true">A</span>
      <span>IAOP Developer Portal</span>
      <span class="brand-sub">Agena</span>
    </a>
    <span class="env-badge"><span id="env-name">DEV</span></span>
    <nav class="portal-nav" aria-label="Portal">
      {overview}
      {frontend}
      {backend}
    </nav>
    <div class="portal-header-actions">
      <span class="user-chip" title="Portal session">
        <span class="session-dot" aria-hidden="true"></span>
        <span class="user-name" id="user-name">Signed in</span>
      </span>
      <button type="button" class="theme-toggle" data-theme-toggle aria-pressed="false" aria-label="Toggle theme" title="Toggle theme">
        <svg class="icon-moon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z"/></svg>
        <svg class="icon-sun" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>
      </button>
      <button type="button" class="logout-btn" id="logout-btn">Sign out</button>
    </div>
  </div>
</header>"#,
        overview = nav_link("/", "Overview", "overview"),
        frontend = nav_link("/#frontend-apis", "Frontend APIs", "frontend"),
        backend = nav_link("/#backend-apis", "Backend APIs", "backend"),
    )
}

/// Shared <head> prelude: charset, viewport, ADL tokens + portal styles,
/// synchronous theme bootstrap (sets data-theme before first paint).
fn html_head(title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta http-equiv="X-Content-Type-Options" content="nosniff" />
  <title>{title}</title>
  <link rel="stylesheet" href="/static/styles/adl-tokens.css" />
  <link rel="stylesheet" href="/static/styles/portal.css" />
  <link rel="stylesheet" href="/static/styles/scalar-overrides.css" />
  <script src="/static/scripts/theme-init.js"></script>
</head>"#,
        title = html_escape(title),
    )
}

/// Deferred theme wiring + optional extra scripts, closed before </body>.
fn html_tail(extra_scripts: &str) -> String {
    format!(
        "<script src=\"/static/scripts/theme.js\" defer></script>{extra}</body></html>",
        extra = extra_scripts,
    )
}

static LANDING_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta http-equiv="X-Content-Type-Options" content="nosniff" />
  <title>IAOP Developer Portal — Agena</title>
  <link rel="stylesheet" href="/static/styles/adl-tokens.css" />
  <link rel="stylesheet" href="/static/styles/portal.css" />
  <script src="/static/scripts/theme-init.js"></script>
</head>
<body>
  {header}
  <main class="portal-main">
    <section class="hero" aria-labelledby="hero-title">
      <h1 id="hero-title">Explore the Agena Platform</h1>
      <p>Discover APIs, inspect contracts, and securely test integrations across the IAOP insurance agent platform.</p>
    </section>

    <section class="catalog-section" id="frontend-apis" aria-labelledby="frontend-title">
      <h2 class="section-title" id="frontend-title">
        <span>Frontend APIs</span>
        <span class="count" id="frontend-count"></span>
      </h2>
      <p class="section-desc">Application-facing BFF contracts — what agent, office, and customer clients actually consume. Browser session + CSRF for API testing.</p>
      <div id="frontend-groups" aria-live="polite"></div>
    </section>

    <section class="catalog-section" id="backend-apis" aria-labelledby="backend-title">
      <h2 class="section-title" id="backend-title">
        <span>Backend APIs</span>
        <span class="count" id="backend-count"></span>
      </h2>
      <p class="section-desc">Authoritative domain and platform service contracts, grouped by bounded context. OAuth2 Authorization Code + PKCE for API testing.</p>
      <div id="backend-sections" aria-live="polite"></div>
    </section>
  </main>
  <footer class="portal-footer">
    IAOP Developer Portal · Agena · environment <span class="mono" id="env-footer">development</span>
  </footer>
  <noscript>
    <div class="portal-main"><div class="state-box">JavaScript is required to render the API catalog.</div></div>
  </noscript>
  <script src="/static/scripts/theme.js" defer></script>
  <script src="/static/scripts/portal.js" defer></script>
</body>
</html>"#;
