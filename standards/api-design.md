# IAOP API Design Standard

- **Protocol:** REST over HTTPS, JSON payloads.
- **Spec format:** OpenAPI 3.1.
- **Base path:** `/api/v1/{resources}` (versioned in path).
- **Methods:** standard semantics — GET read, POST create, PUT replace,
  PATCH partial update, DELETE remove. Business commands use explicit
  action sub-resources (e.g. `POST /agents/{id}/activate`).
- **Identifiers:** UUIDs for internal IDs; never expose database keys.
- **Timestamps:** RFC 3339 UTC (`2026-11-13T10:00:00Z`).
- **Money:** integer minor units + ISO 4217 currency code.
- **Idempotency:** critical commands accept `Idempotency-Key` header;
  servers must deduplicate replays.
- **Concurrency:** state transitions support `If-Match`/`version` field
  optimistic locking; conflicts return `409 CONFLICT`.
- **AuthN/Z:** Bearer JWT (Keycloak OIDC); see `authentication.md`.
- **Errors:** RFC 9457 Problem Details; see `errors.md`.
- **Pagination:** see `pagination.md`.
- **Versioning:** see `versioning.md` — breaking changes require `/api/v2`.
