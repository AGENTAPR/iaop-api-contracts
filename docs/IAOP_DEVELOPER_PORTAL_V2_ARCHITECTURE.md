# IAOP Developer Portal V2 — Architecture Changes

## Summary

Portal V2 replaces the Swagger UI implementation with Scalar API Reference and restructures the catalog from a flat 20-service list into a typed, dual-catalog model. This document describes every material change.

---

## 1. UI: Swagger UI → Scalar API Reference

**V1:** Used `utoipa-swagger-ui` crate (Rust), which bundled Swagger UI assets into the binary and served them at `/swagger-ui/`.

**V2:** Self-hosts [Scalar API Reference](https://scalar.com) JS, downloaded once at Docker build time from jsdelivr CDN (pinned to v1.25.72), stored at `/app/static/scalar.min.js`. Served from same origin — no CDN dependency at runtime.

### Route changes

| V1 Route | V2 Route | Notes |
|---|---|---|
| `/swagger-ui/` | `/reference/frontend/{id}` | Main reference view |
| `/swagger-ui/` | `/reference/backend/{id}` | Backend services |
| _(none)_ | `/` | New landing page |
| `/api/v1/swagger-config` | _(removed)_ | Swagger-specific, no replacement |
| `/api/v1/catalog` | `/api/v1/catalog` | Now returns `DualCatalogResponse` |
| _(none)_ | `/api/v1/catalog/frontend` | Frontend groups only |
| _(none)_ | `/api/v1/catalog/backend` | Backend groups only |
| _(none)_ | `/api/v1/services/{id}` | Per-service ScalarConfig |
| _(none)_ | `/api/v1/services/{id}/spec` | Spec file redirect |
| `/swagger-ui` | redirects to `/reference/frontend` | Backward compat |

---

## 2. Catalog: Single flat list → Dual typed catalog

**V1:** 20 services (including duplicate `agent-bff-proposed`) in a single flat YAML list. Swagger dropdown showed all 20 entries.

**V2:**
- `agent-bff-proposed` permanently removed (was a copy of `agent-bff` with a changed title).
- 19 services remain, tagged with `catalog: frontend | backend`.
- Frontend services: `agent-bff`, `operations-bff`.
- Backend services: 17 domain services grouped by `boundedContext`.
- Bounded context groups: Agent & Distribution, Recruitment & Onboarding, Learning & Certification, Sales & Insurance, Compliance & Performance, Workflow & Documents, Notifications, Integration & Platform.

---

## 3. Schema: V1 → V2

**V1:** Fields: id, name, repository, environment, visibility, maturity, specPath, sourceRef, sourceCommit, apiVersion, enabled, domainOwner, serverUrl, description.

**V2 adds (all backward-compatible):**
- `catalog` (required): `"frontend"` | `"backend"`
- `lifecycle` (required): `"active"` | `"proposed"` | `"deprecated"` | `"blocked"`
- `channel` (optional): `"agent"` | `"office"` | `"customer"` | `"shared"` | `"platform"`
- `boundedContext` (optional): free-text domain group name
- `implementationStatus` (optional): replaces maturity semantics going forward
- `deploymentStatus` (optional): `"deployed"` | `"not_deployed"` | `"unknown"`
- `verificationStatus` (optional): `"e2e_verified"` | `"unit_verified"` | `"not_verified"` | `"unknown"`

---

## 4. CORS: Wildcard → explicit allowlist

**V1:** `CorsLayer::new().allow_origin(Any)` — always wildcard.

**V2:** When `ALLOWED_CORS_ORIGINS` env var is set (comma-separated), only those origins are allowed. When empty, wildcard applies as a development fallback only.

---

## 5. Spec generator: fabrication → extraction

**V1:** `generate_specs_and_catalog.py` contained a 300-line `build_agent_bff_spec()` function that invented endpoint definitions. It also called `build_proposed_bff_spec()` to clone the spec with a changed title, and copied DEV specs to `specs/uat/` and `specs/production/`.

**V2:** Generator extracts the spec from built artifacts (`target/agent_bff_openapi.json`), falls back to the pinned spec in `specs/development/agent-bff/` if no fresh source is found. No UAT/production copying. No spec fabrication. No `agent-bff-proposed`.

---

## 6. CSP update

V1 CSP: `default-src 'self'; script-src 'self' 'unsafe-inline'; ...`  
V2 CSP: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; worker-src 'none'; frame-ancestors 'none';`

Changes: `script-src` removes `'unsafe-inline'` (not needed for Scalar); adds `worker-src 'none'`; adds `frame-ancestors 'none'`.

---

## Files changed

| File | Change |
|---|---|
| `catalog/catalog.schema.json` | V2: new fields, `maximum: 1` removed from schemaVersion |
| `catalog/services.yaml` | V2: removed agent-bff-proposed, added catalog/lifecycle/etc |
| `portal/Cargo.toml` | Removed utoipa-swagger-ui; added mime_guess |
| `portal/src/catalog.rs` | New DTOs: CatalogGroup, DualCatalogResponse, ScalarConfig, ServiceMeta; new fns: build_dual_catalog, build_scalar_config; removed build_swagger_urls, build_swagger_config_dto |
| `portal/src/config.rs` | Added allowed_cors_origins, scalar_js_path |
| `portal/src/security.rs` | Updated CSP |
| `portal/src/routes.rs` | Full rewrite: Scalar routes, landing page, dual catalog API, explicit CORS |
| `portal/src/main.rs` | Updated startup logging |
| `portal/Dockerfile` | Added Scalar bundle download at build time |
| `portal/tests/catalog_tests.rs` | V2 tests: dual catalog, schema v2, no duplicates |
| `portal/tests/http_tests.rs` | V2 tests: Scalar HTML, dual catalog API |
| `portal/tests/security_tests.rs` | Added CORS tests |
| `portal/tests/browser/scalar.spec.js` | New Playwright tests for Scalar portal |
| `portal/tests/browser/playwright.config.js` | Updated testMatch, webServer command |
| `portal/tests/fixtures/scalar.min.js` | Stub JS for Rust unit tests |
| `tools/generate_specs_and_catalog.py` | Removed fabrication, proposed, UAT copy |
| `docs/` | 8 new documentation files |
