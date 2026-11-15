# Frontend API Testing (BFF session)

- Same-origin proxy: `https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech/agent-bff/*` → agent-bff:8080 (prefix stripped)
- BFF login through proxy uses allowlisted `redirect_uri` (`OIDC_REDIRECT_URI_ALLOWLIST` on BFF; PR AGENTAPR/iaop-agent-bff#15)
- Session cookie `__Host-iaop_session` issued for portal host; CSRF via `x-csrf-token` header; token available at `/agent-bff/auth/session` (session-bound)
- Verified: full OIDC login via proxy → `/agent-bff/api/v1/me` 200; `/agent-bff/auth/session` returns csrf_token
- Note: BFF cookie Path=/ is sent to all portal paths (HttpOnly; portal cannot read it)
