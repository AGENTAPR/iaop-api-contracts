//! Controlled reverse proxy for DEV API testing.
//! Fixed upstream allowlist — no arbitrary hosts (SSRF-safe).

use axum::{
    body::{to_bytes, Body},
    extract::{OriginalUri, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use tokio::time::timeout;

use crate::{errors::PortalError, routes::AppState};

const MAX_BODY: usize = 8 * 1024 * 1024; // 8 MiB
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Headers safe to forward upstream. Everything else is dropped.
const FORWARD_HEADERS: &[&str] = &[
    "cookie",
    "content-type",
    "accept",
    "authorization",
    "x-csrf-token",
    "x-request-id",
    "accept-language",
];

/// Headers safe to return downstream. Set-Cookie is required for BFF sessions.
const RETURN_HEADERS: &[&str] = &[
    "content-type",
    "set-cookie",
    "location",
    "cache-control",
    "x-request-id",
    "www-authenticate",
];

fn upstream_for<'a>(state: &'a AppState, service: &str) -> Option<&'a str> {
    match service {
        "agent-bff" => state.config.upstream_agent_bff.as_deref(),
        "agent" => state.config.upstream_agent_service.as_deref(),
        "agency" => state.config.upstream_agency_service.as_deref(),
        "eligibility" => state.config.upstream_eligibility_service.as_deref(),
        _ => None,
    }
}

pub fn proxy_router() -> Router<AppState> {
    Router::new()
        .route("/agent-bff/{*path}", any(proxy_handler))
        .route("/dev-api/{service}/{*path}", any(proxy_handler))
}

async fn proxy_handler(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    method: Method,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, PortalError> {
    let raw = uri.path().to_string();
    // Determine (service, upstream_path) from the allowlisted prefixes.
    let (service, upstream_path) = if let Some(rest) = raw.strip_prefix("/agent-bff/") {
        ("agent-bff", format!("/{rest}"))
    } else if let Some(rest) = raw.strip_prefix("/agent-bff") {
        ("agent-bff", rest.to_string())
    } else if let Some(rest) = raw.strip_prefix("/dev-api/") {
        let mut it = rest.splitn(2, '/');
        let svc = it.next().unwrap_or("");
        let p = it.next().unwrap_or("");
        (svc, format!("/{p}"))
    } else {
        return Err(PortalError::Security("unknown proxy route".into()));
    };

    let base = upstream_for(&state, service)
        .ok_or_else(|| PortalError::NotFound(format!("service '{service}' not configured")))?;

    // Reject path traversal / encoded tricks before forwarding.
    if upstream_path.contains("..") || upstream_path.contains('\0') {
        return Err(PortalError::Security("invalid upstream path".into()));
    }

    let query = uri.query().map(|q| format!("?{q}")).unwrap_or_default();
    let url = format!("{base}{upstream_path}{query}");

    let bytes = to_bytes(body, MAX_BODY)
        .await
        .map_err(|_| PortalError::Security("request body too large".into()))?;

    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none()) // do not follow upstream redirects blindly
        .build()
        .map_err(|e| PortalError::Internal(format!("http client: {e}")))?;

    let mut req = client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes())
            .map_err(|_| PortalError::Security("bad method".into()))?,
        &url,
    );
    for name in FORWARD_HEADERS {
        if let Ok(hn) = HeaderName::from_bytes(name.as_bytes()) {
            for v in headers.get_all(&hn).iter() {
                req = req.header(&hn, v);
            }
        }
    }
    if !bytes.is_empty() {
        req = req.body(bytes.to_vec());
    }

    let upstream = timeout(TIMEOUT, req.send())
        .await
        .map_err(|_| PortalError::Internal("upstream timeout".into()))?
        .map_err(|e| PortalError::Internal(format!("upstream error: {e}")))?;

    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);
    for name in RETURN_HEADERS {
        if let Ok(hn) = HeaderName::from_bytes(name.as_bytes()) {
            for v in upstream.headers().get_all(&hn).iter() {
                if let Ok(val) = HeaderValue::from_bytes(v.as_bytes()) {
                    builder = builder.header(&hn, val);
                }
            }
        }
    }
    let resp_bytes = upstream
        .bytes()
        .await
        .map_err(|_| PortalError::Internal("upstream body read failed".into()))?;
    builder
        .body(Body::from(resp_bytes))
        .map_err(|e| PortalError::Internal(format!("response build: {e}")))
}
