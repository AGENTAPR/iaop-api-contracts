use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "iaop-developer-portal",
    about = "IAOP Central OpenAPI Developer Portal v2 — Scalar UI, dual-catalog, schema v2"
)]
pub struct AppConfig {
    #[arg(short, long, env = "PORT", default_value = "8080")]
    pub port: u16,

    #[arg(long, env = "HOST", default_value = "0.0.0.0")]
    pub host: String,

    #[arg(long, env = "CATALOG_PATH", default_value = "catalog/services.yaml")]
    pub catalog_path: PathBuf,

    #[arg(
        long,
        env = "CATALOG_SCHEMA_PATH",
        default_value = "catalog/catalog.schema.json"
    )]
    pub catalog_schema_path: PathBuf,

    #[arg(long, env = "SPECS_DIR", default_value = "specs")]
    pub specs_dir: PathBuf,

    #[arg(long, env = "ENVIRONMENT", default_value = "development")]
    pub environment: String,

    #[arg(
        long,
        env = "ALLOWED_VISIBILITY",
        default_value = "public,internal,restricted",
        value_delimiter = ','
    )]
    pub allowed_visibility: Vec<String>,

    #[arg(long, env = "PUBLIC_BASE_URL")]
    pub public_base_url: Option<String>,

    /// Comma-separated list of allowed CORS origins.
    /// Leave empty to allow any origin (development only).
    #[arg(
        long,
        env = "ALLOWED_CORS_ORIGINS",
        default_value = "",
        value_delimiter = ','
    )]
    pub allowed_cors_origins: Vec<String>,

    /// Path to the bundled Scalar JS file.
    #[arg(
        long,
        env = "SCALAR_JS_PATH",
        default_value = "/app/static/scalar.min.js"
    )]
    pub scalar_js_path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "0.0.0.0".to_string(),
            catalog_path: PathBuf::from("catalog/services.yaml"),
            catalog_schema_path: PathBuf::from("catalog/catalog.schema.json"),
            specs_dir: PathBuf::from("specs"),
            environment: "development".to_string(),
            allowed_visibility: vec![
                "public".to_string(),
                "internal".to_string(),
                "restricted".to_string(),
            ],
            public_base_url: None,
            allowed_cors_origins: vec![],
            scalar_js_path: PathBuf::from("/app/static/scalar.min.js"),
        }
    }
}
