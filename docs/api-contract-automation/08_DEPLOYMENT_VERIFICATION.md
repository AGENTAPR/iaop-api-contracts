# Deployment & Verification (InstaBox DEV)

Verified during this program:
- `iaop-dev-agency-service`, `iaop-dev-agent-bff`, `iaop-dev-agent-service`,
  `iaop-dev-eligibility-service`, `iaop-dev-api-docs`, `iaop-dev-db`,
  `iaop-dev-kafka` running on the `iaop-dev` Docker network.
- Domain services publish no host ports; BFF on 127.0.0.1:8082, portal/docs
  on 127.0.0.1:8083, Postgres 127.0.0.1:5432, Kafka 127.0.0.1:29092.
- Caddy edge with restricted Keycloak OIDC allowlist (previous program).
- Agency Service live OpenAPI: `/api/docs/openapi.json` -> 15 paths.
- Agent Service live OpenAPI: `/api/docs/openapi.json` -> 16 paths.
- Eligibility Service OpenAPI paths registered via PR #13 (merged); runtime
  redeploy pending to reflect them.
- Agent BFF runtime OpenAPI still emits empty `paths()` — source-annotation
  gap tracked as AGENA-CONTRACT-003 equivalent; catalog retains the pinned
  reviewed spec.
