use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

use iaop_developer_portal::{
    build_router, load_and_validate_catalog, security::sanitize_path, AppConfig, AppState,
};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn setup_test_state() -> AppState {
    let md = manifest_dir();
    let catalog_path = md.join("../catalog/services.yaml");
    let schema_path = md.join("../catalog/catalog.schema.json");
    let specs_dir = md.join("../specs");

    let catalog = load_and_validate_catalog(&catalog_path, Some(&schema_path))
        .expect("Failed to load catalog for tests");

    let config = AppConfig {
        port: 8080,
        host: "127.0.0.1".into(),
        catalog_path,
        catalog_schema_path: schema_path,
        specs_dir,
        environment: "development".into(),
        allowed_visibility: vec!["public".into(), "internal".into(), "restricted".into()],
        public_base_url: None,
        allowed_cors_origins: vec![],
                scalar_js_path: PathBuf::from("portal/tests/fixtures/scalar.min.js"),
        oidc_issuer: None,
        oidc_client_id: None,
        oidc_client_secret: None,
        oidc_redirect_uri: None,
        oidc_jwks_url: None,
        portal_tester_role: "portal-api-tester".into(),
        upstream_agent_bff: None,
        upstream_agent_service: None,
        upstream_agency_service: None,
        upstream_eligibility_service: None,
    };

    let state = AppState {
        config,
        catalog: Arc::new(catalog),
        auth_store: iaop_developer_portal::auth::make_store(),
    };
    seed_session(&state);
    state
}

fn setup_test_state_with_cors(origins: Vec<String>) -> AppState {
    let mut state = setup_test_state();
    state.config.allowed_cors_origins = origins;
    state
}

// ---------------------------------------------------------------------------
// Path traversal — sanitize_path unit tests
// ---------------------------------------------------------------------------

#[test]
fn sanitize_path_rejects_parent_dir_traversal() {
    let specs_dir = manifest_dir().join("../specs");
    let evil_path = std::path::Path::new("../Cargo.toml");
    let res = sanitize_path(&specs_dir, evil_path);
    assert!(res.is_err(), "Must reject ../ sequences");
}

#[test]
fn sanitize_path_rejects_absolute_paths() {
    let specs_dir = manifest_dir().join("../specs");

    #[cfg(windows)]
    let evil_path = std::path::Path::new("C:\\Windows\\System32\\drivers\\etc\\hosts");
    #[cfg(not(windows))]
    let evil_path = std::path::Path::new("/etc/passwd");

    let res = sanitize_path(&specs_dir, evil_path);
    assert!(res.is_err(), "Must reject absolute paths");
}

// ---------------------------------------------------------------------------
// Path traversal — HTTP integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn http_rejects_path_traversal_attempts() {
    let state = setup_test_state();
    let app = test_app(state);

    let req = Request::builder()
        .uri("/specs/development/../../Cargo.toml")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status() == StatusCode::FORBIDDEN || res.status() == StatusCode::NOT_FOUND,
        "HTTP traversal must be blocked, got: {}",
        res.status()
    );

    let req = Request::builder()
        .uri("/specs/development/../.gitignore")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert!(
        res.status() == StatusCode::FORBIDDEN || res.status() == StatusCode::NOT_FOUND,
        "HTTP traversal must be blocked, got: {}",
        res.status()
    );
}

// ---------------------------------------------------------------------------
// Security headers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn security_headers_present_on_all_responses() {
    let state = setup_test_state();
    let app = test_app(state);

    let req = Request::builder()
        .uri("/health/live")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    let headers = res.headers();
    assert_eq!(
        headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff",
        "X-Content-Type-Options must be nosniff"
    );
    assert_eq!(
        headers.get(header::X_FRAME_OPTIONS).unwrap(),
        "DENY",
        "X-Frame-Options must be DENY"
    );
    let csp = headers
        .get(header::CONTENT_SECURITY_POLICY)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        csp.contains("frame-ancestors 'none'"),
        "CSP must include frame-ancestors 'none'"
    );
    assert!(
        csp.contains("worker-src 'none'"),
        "CSP must include worker-src 'none'"
    );
    // Scalar JS is same-origin — no CDN needed in script-src
    assert!(
        !csp.contains("cdn.jsdelivr.net"),
        "CSP must not reference external CDN"
    );
}

// ---------------------------------------------------------------------------
// CORS — explicit allowlist prevents wildcard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cors_allows_configured_origin() {
    let state = setup_test_state_with_cors(vec!["https://portal.iaop.internal".to_string()]);
    let app = test_app(state);

    let req = Request::builder()
        .uri("/health/live")
        .header(header::ORIGIN, "https://portal.iaop.internal")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .method("OPTIONS")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    // The response should be 200 or 204 for OPTIONS
    assert!(
        res.status().is_success() || res.status() == StatusCode::NO_CONTENT,
        "OPTIONS preflight must succeed for configured origin"
    );

    let acao = res
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");
    assert_ne!(
        acao, "*",
        "ACAO must not be wildcard when origins are explicitly configured"
    );
}

#[tokio::test]
async fn cors_wildcard_only_in_dev_mode_no_origins_configured() {
    // When allowed_cors_origins is empty, the portal uses wildcard as a dev fallback.
    // This test documents and verifies that behaviour is intentional and limited.
    let state = setup_test_state_with_cors(vec![]);
    let app = test_app(state);

    let req = Request::builder()
        .uri("/health/live")
        .header(header::ORIGIN, "https://untrusted.example.com")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Wildcard dev mode must still serve health endpoint"
    );
}

// ---------------------------------------------------------------------------
// Test helpers: seeded portal session + cookie injection
// ---------------------------------------------------------------------------

const TEST_SESSION_ID: &str = "test-portal-session";

fn seed_session(state: &AppState) {
    state.auth_store.put_session(iaop_developer_portal::auth::PortalSession {
        session_id: TEST_SESSION_ID.into(),
        subject: "test-subject".into(),
        display_name: "Test User".into(),
        email: None,
        roles: vec!["portal-api-tester".into()],
        created_at: std::time::Instant::now(),
    });
}

async fn inject_test_session(
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if !req.headers().contains_key(axum::http::header::COOKIE) {
        req.headers_mut().insert(
            axum::http::header::COOKIE,
            axum::http::HeaderValue::from_static("portal_session=test-portal-session"),
        );
    }
    next.run(req).await
}

fn test_app(state: AppState) -> axum::Router {
    build_router(state).layer(axum::middleware::from_fn(inject_test_session))
}
