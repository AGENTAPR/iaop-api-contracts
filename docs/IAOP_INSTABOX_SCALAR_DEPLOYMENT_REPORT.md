# IAOP InstaBox Scalar Deployment Runbook

## Service: `docs` (iaop-dev-api-docs)

Defined in: `iaop-infrastructure/instabox/dev/compose/docker-compose.yml`  
Image: `iaop/api-docs:dev`  
Port: `127.0.0.1:8083:8080`  
Public URL: `https://8083-{instabox-id}.instabox.tech` (via `instabox port publish`)

---

## Build and deploy steps

### 1. Rebuild the image

From `iaop-infrastructure/instabox/dev/`:

```bash
docker compose build docs
```

This triggers the multi-stage Dockerfile:
- Downloads Scalar JS bundle from jsdelivr CDN (pinned v1.25.72) at build time.
- Builds Rust binary.
- Packages binary + catalog + specs + scalar.min.js into runtime image.

### 2. Redeploy

```bash
docker compose up -d docs
```

### 3. Verify health

```bash
curl -f http://localhost:8083/health/ready
# Expected: {"status":"ready","services_count":19,"schema_version":2,...}
```

### 4. Smoke test

```bash
# Landing page
curl -s http://localhost:8083/ | grep "IAOP Developer Portal"

# Catalog API
curl -s http://localhost:8083/api/v1/catalog | python -m json.tool | grep schemaVersion

# agent-bff Scalar page
curl -s http://localhost:8083/reference/frontend/agent-bff | grep "scalar.min.js"

# Backward compat
curl -sI http://localhost:8083/swagger-ui/ | grep Location
# Expected: Location: /reference/frontend
```

---

## Environment variables

| Variable | Default | Notes |
|---|---|---|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `8080` | Listen port |
| `CATALOG_PATH` | `/app/catalog/services.yaml` | |
| `CATALOG_SCHEMA_PATH` | `/app/catalog/catalog.schema.json` | |
| `SPECS_DIR` | `/app/specs` | |
| `SCALAR_JS_PATH` | `/app/static/scalar.min.js` | Bundled at build time |
| `ENVIRONMENT` | `development` | Filters catalog by environment field |
| `ALLOWED_CORS_ORIGINS` | `""` (wildcard) | Set to explicit origins in non-dev |
| `RUST_LOG` | `info,iaop_developer_portal=info` | |

---

## Rollback

If the new image fails health check:

```bash
docker compose stop docs
docker compose up -d --no-deps docs   # or pin to previous tag
```

The previous image served Swagger UI at `/swagger-ui/`. The new image redirects `/swagger-ui/` to `/reference/frontend` — this redirect is backward-compatible.

---

## Verification checklist

- [ ] `GET /health/ready` returns `200 {"status":"ready","schema_version":2}`
- [ ] `GET /` serves landing page HTML with "IAOP Developer Portal" heading
- [ ] `GET /reference/frontend/agent-bff` serves Scalar HTML referencing `/static/scalar.min.js`
- [ ] `GET /static/scalar.min.js` returns 200 with `Content-Type: application/javascript`
- [ ] `GET /api/v1/catalog` returns `{schemaVersion: 2, frontend: [...], backend: [...]}`
- [ ] `GET /api/v1/catalog` has exactly one entry with `id: "agent-bff"`
- [ ] `GET /api/v1/catalog` has no entry with `id: "agent-bff-proposed"`
- [ ] `GET /swagger-ui/` redirects to `/reference/frontend`
- [ ] `GET /specs/development/agent-bff/openapi.json` returns 200 valid JSON
- [ ] `GET /specs/development/../../etc/passwd` returns 403 or 404
