# Portal Ops Runbook

- Deploy: `docker compose --env-file .env up -d docs agent-bff` in /data/iaop-dev/compose
- Portal env: OIDC_ISSUER/OIDC_JWKS_URL/OIDC_CLIENT_ID=iaop-developer-portal/OIDC_CLIENT_SECRET/OIDC_REDIRECT_URI=PORTAL_REDIRECT_URI/UPSTREAM_*
- BFF env: OIDC_REDIRECT_URI_ALLOWLIST must include PORTAL_BFF_CALLBACK
- Health: /health/live, /health/ready; logs: docker logs iaop-dev-api-docs
- Rollback: previous image tag iaop/api-docs:dev / iaop/agent-bff:dev
- Keycloak clients managed via admin API (realm iaop-dev)
