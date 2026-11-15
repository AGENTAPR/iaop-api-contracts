use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum PortalError {
    #[error("Catalog error: {0}")]
    Catalog(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Security violation: {0}")]
    Security(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

impl IntoResponse for PortalError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            PortalError::NotFound(msg) => (StatusCode::NOT_FOUND, "RESOURCE_NOT_FOUND", msg),
            PortalError::Security(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN_ACCESS", msg),
            PortalError::Catalog(msg) => (StatusCode::BAD_REQUEST, "CATALOG_ERROR", msg),
            PortalError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg)
            }
        };

        let body = Json(ErrorResponse {
            error: ErrorDetail {
                code: code.to_string(),
                message,
            },
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        (status, body).into_response()
    }
}
