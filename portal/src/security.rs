use crate::errors::PortalError;
use axum::{
    http::{header, HeaderValue, Request, Response},
    middleware::Next,
};
use std::path::{Component, Path, PathBuf};

/// Sanitizes a requested path against directory traversal attacks.
/// Ensures the resolved path stays strictly within `base_dir`.
pub fn sanitize_path(base_dir: &Path, requested_rel_path: &Path) -> Result<PathBuf, PortalError> {
    // Reject absolute paths
    if requested_rel_path.is_absolute() {
        return Err(PortalError::Security("Absolute paths are forbidden".into()));
    }

    // Inspect components: reject ParentDir (..) and null bytes
    for comp in requested_rel_path.components() {
        match comp {
            Component::ParentDir => {
                return Err(PortalError::Security(
                    "Path traversal sequences (..) are forbidden".into(),
                ));
            }
            Component::Normal(os_str) => {
                if let Some(s) = os_str.to_str() {
                    if s.contains('\0') {
                        return Err(PortalError::Security(
                            "Null byte in path is forbidden".into(),
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    let resolved = base_dir.join(requested_rel_path);

    // Canonicalize base_dir
    let canonical_base = base_dir
        .canonicalize()
        .map_err(|e| PortalError::Internal(format!("Base directory inaccessible: {}", e)))?;

    // Canonicalize target file
    let canonical_target = resolved.canonicalize().map_err(|_| {
        PortalError::NotFound(format!("Spec file not found: {:?}", requested_rel_path))
    })?;

    if !canonical_target.starts_with(&canonical_base) {
        return Err(PortalError::Security(
            "Access outside designated specs directory is prohibited".into(),
        ));
    }

    Ok(canonical_target)
}

/// Applies standard enterprise security headers to all responses.
///
/// The CSP is compatible with Scalar API Reference served from `/static/scalar.min.js`
/// (same origin). No CDN URLs are needed at runtime.
pub async fn security_headers_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );
    // Scalar JS is served from /static/ (same origin) — no external script-src needed.
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; \
             connect-src 'self'; \
             worker-src 'none'; \
             frame-ancestors 'none';",
        ),
    );

    response
}
