# Versioning Policy

- Path versioning: `/api/v1/...`. Current version is `v1`.
- Non-breaking additions (new optional fields/endpoints) stay in-version.
- Breaking changes → new major version path; old version supported for a
  documented deprecation window.
- Each service's OpenAPI spec lives in `services/<name>/openapi.yaml` and is
  the contract source of truth consumed by `iaop-api-gateway`.
