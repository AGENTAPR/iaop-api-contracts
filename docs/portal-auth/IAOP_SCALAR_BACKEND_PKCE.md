# Backend API Testing (OAuth2 PKCE)

- Client: `iaop-scalar-dev` (public, PKCE S256, no secret, no password grant)
- Swagger UI OAuth configured via utoipa-swagger-ui `oauth::Config` (PKCE enabled)
- Backend specs get injected `keycloakOAuth` securityScheme + per-service audience client scopes: `aud-agent`, `aud-agency`, `aud-eligibility` (optional scopes on the client)
- Callback: `https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech/swagger-ui/oauth2-redirect.html` (registered redirect URI)
- Verified: authorize→token exchange→aud contains `iaop-agent-api`; `GET /dev-api/agent/api/v1/agents` → 403 staff-scope (domain authZ enforced)
