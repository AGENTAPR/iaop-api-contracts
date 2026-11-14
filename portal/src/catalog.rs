use crate::errors::PortalError;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use utoipa_swagger_ui::Url;

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
    pub maturity: String,
    pub spec_path: String,
    pub source_ref: String,
    pub source_commit: Option<String>,
    pub api_version: String,
    pub enabled: bool,
    #[serde(default)]
    pub domain_owner: Option<String>,
    #[serde(default)]
    pub server_url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwaggerUrlItem {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwaggerConfigDto {
    pub urls: Vec<SwaggerUrlItem>,
    #[serde(rename = "urls.primaryName")]
    pub primary_name: String,
    pub supported_submit_methods: Vec<String>,
    pub query_config_enabled: bool,
    pub validator_url: String,
}

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
    let mut seen_ids = std::collections::HashSet::new();
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

/// Builds the Swagger UI dropdown configuration matching Section 11.
pub fn build_swagger_urls(
    catalog: &ServiceCatalog,
    environment: &str,
    allowed_visibility: &[String],
) -> Vec<Url<'static>> {
    let mut eligible: Vec<&ServiceEntry> = catalog
        .services
        .iter()
        .filter(|s| {
            s.enabled
                && s.environment.eq_ignore_ascii_case(environment)
                && allowed_visibility
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case(&s.visibility))
        })
        .collect();

    // Sort deterministically: implemented_verified first, then name
    eligible.sort_by(|a, b| {
        let a_rank = if a.maturity == "implemented_verified" {
            0
        } else if a.maturity == "implemented_unverified" {
            1
        } else {
            2
        };
        let b_rank = if b.maturity == "implemented_verified" {
            0
        } else if b.maturity == "implemented_unverified" {
            1
        } else {
            2
        };
        a_rank.cmp(&b_rank).then_with(|| a.name.cmp(&b.name))
    });

    eligible
        .into_iter()
        .map(|s| {
            let name: &'static str = Box::leak(s.name.clone().into_boxed_str());
            let spec_url: &'static str = Box::leak(format!("/{}", s.spec_path).into_boxed_str());
            Url::new(name, spec_url)
        })
        .collect()
}

/// Builds the JSON configuration object for frontend inspection.
pub fn build_swagger_config_dto(
    catalog: &ServiceCatalog,
    environment: &str,
    allowed_visibility: &[String],
) -> SwaggerConfigDto {
    let mut eligible: Vec<&ServiceEntry> = catalog
        .services
        .iter()
        .filter(|s| {
            s.enabled
                && s.environment.eq_ignore_ascii_case(environment)
                && allowed_visibility
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case(&s.visibility))
        })
        .collect();

    eligible.sort_by(|a, b| {
        let a_rank = if a.maturity == "implemented_verified" {
            0
        } else if a.maturity == "implemented_unverified" {
            1
        } else {
            2
        };
        let b_rank = if b.maturity == "implemented_verified" {
            0
        } else if b.maturity == "implemented_unverified" {
            1
        } else {
            2
        };
        a_rank.cmp(&b_rank).then_with(|| a.name.cmp(&b.name))
    });

    let url_items: Vec<SwaggerUrlItem> = eligible
        .into_iter()
        .map(|s| SwaggerUrlItem {
            name: s.name.clone(),
            url: format!("/{}", s.spec_path),
        })
        .collect();

    let primary = url_items
        .first()
        .map(|u| u.name.clone())
        .unwrap_or_else(|| "No Services Available".to_string());

    SwaggerConfigDto {
        urls: url_items,
        primary_name: primary,
        supported_submit_methods: vec![], // Section 16: Try it out disabled
        query_config_enabled: false,      // Section 16: Prevent arbitrary remote URL loading
        validator_url: "none".to_string(), // Section 25: Offline validator disabled
    }
}
