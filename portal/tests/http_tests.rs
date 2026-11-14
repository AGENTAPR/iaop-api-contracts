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
    };

    AppState {
        config,
        catalog: Arc::new(catalog),
    }
}

#[tokio::test]
async fn root_redirects_to_swagger_ui() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = res
        .headers()
        .get(header::LOCATION)
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(loc, "/swagger-ui/");
}

#[tokio::test]
async fn health_endpoints_respond_with_json() {
    let state = setup_test_state();
    let app = build_router(state);

    // Live
    let req = Request::builder()
        .uri("/health/live")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");

    // Ready
    let req = Request::builder()
        .uri("/health/ready")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ready");
    assert!(json["services_count"].as_u64().unwrap() >= 19);
}

#[tokio::test]
async fn swagger_ui_html_shell_loads() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/swagger-ui/")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let headers = res.headers();
    assert_eq!(
        headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff"
    );
    assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
}

#[tokio::test]
async fn swagger_config_endpoint_serves_valid_dropdown_urls() {
    let state = setup_test_state();
    let app = build_router(state);

    let req = Request::builder()
        .uri("/api/v1/swagger-config")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let urls = json["urls"].as_array().expect("urls must be an array");
    assert!(urls.len() >= 19);

    assert_eq!(json["supportedSubmitMethods"].as_array().unwrap().len(), 0);
    assert_eq!(json["queryConfigEnabled"], false);
    assert_eq!(json["validatorUrl"], "none");
}

#[tokio::test]
async fn catalog_endpoint_serves_complete_catalog() {
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

    assert_eq!(json["schemaVersion"], 1);
    let svcs = json["services"].as_array().unwrap();
    assert!(svcs.len() >= 19);
}

#[tokio::test]
async fn serves_agent_bff_and_agent_service_specs() {
    let state = setup_test_state();
    let app = build_router(state);

    // Agent BFF
    let req = Request::builder()
        .uri("/specs/development/agent-bff/openapi.json")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/json"
    );
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["openapi"], "3.1.0");
    assert!(json["info"]["title"].as_str().unwrap().contains("AGENA"));
    assert!(json["paths"]["/me"].is_object());
    assert!(json["paths"]["/me/today"].is_object());

    // Agent Service
    let req = Request::builder()
        .uri("/specs/development/agent-service/openapi.json")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["openapi"], "3.1.0");
    assert!(json["info"]["title"]
        .as_str()
        .unwrap()
        .contains("Agent Registry"));
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
