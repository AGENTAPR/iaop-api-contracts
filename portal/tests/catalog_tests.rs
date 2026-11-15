use iaop_developer_portal::catalog::{
    build_dual_catalog, build_scalar_config, load_and_validate_catalog, validate_catalog_schema,
};
use std::path::PathBuf;

fn catalog_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../catalog/services.yaml")
}

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../catalog/catalog.schema.json")
}

// ---------------------------------------------------------------------------
// Catalog loading and schema validation
// ---------------------------------------------------------------------------

#[test]
fn loads_and_validates_catalog_v2() {
    let cat = catalog_path();
    let schema = schema_path();

    assert!(cat.exists(), "services.yaml must exist");
    assert!(schema.exists(), "catalog.schema.json must exist");

    let catalog = load_and_validate_catalog(&cat, Some(&schema))
        .expect("services.yaml must validate against schema v2");

    assert!(
        catalog.schema_version >= 2,
        "Catalog must be schema v2 or above, got {}",
        catalog.schema_version
    );
}

#[test]
fn no_duplicate_service_ids() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    let mut seen = std::collections::HashSet::new();
    for svc in &catalog.services {
        assert!(
            seen.insert(&svc.id),
            "Duplicate service ID found: {}",
            svc.id
        );
    }
}

#[test]
fn exactly_one_active_agent_bff() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    let bff_entries: Vec<_> = catalog
        .services
        .iter()
        .filter(|s| s.id.contains("agent-bff"))
        .collect();

    assert_eq!(
        bff_entries.len(),
        1,
        "Expected exactly one agent-bff entry, found {}: {:?}",
        bff_entries.len(),
        bff_entries.iter().map(|s| &s.id).collect::<Vec<_>>()
    );

    assert_eq!(bff_entries[0].id, "agent-bff");
    assert!(
        !catalog
            .services
            .iter()
            .any(|s| s.id == "agent-bff-proposed"),
        "agent-bff-proposed must not exist in V2 catalog"
    );
}

#[test]
fn agent_bff_in_frontend_catalog() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    let bff = catalog
        .services
        .iter()
        .find(|s| s.id == "agent-bff")
        .expect("agent-bff must be in catalog");

    assert_eq!(
        bff.catalog.as_deref(),
        Some("frontend"),
        "agent-bff must be in the frontend catalog"
    );
}

#[test]
fn agent_service_in_backend_catalog() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    let svc = catalog
        .services
        .iter()
        .find(|s| s.id == "agent-service")
        .expect("agent-service must be in catalog");

    assert_eq!(
        svc.catalog.as_deref(),
        Some("backend"),
        "agent-service must be in the backend catalog"
    );
}

#[test]
fn all_services_have_lifecycle_field() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    for svc in &catalog.services {
        assert!(
            svc.lifecycle.is_some(),
            "Service '{}' is missing the lifecycle field",
            svc.id
        );
    }
}

#[test]
fn all_services_have_catalog_field() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    for svc in &catalog.services {
        assert!(
            svc.catalog.is_some(),
            "Service '{}' is missing the catalog field",
            svc.id
        );
        let cat = svc.catalog.as_deref().unwrap();
        assert!(
            cat == "frontend" || cat == "backend",
            "Service '{}' has invalid catalog value: {}",
            svc.id,
            cat
        );
    }
}

// ---------------------------------------------------------------------------
// Dual catalog builder
// ---------------------------------------------------------------------------

#[test]
fn dual_catalog_groups_correctly() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    let dual = build_dual_catalog(&catalog, "development");

    // Frontend groups must exist and contain agent-bff
    assert!(
        !dual.frontend.is_empty(),
        "Frontend catalog must not be empty"
    );

    let all_frontend_ids: Vec<&str> = dual
        .frontend
        .iter()
        .flat_map(|g| g.services.iter().map(|s| s.id.as_str()))
        .collect();

    assert!(
        all_frontend_ids.contains(&"agent-bff"),
        "agent-bff must appear in frontend dual catalog"
    );
    assert!(
        !all_frontend_ids.contains(&"agent-bff-proposed"),
        "agent-bff-proposed must not appear in dual catalog"
    );

    // Backend groups must exist
    assert!(
        !dual.backend.is_empty(),
        "Backend catalog must not be empty"
    );

    let all_backend_ids: Vec<&str> = dual
        .backend
        .iter()
        .flat_map(|g| g.services.iter().map(|s| s.id.as_str()))
        .collect();

    assert!(
        all_backend_ids.contains(&"agent-service"),
        "agent-service must appear in backend dual catalog"
    );

    // No service should appear in both frontend and backend
    for id in &all_frontend_ids {
        assert!(
            !all_backend_ids.contains(id),
            "Service '{}' appears in both frontend and backend catalogs",
            id
        );
    }
}

#[test]
fn dual_catalog_groups_backend_by_bounded_context() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    let dual = build_dual_catalog(&catalog, "development");

    // Check that bounded context groups exist for key domains
    let group_labels: Vec<&str> = dual.backend.iter().map(|g| g.label.as_str()).collect();

    assert!(
        group_labels.contains(&"Agent & Distribution"),
        "Expected 'Agent & Distribution' group in backend catalog, got: {:?}",
        group_labels
    );
    assert!(
        group_labels.contains(&"Sales & Insurance"),
        "Expected 'Sales & Insurance' group in backend catalog"
    );
}

#[test]
fn active_services_sort_before_proposed() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    let dual = build_dual_catalog(&catalog, "development");

    // Within the Agent & Distribution backend group, agent-service (active) should precede proposed ones
    if let Some(group) = dual
        .backend
        .iter()
        .find(|g| g.label == "Agent & Distribution")
    {
        let first_lc = group
            .services
            .first()
            .and_then(|s| s.lifecycle.as_deref())
            .unwrap_or("proposed");
        assert_eq!(
            first_lc, "active",
            "Active services must sort before proposed within a group"
        );
    }
}

// ---------------------------------------------------------------------------
// Scalar config builder
// ---------------------------------------------------------------------------

#[test]
fn scalar_config_resolves_agent_bff() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    let cfg = build_scalar_config(&catalog, "agent-bff", "development")
        .expect("agent-bff must resolve to a ScalarConfig");

    assert_eq!(cfg.service_meta.id, "agent-bff");
    assert_eq!(cfg.service_meta.catalog, "frontend");
    assert!(
        cfg.spec.url.starts_with("/specs/"),
        "Spec URL must start with /specs/"
    );
}

#[test]
fn scalar_config_returns_none_for_unknown_service() {
    let catalog = load_and_validate_catalog(&catalog_path(), Some(&schema_path())).unwrap();

    let cfg = build_scalar_config(&catalog, "nonexistent-service-xyz", "development");
    assert!(cfg.is_none(), "Unknown service must return None");
}

// ---------------------------------------------------------------------------
// Schema validator unit tests
// ---------------------------------------------------------------------------

#[test]
fn schema_validator_rejects_missing_required_fields() {
    let schema = schema_path();
    let invalid_yaml = r#"
schemaVersion: 2
services:
  - id: broken-service
    # missing name, repository, environment, visibility, maturity, catalog, lifecycle
"#;
    let result = validate_catalog_schema(invalid_yaml, &schema);
    assert!(
        result.is_err(),
        "Schema validator must reject missing required fields"
    );
}

#[test]
fn schema_validator_rejects_invalid_maturity_enum() {
    let schema = schema_path();
    let invalid_yaml = r#"
schemaVersion: 2
services:
  - id: invalid-maturity
    name: Invalid Maturity Service
    repository: AGENTAPR/invalid
    environment: development
    visibility: internal
    maturity: super_production_ready
    catalog: backend
    lifecycle: active
    specPath: specs/development/invalid/openapi.json
    sourceRef: development
    apiVersion: 1.0.0
    enabled: true
"#;
    let result = validate_catalog_schema(invalid_yaml, &schema);
    assert!(
        result.is_err(),
        "Schema validator must reject invalid maturity enum"
    );
}

#[test]
fn schema_validator_rejects_invalid_catalog_enum() {
    let schema = schema_path();
    let invalid_yaml = r#"
schemaVersion: 2
services:
  - id: bad-catalog
    name: Bad Catalog Service
    repository: AGENTAPR/bad
    environment: development
    visibility: internal
    maturity: proposed_contract
    catalog: middleware
    lifecycle: proposed
    specPath: specs/development/bad/openapi.json
    sourceRef: development
    apiVersion: 1.0.0
    enabled: true
"#;
    let result = validate_catalog_schema(invalid_yaml, &schema);
    assert!(
        result.is_err(),
        "Schema validator must reject invalid catalog enum ('middleware' is not valid)"
    );
}

#[test]
fn schema_validator_rejects_invalid_lifecycle_enum() {
    let schema = schema_path();
    let invalid_yaml = r#"
schemaVersion: 2
services:
  - id: bad-lifecycle
    name: Bad Lifecycle Service
    repository: AGENTAPR/bad
    environment: development
    visibility: internal
    maturity: proposed_contract
    catalog: backend
    lifecycle: in_progress
    specPath: specs/development/bad/openapi.json
    sourceRef: development
    apiVersion: 1.0.0
    enabled: true
"#;
    let result = validate_catalog_schema(invalid_yaml, &schema);
    assert!(
        result.is_err(),
        "Schema validator must reject invalid lifecycle enum"
    );
}
