//! Developer Portal SSO: OIDC Authorization Code + PKCE against Keycloak.
//! Server-side sessions (in-memory for DEV; documented limitation).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use iaop_security::jwks::{JwtValidator, RemoteJwksProvider};

use crate::{errors::PortalError, routes::AppState};

pub const SESSION_COOKIE: &str = "portal_session";
pub const FLOW_COOKIE: &str = "portal_oidc_flow";
const SESSION_TTL: Duration = Duration::from_secs(8 * 3600);
const FLOW_TTL: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct PortalSession {
    pub session_id: String,
    pub subject: String,
    pub display_name: String,
    pub email: Option<String>,
    /// Verified realm roles from the validated id_token.
    pub roles: Vec<String>,
    pub created_at: Instant,
}

impl PortalSession {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

#[derive(Clone)]
pub struct PendingFlow {
    pub state: String,
    pub nonce: String,
    pub code_verifier: String,
    pub return_to: String,
    pub created_at: Instant,
}

#[derive(Default)]
pub struct SessionStore {
    sessions: Mutex<HashMap<String, PortalSession>>,
    flows: Mutex<HashMap<String, PendingFlow>>,
}

impl SessionStore {
    fn gc(&self) {
        let now = Instant::now();
        self.sessions
            .lock()
            .unwrap()
            .retain(|_, s| now.duration_since(s.created_at) < SESSION_TTL);
        self.flows
            .lock()
            .unwrap()
            .retain(|_, f| now.duration_since(f.created_at) < FLOW_TTL);
    }

    pub fn put_session(&self, s: PortalSession) {
        self.gc();
        self.sessions
            .lock()
            .unwrap()
            .insert(s.session_id.clone(), s);
    }

    pub fn get_session(&self, id: &str) -> Option<PortalSession> {
        self.sessions.lock().unwrap().get(id).cloned()
    }

    pub fn drop_session(&self, id: &str) {
        self.sessions.lock().unwrap().remove(id);
    }

    pub fn put_flow(&self, f: PendingFlow) {
        self.flows.lock().unwrap().insert(f.state.clone(), f);
    }

    /// Single-use: removes and returns the pending flow.
    pub fn take_flow(&self, state: &str) -> Option<PendingFlow> {
        self.flows.lock().unwrap().remove(state)
    }
}

pub type SharedStore = Arc<SessionStore>;

fn rand_b64(len: usize) -> String {
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(&buf)
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn set_cookie(name: &str, value: &str, max_age: i64, secure: bool) -> String {
    format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{}",
        if secure { "; Secure" } else { "" }
    )
}

fn clear_cookie(name: &str, secure: bool) -> String {
    format!(
        "{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        if secure { "; Secure" } else { "" }
    )
}

fn read_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(';'))
        .filter_map(|p| p.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
}

fn secure_cookies(state: &AppState) -> bool {
    state
        .config
        .public_base_url
        .as_deref()
        .map(|u| u.starts_with("https://"))
        .unwrap_or(true)
}

fn sanitize_return_to(raw: Option<&str>) -> String {
    match raw {
        Some(p) if p.starts_with('/') && !p.starts_with("//") && !p.contains("\\") => {
            p.chars().take(512).collect()
        }
        _ => "/".to_string(),
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub return_to: Option<String>,
}

/// GET /auth/login — start OIDC auth-code + PKCE.
pub async fn login(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<LoginQuery>,
) -> Result<Response, PortalError> {
    let cfg = &state.config;
    let (issuer, client_id, redirect) = match (
        &cfg.oidc_issuer,
        &cfg.oidc_client_id,
        &cfg.oidc_redirect_uri,
    ) {
        (Some(i), Some(c), Some(r)) => (i, c, r),
        _ => return Err(PortalError::Internal("OIDC not configured".into())),
    };

    let flow_state = rand_b64(24);
    let nonce = rand_b64(24);
    let verifier = rand_b64(48);
    let return_to = sanitize_return_to(q.return_to.as_deref());

    state.auth_store.put_flow(PendingFlow {
        state: flow_state.clone(),
        nonce: nonce.clone(),
        code_verifier: verifier.clone(),
        return_to,
        created_at: Instant::now(),
    });

    let url = format!(
        "{issuer}/protocol/openid-connect/auth?response_type=code&client_id={client_id}&redirect_uri={}&scope=openid%20profile%20email%20roles&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",
        urlencoding(redirect),
        urlencoding(&flow_state),
        urlencoding(&nonce),
        urlencoding(&pkce_challenge(&verifier)),
    );

    let mut resp = Redirect::to(&url).into_response();
    let secure = secure_cookies(&state);
    resp.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(FLOW_COOKIE, &flow_state, 600, secure)
            .parse()
            .unwrap(),
    );
    Ok(resp)
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PortalClaims {
    sub: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

/// GET /auth/callback — exchange code, validate id_token, create session.
pub async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<CallbackQuery>,
) -> Result<Response, PortalError> {
    let cfg = &state.config;
    let (issuer, client_id, redirect, secret) = match (
        &cfg.oidc_issuer,
        &cfg.oidc_client_id,
        &cfg.oidc_redirect_uri,
        &cfg.oidc_client_secret,
    ) {
        (Some(i), Some(c), Some(r), Some(s)) => (i, c, r, s),
        _ => return Err(PortalError::Internal("OIDC not configured".into())),
    };

    if let Some(err) = q.error {
        return Ok(Redirect::to(&format!("/?auth_error={}", urlencoding(&err))).into_response());
    }
    let (code, cb_state) = match (q.code, q.state) {
        (Some(c), Some(s)) => (c, s),
        _ => return Err(PortalError::Security("missing code or state".into())),
    };

    // State must match BOTH the pending flow and the flow cookie (binds browser).
    let cookie_state = read_cookie(&headers, FLOW_COOKIE).unwrap_or("");
    if cookie_state.is_empty() || cookie_state != cb_state {
        return Err(PortalError::Security("OIDC state mismatch".into()));
    }
    let flow = state
        .auth_store
        .take_flow(&cb_state)
        .ok_or_else(|| PortalError::Security("unknown or expired OIDC state".into()))?;
    if flow.state != cb_state {
        return Err(PortalError::Security("OIDC state mismatch".into()));
    }

    // Exchange code at token endpoint.
    let token_url = format!("{issuer}/protocol/openid-connect/token");
    let http = reqwest::Client::new();
    let resp = http
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id.as_str()),
            ("client_secret", secret.as_str()),
            ("code", code.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("code_verifier", flow.code_verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| PortalError::Internal(format!("token endpoint unreachable: {e}")))?;
    if !resp.status().is_success() {
        return Err(PortalError::Security("token exchange failed".into()));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| PortalError::Internal("invalid token response".into()))?;
    let id_token = body["id_token"]
        .as_str()
        .ok_or_else(|| PortalError::Internal("no id_token in response".into()))?;

    // Validate id_token signature/iss/aud/exp via JWKS.
    let jwks_url = cfg
        .oidc_jwks_url
        .clone()
        .unwrap_or_else(|| format!("{issuer}/protocol/openid-connect/certs"));
    let validator: JwtValidator<RemoteJwksProvider> = JwtValidator::new(
        RemoteJwksProvider::new(jwks_url),
        issuer.clone(),
        client_id.clone(),
    );
    let claims: PortalClaims = validator
        .validate(id_token)
        .await
        .map_err(|e| PortalError::Security(format!("id_token validation failed: {e:?}")))?;

    // Nonce check: Keycloak echoes nonce in id_token.
    let raw: serde_json::Value = jsonwebtoken_decode_payload(id_token)
        .ok_or_else(|| PortalError::Security("cannot decode id_token payload".into()))?;
    if raw["nonce"].as_str() != Some(flow.nonce.as_str()) {
        return Err(PortalError::Security("OIDC nonce mismatch".into()));
    }

    // Realm roles: fetched from Keycloak's UserInfo endpoint using the
    // access token as a Bearer credential. Keycloak validates the access
    // token server-side before returning claims, so only verified roles
    // are accepted. We never trust the access token payload decoded without
    // signature verification.
    let userinfo_url = format!("{issuer}/protocol/openid-connect/userinfo");
    let mut roles: Vec<String> = Vec::new();
    if let Some(access_token) = body["access_token"].as_str() {
        match http
            .get(&userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await
        {
            Ok(ui_resp) if ui_resp.status().is_success() => {
                if let Ok(ui_claims) = ui_resp.json::<serde_json::Value>().await {
                    // Verify the UserInfo sub matches the id_token sub (required by OIDC spec).
                    if ui_claims["sub"].as_str() != Some(claims.sub.as_str()) {
                        return Err(PortalError::Security(
                            "UserInfo sub mismatch with id_token".into(),
                        ));
                    }
                    // Extract realm roles from verified UserInfo response.
                    if let Some(realm_roles) = ui_claims["realm_access"]["roles"].as_array() {
                        roles = realm_roles
                            .iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect();
                    }
                }
            }
            Ok(_) => {
                // UserInfo returned non-success: treat as no roles (fail closed).
                tracing::warn!(
                    "UserInfo endpoint returned non-success; portal roles will be empty"
                );
            }
            Err(e) => {
                tracing::warn!("UserInfo endpoint unreachable: {e}; portal roles will be empty");
            }
        }
    }

    let session_id = rand_b64(32);
    state.auth_store.put_session(PortalSession {
        session_id: session_id.clone(),
        subject: claims.sub.clone(),
        display_name: claims
            .name
            .clone()
            .or(claims.preferred_username.clone())
            .unwrap_or_else(|| claims.sub.clone()),
        email: claims.email.clone(),
        roles,
        created_at: Instant::now(),
    });

    let mut resp = Redirect::to(&flow.return_to).into_response();
    let secure = secure_cookies(&state);
    let h = resp.headers_mut();
    h.append(
        header::SET_COOKIE,
        set_cookie(SESSION_COOKIE, &session_id, 28800, secure)
            .parse()
            .unwrap(),
    );
    h.append(
        header::SET_COOKIE,
        clear_cookie(FLOW_COOKIE, secure).parse().unwrap(),
    );
    Ok(resp)
}

fn jsonwebtoken_decode_payload(token: &str) -> Option<serde_json::Value> {
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// GET /auth/session — lightweight session probe for UI.
pub async fn session_info(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match session_from_headers(&state, &headers) {
        Some(s) => Json(serde_json::json!({
            "authenticated": true,
            "subject": s.subject,
            "displayName": s.display_name,
            "email": s.email,
        }))
        .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"authenticated": false})),
        )
            .into_response(),
    }
}

/// GET /auth/me — authenticated identity.
pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match session_from_headers(&state, &headers) {
        Some(s) => Json(serde_json::json!({
            "subject": s.subject,
            "displayName": s.display_name,
            "email": s.email,
        }))
        .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"unauthenticated"})),
        )
            .into_response(),
    }
}

/// POST /auth/logout — invalidate portal session.
///
/// CSRF protection: validates that the request Origin matches the configured
/// public base URL. Cross-origin logout requests are rejected with 403.
/// Only the portal-local session is invalidated; Keycloak SSO sessions remain
/// active unless the client separately calls the Keycloak end_session endpoint.
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    // Origin check: reject cross-origin logout attempts (CSRF protection).
    let allowed_origin = state
        .config
        .public_base_url
        .as_deref()
        .unwrap_or("http://localhost");
    // Trim trailing slash for comparison.
    let allowed_origin = allowed_origin.trim_end_matches('/');
    let request_origin = headers
        .get(header::ORIGIN)
        .or_else(|| headers.get(header::REFERER))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // A missing Origin on a same-site form POST is acceptable only when
    // Referer is also absent (strict browser) — allow it only if no cross-
    // origin evidence is present.
    let origin_ok = request_origin.is_empty() || request_origin.starts_with(allowed_origin);
    if !origin_ok {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": {"code": "CSRF_REJECTED", "message": "Cross-origin logout rejected"}
            })),
        )
            .into_response();
    }

    if let Some(sid) = read_cookie(&headers, SESSION_COOKIE) {
        state.auth_store.drop_session(sid);
    }
    let mut resp = Redirect::to("/").into_response();
    let secure = secure_cookies(&state);
    resp.headers_mut().append(
        header::SET_COOKIE,
        clear_cookie(SESSION_COOKIE, secure).parse().unwrap(),
    );
    resp
}

pub fn session_from_headers(state: &AppState, headers: &HeaderMap) -> Option<PortalSession> {
    let sid = read_cookie(headers, SESSION_COOKIE)?;
    state.auth_store.get_session(sid)
}

/// Returns true when the client asked for an HTML document — i.e. a browser
/// page navigation rather than a fetch/XHR/API call.
fn accepts_html(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false)
}

/// Paths that render HTML pages and therefore participate in the
/// unauthenticated → Keycloak redirect flow. Everything else under the
/// protected layer (/api/v1/*, /specs/*, /static/*) always gets JSON 401.
fn is_html_page_path(path: &str) -> bool {
    path == "/"
        || path.starts_with("/reference/")
        || path == "/reference"
        || path.starts_with("/swagger-ui")
        // BFF session-establishment endpoints are browser navigations too —
        // an unauthenticated browser hitting them should enter portal login
        // (with return_to) rather than receive a bare JSON 401. Exact paths
        // only: arbitrary /agent-bff/* API paths still get JSON 401/403.
        || matches!(
            path,
            "/agent-bff/auth/login"
                | "/agent-bff/auth/callback"
                | "/agent-bff/auth/session"
                | "/agent-bff/auth/me"
        )
}

/// Axum middleware: require a valid portal session.
///
/// Unauthenticated **browser page navigations** (GET/HEAD + Accept: text/html
/// on an HTML page path) are redirected into the existing OIDC login flow
/// with the originally requested relative destination preserved in the
/// validated `return_to` parameter. API, spec, static-asset and proxy
/// requests keep receiving JSON 401 — they are never redirected.
pub async fn require_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if session_from_headers(&state, &headers).is_some() {
        return next.run(request).await;
    }

    let is_navigation = matches!(
        request.method(),
        &axum::http::Method::GET | &axum::http::Method::HEAD
    ) && accepts_html(&headers)
        && is_html_page_path(request.uri().path());

    if is_navigation {
        let dest = request
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        let safe_dest = sanitize_return_to(Some(dest));
        let target = format!("/auth/login?return_to={}", urlencoding(&safe_dest));
        return Redirect::to(&target).into_response();
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "error": {"code":"UNAUTHENTICATED","message":"Portal login required"}
        })),
    )
        .into_response()
}

/// Axum middleware: require a valid portal session AND the configured
/// API-tester realm role. Used on the /dev-api and /agent-bff proxy routes
/// so authenticated portal users cannot execute API calls without the
/// explicit testing permission.
pub async fn require_api_tester(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let session = session_from_headers(&state, &headers);
    match session {
        Some(s) if s.has_role(&state.config.portal_tester_role) => next.run(request).await,
        Some(_) => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": {"code":"FORBIDDEN","message":"API testing permission required"}
            })),
        )
            .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": {"code":"UNAUTHENTICATED","message":"Portal login required"}
            })),
        )
            .into_response(),
    }
}

pub fn make_store() -> SharedStore {
    Arc::new(SessionStore::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_to_accepts_relative_paths() {
        assert_eq!(sanitize_return_to(Some("/")), "/");
        assert_eq!(
            sanitize_return_to(Some("/reference/frontend/agent-bff")),
            "/reference/frontend/agent-bff"
        );
        assert_eq!(
            sanitize_return_to(Some("/reference/backend/agency-service?tab=ops")),
            "/reference/backend/agency-service?tab=ops"
        );
    }

    #[test]
    fn return_to_rejects_open_redirects() {
        for bad in [
            "https://evil.example.com/",
            "http://evil.example.com",
            "//evil.example.com/path",
            r"/\evil",
            "javascript:alert(1)",
            "reference/frontend",
            "",
        ] {
            assert_eq!(
                sanitize_return_to(Some(bad)),
                "/",
                "should sanitize {bad:?}"
            );
        }
        assert_eq!(sanitize_return_to(None), "/");
    }

    #[test]
    fn html_page_paths_cover_only_page_routes() {
        assert!(is_html_page_path("/"));
        assert!(is_html_page_path("/reference/frontend/agent-bff"));
        assert!(is_html_page_path("/reference/backend"));
        assert!(is_html_page_path("/swagger-ui"));
        // BFF session-establishment GET endpoints are browser navigations.
        assert!(is_html_page_path("/agent-bff/auth/login"));
        assert!(is_html_page_path("/agent-bff/auth/callback"));
        assert!(is_html_page_path("/agent-bff/auth/session"));
        assert!(is_html_page_path("/agent-bff/auth/me"));
        for api in [
            "/api/v1/catalog",
            "/specs/development/agent-bff/openapi.json",
            "/static/scripts/portal.js",
            "/dev-api/agent/api/v1/agents",
            "/agent-bff/api/v1/me",
            "/agent-bff/auth",      // not an exact allowlisted path
            "/agent-bff/auth/login/extra",
            "/agent-bff/auth/session/anything",
            "/agent-bff/me",
            "/auth/login",
            "/health/live",
        ] {
            assert!(!is_html_page_path(api), "{api} must stay JSON 401");
        }
    }

    #[test]
    fn accept_header_detection() {
        let mut h = HeaderMap::new();
        assert!(!accepts_html(&h));
        h.insert(header::ACCEPT, "application/json".parse().unwrap());
        assert!(!accepts_html(&h));
        h.insert(
            header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                .parse()
                .unwrap(),
        );
        assert!(accepts_html(&h));
    }
}
