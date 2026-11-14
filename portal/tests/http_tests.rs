use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

use iaop_developer_portal::{build_router, load_and_validate_catalog, AppConfig, AppState};

fn setup_test_state() -> AppState {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog_path = manifest_dir.join("../catalog/services.yaml");
    let schema_path = manifest_dir.join("../catalog/catalog.schema.json");
    let specs_dir = manifest_dir.join("../specs");

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
        // For tests, Scalar JS won't exist on disk — the route will return 500 if hit,
        // but health/catalog/specs tests don't exercise it.
        scalar_js_path: PathBuf::from("portal/tests/fixtures/scalar.min.js"),
    };

    AppState {
        config,
        catalog: Arc::new(catalog),
    }
}

// ---------------------------------------------------------------------------
// Landing page and routing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn root_serves_landing_page_html() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).unwrap();
    assert!(
        html.contains("IAOP Developer Portal"),
        "Landing page must contain portal title"
    );
    assert!(
        html.contains("Frontend APIs"),
        "Landing page must reference Frontend APIs section"
    );
}

#[tokio::test]
async fn swagger_ui_redirects_to_reference_frontend() {
    let state = setup_test_state();
    let app = build_router(state.clone());

    // /swagger-ui (no trailing slash)
    let req = Request::builder()
        .uri("/swagger-ui")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = res
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(loc, "/reference/frontend");

    // /swagger-ui/ (trailing slash)
    let req = Request::builder()
        .uri("/swagger-ui/")
        .body(Body::empty())
        .unwrap();
    let res = build_router(setup_test_state()).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = res
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(loc, "/reference/frontend");
}

#[tokio::test]
async fn frontend_index_redirects_to_first_frontend_service() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/reference/frontend")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = res
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        loc.starts_with("/reference/frontend/"),
        "Must redirect to a specific frontend service, got: {}",
        loc
    );
}

#[tokio::test]
async fn reference_frontend_agent_bff_serves_scalar_html() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/reference/frontend/agent-bff")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).unwrap();

    // Must reference the Scalar script from same origin
    assert!(
        html.contains("/static/scalar.min.js"),
        "Scalar page must load JS from /static/scalar.min.js (same origin)"
    );
    // Must reference the spec URL
    assert!(
        html.contains("/specs/development/agent-bff/openapi.json"),
        "Scalar page must point data-url at the spec file"
    );
    // Must use id="api-reference" for Scalar mount point
    assert!(
        html.contains(r#"id="api-reference""#),
        "Scalar page must use id=api-reference script tag"
    );
    // Must NOT reference any CDN
    assert!(
        !html.contains("cdn.jsdelivr.net"),
        "Scalar page must not reference external CDN"
    );
    assert!(
        !html.contains("unpkg.com"),
        "Scalar page must not reference unpkg CDN"
    );
}

#[tokio::test]
async fn reference_backend_serves_scalar_html() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/reference/backend/agent-service")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let html = std::str::from_utf8(&body).unwrap();
    assert!(html.contains("/static/scalar.min.js"));
    assert!(html.contains("/specs/development/agent-service/openapi.json"));
}

#[tokio::test]
async fn reference_nonexistent_service_returns_404() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/reference/frontend/nonexistent-xyz-9999")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Health probes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_live_returns_ok() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/health/live")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn health_ready_reports_service_count_and_schema_version() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/health/ready")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ready");
    assert!(
        json["services_count"].as_u64().unwrap() >= 18,
        "Must report at least 18 services (19 minus the removed agent-bff-proposed)"
    );
    assert_eq!(
        json["schema_version"].as_u64().unwrap(),
        2,
        "schema_version must be 2"
    );
}

// ---------------------------------------------------------------------------
// Catalog API endpoints (V2 dual structure)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn catalog_endpoint_returns_dual_structure() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/api/v1/catalog")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Must have frontend and backend top-level keys
    assert!(
        json["frontend"].is_array(),
        "catalog response must have 'frontend' array"
    );
    assert!(
        json["backend"].is_array(),
        "catalog response must have 'backend' array"
    );

    // Schema version must be 2
    assert_eq!(
        json["schemaVersion"].as_u64().unwrap(),
        2,
        "schemaVersion must be 2"
    );

    // agent-bff must be in frontend, not backend
    let empty_vec = vec![];
    let all_svc_ids: Vec<String> = json["frontend"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|group| group["services"].as_array().unwrap_or(&empty_vec))
        .map(|s| s["id"].as_str().unwrap_or("").to_string())
        .collect();

    assert!(
        all_svc_ids.contains(&"agent-bff".to_string()),
        "agent-bff must be in frontend catalog response"
    );
    assert!(
        !all_svc_ids.contains(&"agent-bff-proposed".to_string()),
        "agent-bff-proposed must NOT be in catalog response"
    );
}

#[tokio::test]
async fn catalog_frontend_endpoint_returns_array() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/api/v1/catalog/frontend")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        json.is_array(),
        "/api/v1/catalog/frontend must return an array"
    );
}

#[tokio::test]
async fn catalog_backend_endpoint_returns_array() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/api/v1/catalog/backend")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        json.is_array(),
        "/api/v1/catalog/backend must return an array"
    );
}

#[tokio::test]
async fn service_meta_endpoint_returns_scalar_config() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/api/v1/services/agent-bff")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["serviceMeta"]["id"], "agent-bff");
    assert_eq!(json["serviceMeta"]["catalog"], "frontend");
    assert!(json["spec"]["url"].as_str().unwrap().starts_with("/specs/"));
}

#[tokio::test]
async fn service_spec_redirect_works() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/api/v1/services/agent-bff/spec")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = res
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        loc.contains("agent-bff"),
        "Spec redirect must point to agent-bff spec"
    );
}

// ---------------------------------------------------------------------------
// Spec file serving
// ---------------------------------------------------------------------------

#[tokio::test]
async fn serves_agent_bff_spec_json() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/specs/development/agent-bff/openapi.json")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["openapi"], "3.1.0");
    assert!(json["info"]["title"].as_str().unwrap().contains("AGENA"));
}

#[tokio::test]
async fn serves_agent_service_spec_json() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/specs/development/agent-service/openapi.json")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["openapi"], "3.1.0");
}

#[tokio::test]
async fn nonexistent_spec_returns_404() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/specs/development/nonexistent-service-123/openapi.json")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
