# E2E Results (2026-11-14, demo.agent.1)

| Test | Result |
|---|---|
| Portal anon /api/v1/catalog | 401 PASS |
| Portal anon /specs/... | 401 PASS |
| Portal login→Keycloak→callback→/auth/me | 200 PASS |
| Catalog+spec authed | 200, keycloakOAuth injected PASS |
| BFF login via /agent-bff proxy | 303→Keycloak→callback→session cookie PASS |
| /agent-bff/api/v1/me via proxy | 200 PASS |
| /agent-bff/auth/session | csrf_token issued PASS |
| PKCE token exchange (iaop-scalar-dev) | aud=iaop-agent-api PASS |
| /dev-api/agent call w/ token | 403 staff-scope (domain authZ) PASS |
| /dev-api/agent anon | 401 PASS |
