# Authentication Standard

- Bearer JWT issued by Keycloak (OIDC). Validated per-service:
  signature via JWKS, `iss`, `aud`, `exp`.
- Public paths are limited to `/health/*` and `/api/docs/*`.
- Service-to-service calls use client-credentials tokens.
- Authorization policies are defined per service; RBAC business rules
  are out of scope for the platform baseline.
- Tokens must never appear in logs or error responses.
