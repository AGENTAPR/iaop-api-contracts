use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "iaop-developer-portal",
    about = "Centralized Swagger UI & Enterprise OpenAPI Developer Portal for IAOP"
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
        }
    }
}
