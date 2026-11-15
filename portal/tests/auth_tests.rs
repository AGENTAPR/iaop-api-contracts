//! Portal authentication & authorization tests.
//! Verifies login-first enforcement and API-tester permission on proxy routes.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

use iaop_developer_portal::{
    auth, build_router, load_and_validate_catalog, AppConfig, AppState,
};

fn setup_state() -> AppState {
    let md = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog = load_and_validate_catalog(
        &md.join("../catalog/services.yaml"),
        Some(&md.join("../catalog/catalog.schema.json")),
    )
    .expect("catalog");

    AppState {
        config: AppConfig {
            port: 8080,
            host: "127.0.0.1".into(),
            catalog_path: md.join("../catalog/services.yaml"),
            catalog_schema_path: md.join("../catalog/catalog.schema.json"),
            specs_dir: md.join("../specs"),
            environment: "development".into(),
            allowed_visibility: vec!["public".into(), "internal".into(), "restricted".into()],
            public_base_url: Some("https://portal.test".into()),
            allowed_cors_origins: vec![],
            scalar_js_path: PathBuf::from("portal/tests/fixtures/scalar.min.js"),
            oidc_issuer: Some("https://idp.test/realms/test".into()),
            oidc_client_id: Some("iaop-developer-portal".into()),
            oidc_client_secret: Some("secret".into()),
            oidc_redirect_uri: Some("https://portal.test/auth/callback".into()),
            oidc_jwks_url: None,
            portal_tester_role: "portal-api-tester".into(),
            upstream_agent_bff: Some("http://agent-bff:8080".into()),
            upstream_agent_service: Some("http://agent-service:8080".into()),
            upstream_agency_service: None,
            upstream_eligibility_service: None,
        },
        catalog: Arc::new(catalog),
        auth_store: auth::make_store(),
    }
}

fn session(state: &AppState, sid: &str, roles: Vec<&str>) {
    state.auth_store.put_session(auth::PortalSession {
        session_id: sid.into(),
        subject: "sub".into(),
        display_name: "U".into(),
        email: None,
        roles: roles.into_iter().map(str::to_string).collect(),
        created_at: std::time::Instant::now(),
    });
}

fn req(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn req_with_session(uri: &str, sid: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header(header::COOKIE, format!("portal_session={sid}"))
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn anonymous_denied_everywhere_except_health_and_auth() {
    let app = build_router(setup_state());
    for uri in [
        "/",
        "/api/v1/catalog",
        "/api/v1/catalog/backend",
        "/api/v1/services/agent-service",
        "/specs/development/agent-service/openapi.json",
        "/reference/backend/agent-service",
        "/reference/frontend/agent-bff",
        "/static/scalar.min.js",
    ] {
        let res = app.clone().oneshot(req(uri)).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "anon {uri}");
    }
    // Health probes stay public.
    for uri in ["/health/live", "/health/ready"] {
        let res = app.clone().oneshot(req(uri)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "public {uri}");
    }
}

#[tokio::test]
async fn authenticated_session_accesses_catalog_but_not_proxy_without_role() {
    let state = setup_state();
    session(&state, "viewer", vec![]); // authenticated, NO tester role
    let app = build_router(state);

    // Catalog + spec: session is sufficient.
    let res = app
        .clone()
        .oneshot(req_with_session("/api/v1/catalog", "viewer"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(req_with_session(
            "/specs/development/agent-service/openapi.json",
            "viewer",
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Spec must carry the injected Keycloak OAuth scheme for backend APIs.
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let scheme = &doc["components"]["securitySchemes"]["keycloakOAuth"];
    assert_eq!(scheme["type"], "oauth2");
    assert_eq!(
        scheme["flows"]["authorizationCode"]["x-usePkce"],
        "SHA-256"
    );
    assert_eq!(
        scheme["flows"]["authorizationCode"]["x-scalar-client-id"],
        "iaop-scalar-dev"
    );
    // Server rewritten to the controlled /dev-api proxy.
    assert_eq!(
        doc["servers"][0]["url"],
        "https://portal.test/dev-api/agent"
    );

    // Proxy routes: authenticated but missing tester role -> 403.
    for uri in [
        "/dev-api/agent/api/v1/agents",
        "/dev-api/agency/api/v1/system/info",
        "/agent-bff/api/v1/me",
    ] {
        let res = app
            .clone()
            .oneshot(req_with_session(uri, "viewer"))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "proxy {uri}");
    }
}

#[tokio::test]
async fn tester_role_reaches_proxy_and_unknown_upstream_denied() {
    let state = setup_state();
    session(&state, "tester", vec!["portal-api-tester"]);
    let app = build_router(state);

    // agency upstream is NOT configured -> 404 even for authorized tester.
    let res = app
        .clone()
        .oneshot(req_with_session("/dev-api/agency/api/v1/system/info", "tester"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // agent upstream IS configured; request will attempt a real upstream
    // call which fails (no such host in test env) -> 500, but NOT 401/403:
    // proves the request passed portal authorization.
    let res = app
        .clone()
        .oneshot(req_with_session("/dev-api/agent/api/v1/agents", "tester"))
        .await
        .unwrap();
    assert!(
        res.status().is_server_error() || res.status().is_success(),
        "expected upstream attempt, got {}",
        res.status()
    );
}

#[tokio::test]
async fn bogus_session_cookie_denied() {
    let app = build_router(setup_state());
    let res = app
        .oneshot(req_with_session("/api/v1/catalog", "nonexistent-sid"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
