use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

use iaop_developer_portal::{
    build_router, load_and_validate_catalog, security::sanitize_path, AppConfig, AppState,
};

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

#[test]
fn sanitize_path_rejects_parent_dir_traversal() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let specs_dir = manifest_dir.join("../specs");

    let evil_path = std::path::Path::new("../Cargo.toml");
    let res = sanitize_path(&specs_dir, evil_path);
    assert!(res.is_err(), "Must reject ../ sequences");
}

#[test]
fn sanitize_path_rejects_absolute_paths() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let specs_dir = manifest_dir.join("../specs");

    #[cfg(windows)]
    let evil_path = std::path::Path::new("C:\\Windows\\System32\\drivers\\etc\\hosts");
    #[cfg(not(windows))]
    let evil_path = std::path::Path::new("/etc/passwd");

    let res = sanitize_path(&specs_dir, evil_path);
    assert!(res.is_err(), "Must reject absolute paths");
}

#[tokio::test]
async fn http_rejects_path_traversal_attempts() {
    let state = setup_test_state();
    let app = build_router(state);

    // Attempt to access Cargo.toml via traversal
    let req = Request::builder()
        .uri("/specs/development/../../Cargo.toml")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status() == StatusCode::FORBIDDEN || res.status() == StatusCode::NOT_FOUND,
        "HTTP traversal must be blocked"
    );

    // Attempt to access dotfiles
    let req = Request::builder()
        .uri("/specs/development/../.gitignore")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert!(
        res.status() == StatusCode::FORBIDDEN || res.status() == StatusCode::NOT_FOUND,
        "HTTP traversal must be blocked"
    );
}
