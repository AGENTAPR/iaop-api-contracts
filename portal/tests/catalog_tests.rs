use iaop_developer_portal::catalog::{
    build_swagger_config_dto, load_and_validate_catalog, validate_catalog_schema,
};
use std::path::PathBuf;

fn get_workspace_catalog_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../catalog/services.yaml")
}

fn get_workspace_schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../catalog/catalog.schema.json")
}

#[test]
fn loads_and_validates_official_catalog() {
    let cat_path = get_workspace_catalog_path();
    let schema_path = get_workspace_schema_path();

    assert!(cat_path.exists(), "services.yaml must exist");
    assert!(schema_path.exists(), "catalog.schema.json must exist");

    let catalog = load_and_validate_catalog(&cat_path, Some(&schema_path))
        .expect("Official services.yaml must validate against schema");

    assert_eq!(catalog.schema_version, 1);
    assert!(
        catalog.services.len() >= 19,
        "Expected at least 19 services in catalog, found {}",
        catalog.services.len()
    );

    // Verify key services exist
    let ids: Vec<&str> = catalog.services.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"agent-bff"));
    assert!(ids.contains(&"agent-service"));
    assert!(ids.contains(&"agency-service"));
    assert!(ids.contains(&"sales-service"));
}

#[test]
fn schema_validator_rejects_missing_required_fields() {
    let schema_path = get_workspace_schema_path();
    let invalid_yaml = r#"
schemaVersion: 1
services:
  - id: broken-service
    # missing name, repository, environment, visibility, maturity, etc.
"#;
    let result = validate_catalog_schema(invalid_yaml, &schema_path);
    assert!(
        result.is_err(),
        "Schema validator must reject missing required fields"
    );
}

#[test]
fn schema_validator_rejects_invalid_maturity_enum() {
    let schema_path = get_workspace_schema_path();
    let invalid_yaml = r#"
schemaVersion: 1
services:
  - id: invalid-maturity
    name: Invalid Maturity Service
    repository: AGENTAPR/invalid
    environment: development
    visibility: internal
    maturity: super_production_ready # Invalid enum
    specPath: specs/development/invalid/openapi.json
    sourceRef: development
    apiVersion: 1.0.0
    enabled: true
"#;
    let result = validate_catalog_schema(invalid_yaml, &schema_path);
    assert!(
        result.is_err(),
        "Schema validator must reject invalid maturity enum"
    );
}

#[test]
fn builds_swagger_urls_with_deterministic_priority() {
    let cat_path = get_workspace_catalog_path();
    let schema_path = get_workspace_schema_path();
    let catalog = load_and_validate_catalog(&cat_path, Some(&schema_path)).unwrap();

    let dto = build_swagger_config_dto(
        &catalog,
        "development",
        &["public".into(), "internal".into(), "restricted".into()],
    );

    assert!(!dto.urls.is_empty());
    // First URLs should be implemented_verified (Agent BFF, Agent Service)
    let first = &dto.urls[0];
    let second = &dto.urls[1];
    assert!(
        first.name.contains("Implemented"),
        "First item must be implemented"
    );
    assert!(
        second.name.contains("Implemented"),
        "Second item must be implemented"
    );
}

#[test]
fn visibility_filtering_isolates_public_vs_internal_vs_restricted() {
    let cat_path = get_workspace_catalog_path();
    let schema_path = get_workspace_schema_path();
    let catalog = load_and_validate_catalog(&cat_path, Some(&schema_path)).unwrap();

    // Public only
    let public_dto = build_swagger_config_dto(&catalog, "development", &["public".into()]);
    assert!(!public_dto.urls.is_empty());
    for u in &public_dto.urls {
        assert!(
            u.name.contains("BFF") || u.name.contains("Gateway"),
            "Public catalog must only include public services"
        );
    }

    // Full visibility includes commission service (restricted)
    let all_dto = build_swagger_config_dto(
        &catalog,
        "development",
        &["public".into(), "internal".into(), "restricted".into()],
    );
    assert!(all_dto.urls.len() > public_dto.urls.len());
}

#[test]
fn swagger_config_disables_try_it_out_and_validator() {
    let cat_path = get_workspace_catalog_path();
    let schema_path = get_workspace_schema_path();
    let catalog = load_and_validate_catalog(&cat_path, Some(&schema_path)).unwrap();

    let dto = build_swagger_config_dto(
        &catalog,
        "development",
        &["public".into(), "internal".into(), "restricted".into()],
    );

    assert!(
        dto.supported_submit_methods.is_empty(),
        "supportedSubmitMethods must be empty to disable Try it out"
    );
    assert!(
        !dto.query_config_enabled,
        "queryConfigEnabled must be false to block external remote URL injection"
    );
    assert_eq!(
        dto.validator_url, "none",
        "validatorUrl must be none for offline operation"
    );
}
