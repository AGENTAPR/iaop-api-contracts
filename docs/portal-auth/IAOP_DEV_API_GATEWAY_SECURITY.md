# Dev API Gateway Security

- `/dev-api/{agent|agency|eligibility}/*` and `/agent-bff/*` are the ONLY proxy routes; upstreams are fixed env-configured URLs — no arbitrary host/SSRF
- Forwarded headers allowlist: cookie, content-type, accept, authorization, x-csrf-token, x-request-id, accept-language
- Returned headers allowlist: content-type, set-cookie, location, cache-control, x-request-id, www-authenticate
- 8 MiB request body cap, 30s timeout, no upstream redirect following, `..`/NUL path rejection
- Domain services still verify their own JWTs — proxy never injects credentials
- DEV-only: disable UPSTREAM_* envs to turn routes off
