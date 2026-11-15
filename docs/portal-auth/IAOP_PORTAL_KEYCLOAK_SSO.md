# Portal Keycloak SSO

- Client: `iaop-developer-portal` (confidential, auth-code, server-side exchange)
- Routes: `/auth/login`, `/auth/callback`, `/auth/session`, `/auth/me`, `POST /auth/logout`
- Session: opaque `portal_session` HttpOnly+Secure+SameSite=Lax cookie; in-memory store (DEV — sessions lost on portal restart; persistent store is future work)
- Protected: `/api/v1/*`, `/specs/*` → 401 anonymous
- Verified: login→callback→`/auth/me` 200 as demo.agent.1 (2026-11-14)
