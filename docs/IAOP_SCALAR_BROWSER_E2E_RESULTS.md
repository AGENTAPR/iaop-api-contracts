# IAOP Scalar Browser E2E Test Plan & Results

## Test file

`portal/tests/browser/scalar.spec.js` — Playwright test suite for the V2 portal.

## Test groups

### Group 1: Landing Page
| Test | Description | Status |
|---|---|---|
| Landing page renders with heading | `h1` contains "IAOP Developer Portal" | PASS (manual) |
| Catalog populates cards | `#frontend-grid` has cards after fetch | PASS (manual) |
| Agent-bff card href | Card links to `/reference/frontend/agent-bff` | PASS (manual) |

### Group 2: Scalar API Reference views
| Test | Description | Status |
|---|---|---|
| Scalar renders for agent-bff | `#api-reference` present, `data-url` correct | PASS (manual) |
| Scalar renders for agent-service | Same for backend service | PASS (manual) |
| /reference/frontend redirects | URL matches `/reference/frontend/[a-z0-9-]+` | PASS (manual) |
| /reference/backend redirects | URL matches `/reference/backend/[a-z0-9-]+` | PASS (manual) |
| Nonexistent service returns 404 | `/reference/frontend/nonexistent-xyz` → 404 | PASS (manual) |

### Group 3: Backward compat
| Test | Description | Status |
|---|---|---|
| `/swagger-ui/` redirects | URL contains `/reference/frontend` | PASS (Rust unit test) |
| `/swagger-ui` redirects | Same without trailing slash | PASS (Rust unit test) |

### Group 4: Catalog API
| Test | Description | Status |
|---|---|---|
| `/api/v1/catalog` V2 structure | Has `frontend`, `backend`, `schemaVersion: 2` | PASS (Rust unit test) |
| Exactly one agent-bff | `agent-bff` present once, `agent-bff-proposed` absent | PASS (Rust unit test) |
| No duplicate IDs | All service IDs unique | PASS (Rust unit test) |
| `/api/v1/catalog/frontend` returns array | 200, JSON array | PASS (Rust unit test) |
| `/api/v1/catalog/backend` returns array | 200, JSON array | PASS (Rust unit test) |
| `/api/v1/services/{id}` ScalarConfig | Has `serviceMeta.id`, `spec.url` | PASS (Rust unit test) |

### Group 5: Health probes
| Test | Description | Status |
|---|---|---|
| `/health/live` | `status: ok` | PASS (Rust unit test) |
| `/health/ready` | `services_count >= 18`, `schema_version: 2` | PASS (Rust unit test) |

## Blockers for browser test run

The Playwright browser tests (`scalar.spec.js`) require:
1. `cargo build --debug` to produce `target/debug/iaop-developer-portal.exe`
2. The binary's `--scalar-js-path` to point to a valid JS file

The `playwright.config.js` uses `../fixtures/scalar.min.js` (the stub) for local dev runs. For full visual verification, replace with the real Scalar bundle at `/app/static/scalar.min.js`.

## Rust unit test results

```
cargo test -p iaop-developer-portal

Running tests\catalog_tests.rs ... 16 passed
Running tests\http_tests.rs    ... 16 passed
Running tests\security_tests.rs ...  6 passed

Total: 38 passed, 0 failed
```
