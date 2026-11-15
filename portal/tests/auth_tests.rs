//! Portal authentication & authorization tests.
//! Verifies login-first enforcement and API-tester permission on proxy routes.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

use iaop_developer_portal::{auth, build_router, load_and_validate_catalog, AppConfig, AppState};

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
        id_token_hint: None,
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

/// Browser page-navigation request: GET + Accept: text/html.
fn req_html(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header(
            header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .body(Body::empty())
        .unwrap()
}

fn location_of(res: &axum::response::Response) -> String {
    res.headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
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
    assert_eq!(scheme["flows"]["authorizationCode"]["x-usePkce"], "SHA-256");
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
        .oneshot(req_with_session(
            "/dev-api/agency/api/v1/system/info",
            "tester",
        ))
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

// ---------------------------------------------------------------------------
// HTML navigation → OIDC login redirect
// ---------------------------------------------------------------------------

#[tokio::test]
async fn anonymous_html_navigation_redirects_to_login_with_return_to() {
    let app = build_router(setup_state());
    for (uri, expected_dest) in [
        ("/", "%2F"),
        (
            "/reference/frontend/agent-bff",
            "%2Freference%2Ffrontend%2Fagent-bff",
        ),
        (
            "/reference/backend/agency-service",
            "%2Freference%2Fbackend%2Fagency-service",
        ),
    ] {
        let res = app.clone().oneshot(req_html(uri)).await.unwrap();
        assert_eq!(res.status(), StatusCode::SEE_OTHER, "nav {uri}");
        let loc = location_of(&res);
        assert!(
            loc.starts_with("/auth/login?return_to="),
            "{uri} location: {loc}"
        );
        assert!(
            loc.contains(expected_dest),
            "{uri} must preserve destination, got {loc}"
        );
    }
}

#[tokio::test]
async fn return_to_preserves_path_and_query() {
    let app = build_router(setup_state());
    let res = app
        .oneshot(req_html(
            "/reference/backend/agency-service?tab=operations&env=dev",
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let loc = location_of(&res);
    // return_to value is percent-encoded; decode and verify round-trip.
    let encoded = loc.rsplit('=').next().unwrap();
    let decoded = urlencoding_decode(encoded);
    assert_eq!(
        decoded,
        "/reference/backend/agency-service?tab=operations&env=dev"
    );
}

fn urlencoding_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap();
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap()
}

#[tokio::test]
async fn anonymous_api_spec_static_and_proxy_stay_json_401() {
    let app = build_router(setup_state());
    // Even with an HTML Accept header, non-page paths must NOT redirect —
    // API clients and asset fetches keep receiving JSON 401.
    for uri in [
        "/api/v1/catalog",
        "/api/v1/services/agent-service",
        "/specs/development/agent-bff/openapi.json",
        "/static/scripts/portal.js",
        "/static/scalar.min.js",
        "/dev-api/agent/api/v1/agents",
        "/agent-bff/api/v1/me",
    ] {
        let res = app.clone().oneshot(req_html(uri)).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{uri}");
        assert!(location_of(&res).is_empty(), "{uri} must not redirect");
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("UNAUTHENTICATED"), "{uri} body: {text}");
    }
}

#[tokio::test]
async fn authenticated_session_gets_page_without_redirect() {
    let state = setup_state();
    session(&state, "viewer-html", vec![]);
    let app = build_router(state);
    let res = app
        .clone()
        .oneshot({
            let mut r = req_html("/");
            r.headers_mut().insert(
                header::COOKIE,
                "portal_session=viewer-html".parse().unwrap(),
            );
            r
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(location_of(&res).is_empty());
}

#[tokio::test]
async fn bogus_or_expired_session_cookie_redirects_html_navigation() {
    let app = build_router(setup_state());
    // Unknown session id behaves exactly like no session for HTML nav.
    let mut r = req_html("/reference/frontend/agent-bff");
    r.headers_mut().insert(
        header::COOKIE,
        "portal_session=does-not-exist".parse().unwrap(),
    );
    let res = app.oneshot(r).await.unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    assert!(location_of(&res).contains("/auth/login?return_to="));
}

#[tokio::test]
async fn auth_endpoints_never_redirect_into_middleware() {
    let app = build_router(setup_state());
    // /auth/login is public and must redirect to Keycloak, never loop back
    // through require_session.
    let res = app.clone().oneshot(req_html("/auth/login")).await.unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let loc = location_of(&res);
    assert!(
        loc.contains("idp.test/realms/test/protocol/openid-connect/auth"),
        "login must go to issuer, got {loc}"
    );
    // Health probes remain public 200.
    for uri in ["/health/live", "/health/ready"] {
        let res = app.clone().oneshot(req(uri)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{uri}");
    }
}

// ---------------------------------------------------------------------------
// BFF session-establishment vs API-execution authorization split
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bff_login_requires_portal_session_not_tester_role() {
    let state = setup_state();
    // Authenticated portal user WITHOUT portal-api-tester role.
    session(&state, "viewer-bff", vec![]);
    let app = build_router(state);

    // BFF login bootstrap is PortalAuthenticated: the request must pass the
    // portal auth layer and reach the upstream (which fails in test env ->
    // 5xx, proving it was NOT blocked by 401/403).
    for uri in [
        "/agent-bff/auth/login?return_to=%2F",
        "/agent-bff/auth/callback?code=x&state=y",
        "/agent-bff/auth/session",
        "/agent-bff/auth/me",
    ] {
        let res = app
            .clone()
            .oneshot({
                let mut r = req(uri);
                r.headers_mut()
                    .insert(header::COOKIE, "portal_session=viewer-bff".parse().unwrap());
                r
            })
            .await
            .unwrap();
        assert!(
            res.status().is_server_error() || res.status().is_redirection(),
            "{uri} should reach upstream, got {}",
            res.status()
        );
    }

    // But actual BFF API execution still requires the tester role -> 403.
    let res = app
        .clone()
        .oneshot({
            let mut r = req("/agent-bff/api/v1/me");
            r.headers_mut()
                .insert(header::COOKIE, "portal_session=viewer-bff".parse().unwrap());
            r
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn anonymous_bff_login_denied_but_html_nav_redirects() {
    let app = build_router(setup_state());
    // fetch/XHR-style request (no HTML accept) -> JSON 401.
    let res = app
        .clone()
        .oneshot(req("/agent-bff/auth/login"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Browser navigation -> 303 into portal login with return_to preserved.
    let res = app
        .clone()
        .oneshot(req_html(
            "/agent-bff/auth/login?return_to=%2Freference%2Ffrontend%2Fagent-bff",
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let loc = location_of(&res);
    assert!(loc.starts_with("/auth/login?return_to="), "loc={loc}");
    assert!(loc.contains("agent-bff"), "loc={loc}");
}

#[tokio::test]
async fn bff_auth_subpaths_are_not_allowlisted() {
    let state = setup_state();
    session(&state, "tester-bff", vec!["portal-api-tester"]);
    let app = build_router(state);

    // /agent-bff/auth/session/extra and similar are NOT session routes —
    // they fall through to the tester-gated wildcard. With a tester session
    // they reach upstream; the important guarantee is they never bypass
    // PortalApiTester for non-testers (covered above). Verify the exact
    // allowlist does not accidentally match prefixes: a non-tester gets 403.
    let state2 = setup_state();
    session(&state2, "viewer-bff2", vec![]);
    let app2 = build_router(state2);
    for uri in [
        "/agent-bff/auth/session/extra",
        "/agent-bff/auth",
        "/agent-bff/authentication/login",
        "/agent-bff/me",
    ] {
        let res = app2
            .clone()
            .oneshot({
                let mut r = req(uri);
                r.headers_mut().insert(
                    header::COOKIE,
                    "portal_session=viewer-bff2".parse().unwrap(),
                );
                r
            })
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN, "{uri}");
    }
    // Sanity: tester session on same paths reaches upstream (not 403).
    let res = app
        .clone()
        .oneshot({
            let mut r = req("/agent-bff/auth/session/extra");
            r.headers_mut()
                .insert(header::COOKIE, "portal_session=tester-bff".parse().unwrap());
            r
        })
        .await
        .unwrap();
    assert!(
        res.status().is_server_error()
            || res.status().is_success()
            || res.status() == StatusCode::NOT_FOUND,
        "tester should reach upstream, got {}",
        res.status()
    );
}

#[tokio::test]
async fn non_html_methods_do_not_redirect() {
    let app = build_router(setup_state());
    // POST to a page path is not a navigation → JSON 401.
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header(header::ACCEPT, "text/html")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // POST / has no route under the protected layer → 405 from router, but
    // crucially it must NOT be a login redirect. Accept either 405 or 401.
    assert!(
        res.status() == StatusCode::METHOD_NOT_ALLOWED || res.status() == StatusCode::UNAUTHORIZED,
        "unexpected {}",
        res.status()
    );
    assert!(location_of(&res).is_empty());
}
