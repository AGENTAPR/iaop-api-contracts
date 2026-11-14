use crate::errors::PortalError;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::Path,
};

// ---------------------------------------------------------------------------
// Core catalog types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceCatalog {
    pub schema_version: u32,
    pub services: Vec<ServiceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceEntry {
    pub id: String,
    pub name: String,
    pub repository: String,
    pub environment: String,
    pub visibility: String,
    /// Legacy maturity field — kept for backward compatibility.
    pub maturity: String,
    pub spec_path: String,
    pub source_ref: String,
    #[serde(default)]
    pub source_commit: Option<String>,
    pub api_version: String,
    pub enabled: bool,
    #[serde(default)]
    pub domain_owner: Option<String>,
    #[serde(default)]
    pub server_url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,

    // ---- V2 fields --------------------------------------------------------
    /// "frontend" | "backend"
    #[serde(default)]
    pub catalog: Option<String>,
    /// "agent" | "office" | "customer" | "shared" | "platform"
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub bounded_context: Option<String>,
    /// "active" | "proposed" | "deprecated" | "blocked"
    #[serde(default)]
    pub lifecycle: Option<String>,
    /// "implemented_verified" | "implemented_unverified" | "proposed" | "not_started"
    #[serde(default)]
    pub implementation_status: Option<String>,
    /// "deployed" | "not_deployed" | "unknown"
    #[serde(default)]
    pub deployment_status: Option<String>,
    /// "e2e_verified" | "unit_verified" | "not_verified" | "unknown"
    #[serde(default)]
    pub verification_status: Option<String>,
}

// ---------------------------------------------------------------------------
// V2 dual-catalog response types
// ---------------------------------------------------------------------------

/// A group of services within one catalog section (e.g. a bounded context).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogGroup {
    pub id: String,
    pub label: String,
    pub services: Vec<ServiceEntry>,
}

/// Top-level dual-catalog API response separating frontend and backend services.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DualCatalogResponse {
    pub schema_version: u32,
    pub frontend: Vec<CatalogGroup>,
    pub backend: Vec<CatalogGroup>,
}

// ---------------------------------------------------------------------------
// Scalar integration types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScalarConfig {
    pub spec: ScalarSpecRef,
    pub service_meta: ServiceMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarSpecRef {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMeta {
    pub id: String,
    pub name: String,
    pub lifecycle: String,
    pub catalog: String,
    pub bounded_context: Option<String>,
    pub channel: Option<String>,
    pub api_version: String,
    pub repository: String,
    pub domain_owner: Option<String>,
    pub description: Option<String>,
    pub server_url: Option<String>,
    pub maturity: String,
}

// ---------------------------------------------------------------------------
// Catalog loading & validation
// ---------------------------------------------------------------------------

/// Loads and validates the service catalog from disk.
pub fn load_and_validate_catalog(
    catalog_path: &Path,
    schema_path: Option<&Path>,
) -> Result<ServiceCatalog, PortalError> {
    if !catalog_path.exists() {
        return Err(PortalError::NotFound(format!(
            "Catalog file not found at {:?}",
            catalog_path
        )));
    }

    let raw_yaml = fs::read_to_string(catalog_path)
        .map_err(|e| PortalError::Catalog(format!("Failed to read catalog YAML: {}", e)))?;

    // Validate with JSON Schema if provided
    if let Some(s_path) = schema_path {
        if s_path.exists() {
            validate_catalog_schema(&raw_yaml, s_path)?;
        }
    }

    let catalog: ServiceCatalog = serde_yaml::from_str(&raw_yaml)
        .map_err(|e| PortalError::Catalog(format!("Failed to parse catalog YAML: {}", e)))?;

    // Check for unique service IDs
    let mut seen_ids = HashSet::new();
    for s in &catalog.services {
        if !seen_ids.insert(&s.id) {
            return Err(PortalError::Catalog(format!(
                "Duplicate service ID detected in catalog: {}",
                s.id
            )));
        }
    }

    Ok(catalog)
}

/// Validates raw catalog YAML content against the JSON Schema.
pub fn validate_catalog_schema(raw_yaml: &str, schema_path: &Path) -> Result<(), PortalError> {
    let raw_schema = fs::read_to_string(schema_path)
        .map_err(|e| PortalError::Catalog(format!("Failed to read catalog schema JSON: {}", e)))?;

    let schema_json: serde_json::Value = serde_json::from_str(&raw_schema)
        .map_err(|e| PortalError::Catalog(format!("Invalid catalog schema JSON: {}", e)))?;

    let catalog_val: serde_json::Value = serde_yaml::from_str(raw_yaml).map_err(|e| {
        PortalError::Catalog(format!("Cannot parse YAML for schema validation: {}", e))
    })?;

    let validator = jsonschema::validator_for(&schema_json)
        .map_err(|e| PortalError::Catalog(format!("Failed to compile catalog schema: {}", e)))?;

    let mut errors = Vec::new();
    for error in validator.iter_errors(&catalog_val) {
        errors.push(format!("Property '{}': {}", error.instance_path, error));
    }

    if !errors.is_empty() {
        return Err(PortalError::Catalog(format!(
            "Catalog schema validation failed: {}",
            errors.join("; ")
        )));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// V2 dual catalog builder
// ---------------------------------------------------------------------------

fn service_sort_rank(s: &ServiceEntry) -> u8 {
    let lc = s.lifecycle.as_deref().unwrap_or("proposed");
    let im = s.implementation_status.as_deref().unwrap_or(&s.maturity);
    if lc == "active" || im == "implemented_verified" {
        0
    } else if im == "implemented_unverified" {
        1
    } else {
        2
    }
}

/// Builds the dual-catalog response separating frontend and backend services.
/// Services are filtered to `enabled == true` and the given `environment`.
/// Backend services are grouped by `bounded_context`; frontend services by `channel`.
pub fn build_dual_catalog(catalog: &ServiceCatalog, environment: &str) -> DualCatalogResponse {
    let eligible: Vec<&ServiceEntry> = catalog
        .services
        .iter()
        .filter(|s| s.enabled && s.environment.eq_ignore_ascii_case(environment))
        .collect();

    // ----- Frontend -----
    let frontend_services: Vec<&ServiceEntry> = eligible
        .iter()
        .filter(|s| s.catalog.as_deref() == Some("frontend"))
        .copied()
        .collect();

    // Group frontend by channel label
    let mut fe_groups: BTreeMap<String, Vec<&ServiceEntry>> = BTreeMap::new();
    for s in &frontend_services {
        let group_key = s.channel.clone().unwrap_or_else(|| "other".to_string());
        fe_groups.entry(group_key).or_default().push(s);
    }

    let mut frontend: Vec<CatalogGroup> = fe_groups
        .into_iter()
        .map(|(key, mut svcs)| {
            svcs.sort_by(|a, b| {
                service_sort_rank(a)
                    .cmp(&service_sort_rank(b))
                    .then_with(|| a.name.cmp(&b.name))
            });
            let label = channel_label(&key);
            CatalogGroup {
                id: key,
                label,
                services: svcs.into_iter().cloned().collect(),
            }
        })
        .collect();
    frontend.sort_by_key(|g| g.id.clone());

    // ----- Backend -----
    let backend_services: Vec<&ServiceEntry> = eligible
        .iter()
        .filter(|s| s.catalog.as_deref().unwrap_or("backend") == "backend")
        .copied()
        .collect();

    // Group backend by bounded_context
    let mut be_groups: BTreeMap<String, Vec<&ServiceEntry>> = BTreeMap::new();
    for s in &backend_services {
        let group_key = s
            .bounded_context
            .clone()
            .unwrap_or_else(|| "Platform".to_string());
        be_groups.entry(group_key).or_default().push(s);
    }

    let mut backend: Vec<CatalogGroup> = be_groups
        .into_iter()
        .map(|(key, mut svcs)| {
            svcs.sort_by(|a, b| {
                service_sort_rank(a)
                    .cmp(&service_sort_rank(b))
                    .then_with(|| a.name.cmp(&b.name))
            });
            CatalogGroup {
                id: slugify(&key),
                label: key,
                services: svcs.into_iter().cloned().collect(),
            }
        })
        .collect();
    backend.sort_by_key(|g| g.label.clone());

    DualCatalogResponse {
        schema_version: catalog.schema_version,
        frontend,
        backend,
    }
}

fn channel_label(channel: &str) -> String {
    match channel {
        "agent" => "Agent Channel".to_string(),
        "office" => "Operations / Back Office".to_string(),
        "customer" => "Customer Channel".to_string(),
        "shared" => "Shared".to_string(),
        "platform" => "Platform".to_string(),
        other => other.to_string(),
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .replace(' ', "-")
        .replace('&', "and")
        .replace('/', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

// ---------------------------------------------------------------------------
// Scalar config builder
// ---------------------------------------------------------------------------

/// Looks up a service by ID in the catalog and returns a Scalar configuration
/// for it. Returns `None` if the service is not found or not enabled for `environment`.
pub fn build_scalar_config(
    catalog: &ServiceCatalog,
    service_id: &str,
    environment: &str,
) -> Option<ScalarConfig> {
    let entry = catalog.services.iter().find(|s| {
        s.id == service_id && s.enabled && s.environment.eq_ignore_ascii_case(environment)
    })?;

    let spec_url = format!("/{}", entry.spec_path);

    Some(ScalarConfig {
        spec: ScalarSpecRef { url: spec_url },
        service_meta: ServiceMeta {
            id: entry.id.clone(),
            name: entry.name.clone(),
            lifecycle: entry
                .lifecycle
                .clone()
                .unwrap_or_else(|| "proposed".to_string()),
            catalog: entry
                .catalog
                .clone()
                .unwrap_or_else(|| "backend".to_string()),
            bounded_context: entry.bounded_context.clone(),
            channel: entry.channel.clone(),
            api_version: entry.api_version.clone(),
            repository: entry.repository.clone(),
            domain_owner: entry.domain_owner.clone(),
            description: entry.description.clone(),
            server_url: entry.server_url.clone(),
            maturity: entry.maturity.clone(),
        },
    })
}
