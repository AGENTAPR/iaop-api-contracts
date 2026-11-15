# IAOP Developer Portal — Security Architecture

## 1. Path traversal protection

Spec files are served from `GET /specs/{*spec_path}`. The `sanitize_path()` function in `portal/src/security.rs` protects against directory traversal:

1. Rejects absolute paths (Windows `C:\...`, Unix `/etc/...`).
2. Rejects `..` path components.
3. Rejects null bytes in path segments.
4. Canonicalises both the base `specs/` directory and the resolved target, then asserts the target is under the base.
5. If the file does not exist, returns 404 (not 403) to avoid leaking filesystem structure.

Covered by tests:
- `sanitize_path_rejects_parent_dir_traversal`
- `sanitize_path_rejects_absolute_paths`
- `http_rejects_path_traversal_attempts`

---

## 2. Security headers (all responses)

Applied by `security_headers_middleware` in `portal/src/security.rs`:

| Header | Value |
|---|---|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |
| `X-XSS-Protection` | `1; mode=block` |
| `Content-Security-Policy` | See below |

### CSP

```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
img-src 'self' data:;
connect-src 'self';
worker-src 'none';
frame-ancestors 'none';
```

Notes:
- `script-src 'self'` — Scalar JS is served from `/static/` (same origin). No CDN, no `unsafe-inline`.
- `style-src 'unsafe-inline'` — Scalar injects styles inline. This is an accepted trade-off for the Scalar bundle.
- `frame-ancestors 'none'` — prevents the portal from being embedded in iframes.
- `worker-src 'none'` — no service workers needed.

---

## 3. CORS

**Configuration:** `ALLOWED_CORS_ORIGINS` env var (comma-separated list of origin URLs).

**Behaviour:**
- If set: only listed origins receive `Access-Control-Allow-Origin`. Methods: `GET, OPTIONS`. Headers: `Content-Type, Authorization`.
- If empty: wildcard (`*`) applies. This is a development fallback only. Production deployments must set explicit origins.

**Test coverage:** `cors_allows_configured_origin`, `cors_wildcard_only_in_dev_mode_no_origins_configured`.

---

## 4. HTML injection prevention

The Scalar HTML template interpolates `service_name` and `spec_url` from catalog data. Both values are passed through `html_escape()` in `routes.rs` before insertion, escaping `&`, `<`, `>`, `"`, `'`.

---

## 5. Spec access control

The portal currently serves specs to anyone who can reach the network (no authentication on spec endpoints). Access control is enforced at the network layer (InstaBox port publishing, VPN, firewall).

For restricted-visibility services (`commission-service`, `compliance-service`, `operations-bff`), the specs are served but only linked from the portal when `ALLOWED_VISIBILITY` includes `"restricted"`. The spec files themselves are not gated by an auth layer in the portal binary — this is consistent with the internal-network deployment model.

**Future work:** Add optional JWT/API-key guard for `visibility: restricted` spec endpoints.

---

## 6. Scalar JS integrity

The Scalar bundle is downloaded at Docker **build time** from a pinned version URL. The image ships with the bundle at `/app/static/scalar.min.js`. No network call is made at runtime to serve or load Scalar.

**Recommended improvement:** Add `--fail-with-body` and an integrity check (sha256sum) to the `curl` command in the Dockerfile after download, to verify the bundle hasn't been tampered with on the CDN.
