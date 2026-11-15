# AGENA — Developer Portal & Agent BFF Identity Integration Report

Date: 2026-02-14 (DEV)
Environment: InstaBox DEV (`iaop-keycloak-dev` sandbox)
Portal: `https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech`
Keycloak realm: `iaop-dev` (`https://8080-agq5lpbszgbva3ygddcbarv6ea.instabox.tech/realms/iaop-dev`)

## A. Root cause analysis

1. **Portal SSO ≠ BFF session.** Portal login (`/auth/login` → `portal_session`)
   only proves *developer* identity. The Agent BFF requires its own
   `__Host-iaop_session`, created exclusively by the BFF's Authorization Code +
   PKCE flow. Nothing in the portal login path ever created one — by design.
2. **BFF bootstrap blocked by over-broad authorization (fixed previously,
   commit `e7e8bd5`).** `proxy_router()` applied `portal-api-tester` to all of
   `/agent-bff/*`, so `GET /agent-bff/auth/login` returned 403. Fixed by
   splitting `bff_session_router()` (exact allowlist:
   `GET /agent-bff/auth/{login,callback,session,me}`, `POST /auth/logout`)
   under `require_session`, while wildcard API execution stays under
   `require_api_tester`.
3. **Empty portal roles.** Keycloak's realm-roles mapper lacked
   `userinfo.token.claim`, so UserInfo never returned `realm_access.roles`;
   every portal session had `roles: []` and tester checks failed closed.
   Mapper corrected in the `iaop-dev` realm (sessions must re-login to pick
   up roles).
4. **OpenAPI drift.** The published spec declared cookie `agena_session_id`;
   the actual cookie is `__Host-iaop_session`. Reconciled (this change).
5. **No connection visibility.** Scalar pages gave no indication of which
   identity contexts existed. Fixed with the connect panel (this change).

## B. Architecture — final identity boundaries

| Context | Credential | Created by | Authorizes |
|---|---|---|---|
| Developer Portal | `portal_session` (HttpOnly) | Portal OIDC (`iaop-developer-portal` client) | Catalog, docs, Scalar; `portal-api-tester` role required for API execution |
| Agent BFF | `__Host-iaop_session` (HttpOnly, Secure, SameSite=Lax, Path=/) | BFF OIDC (`iaop-agent-bff` client, confidential, PKCE S256) | Agent-facing BFF APIs; session holds subject, agent_id, workspace, server-side access/refresh tokens, CSRF token |
| Domain services | `Authorization: Bearer <agent-access-token>` | BFF forwards the *agent's* Keycloak access token | Independent issuer/signature/audience/expiry + resource-level authz |

Core invariant preserved: **Developer Identity ≠ Agent Business Identity.**
A BFF login resolves the agent link authoritatively via Agent Service
(`resolve_identity(issuer, sub)`); only `ACTIVE` + non-revoked links grant
`agent_id`. Unlinked users get a restricted "candidate" session and fail
closed on agent endpoints. Developer roles never substitute for agent links.

## C. Implementation — modified files (this phase)

- `portal/src/routes.rs` — connect-panel mount (`#bff-connect`) + `bff-connect.js`
  script tag rendered only for `catalog_kind == "frontend"` references;
  asset route for the new script.
- `portal/src/auth.rs` — `/auth/me` now returns `apiTester` (derived from
  verified session roles vs `portal_tester_role`).
- `portal/static/scripts/bff-connect.js` (new) — renders developer identity,
  API-testing permission, BFF session state; actions: Check connection,
  Connect agent identity (→ `/agent-bff/auth/login?redirect_uri=<origin>/agent-bff/auth/callback`),
  Disconnect agent (POST `/agent-bff/auth/logout` with `x-csrf-token`).
- `portal/static/styles/portal.css` — `.connect-panel`/`.cp-*` styles (ADL tokens).
- `portal/tests/browser/bff-e2e.js` — added connect-panel, CSRF logout,
  post-logout denial checks (12 checks total).
- `specs/development/agent-bff/openapi.json` — `sessionCookie.name` →
  `__Host-iaop_session`; CSRF description now points to
  `GET /agent-bff/auth/session` for the token.

Prior phase (already deployed): `portal/src/proxy.rs` (`bff_session_router`,
cookie filtering strips `portal_*` before upstream), `portal/src/auth.rs`
(HTML-navigation redirect + `sanitize_return_to`), `portal/src/routes.rs`
(policy split).

## D. OpenAPI changes

- `components.securitySchemes.sessionCookie`: `apiKey`/`cookie`,
  name `__Host-iaop_session` (was `agena_session_id` — stale).
- `csrfToken` header scheme retained; description documents that the value
  comes from the authorized `GET /agent-bff/auth/session` response.
- 47 BFF paths unchanged; servers rewritten by the portal to
  `/agent-bff/api/v1`. Auth bootstrap paths (`/auth/*`) are intentionally not
  API operations — they are session-establishment endpoints proxied under
  `PortalAuthenticated`.

## E. Keycloak configuration (no secrets)

- Realm `iaop-dev`; clients `iaop-developer-portal` (portal, public+PKCE) and
  the BFF confidential client (Authorization Code + PKCE S256, redirect URI
  allowlist incl. the portal-proxied callback).
- Role `portal-api-tester` exists; assigned explicitly to approved dev/test
  users only (verified: `demo.agent.1` also holds `IAOP_AGENT`).
- `realm roles` mapper: `access.token.claim=true` **and**
  `userinfo.token.claim=true` (fixed this engagement) so portal UserInfo
  yields `realm_access.roles`.
- Redirect URIs: portal callback `…:8083…/auth/callback`; BFF callback
  `…:8083…/agent-bff/auth/callback` (proxied) — no wildcard/arbitrary URIs.

## F. Tests

- Rust: `cargo test` in `portal/` — **60 passed, 0 failed** (auth redirect,
  return_to sanitization, proxy allowlists, cookie filtering, catalog,
  security headers).
- Playwright E2E (`portal/tests/browser/bff-e2e.js`, real deployed DEV):
  **12/12 PASS** —
  portal login; BFF bootstrap 303→Keycloak (not 403); full OIDC chain;
  `/agent-bff/auth/session` authenticated; `/agent-bff/api/v1/me` **200**
  (returns real `agentId`); `__Host-iaop_session` HttpOnly; no tokens in
  web storage; connect panel shows both identities + "connected";
  CSRF logout 200 → subsequent `/api/v1/me` **401**, session reports
  unauthenticated; fresh-context portal-only session → BFF API **401**.
- Negative coverage (Rust + prior live runs): anonymous → 401/redirect;
  non-tester → 403 on API execution; unknown `/agent-bff/*` paths → denied;
  invalid/expired OIDC state, nonce mismatch, browser-binding mismatch →
  denied inside BFF (`take_login_txn` single-use + binding cookie check,
  `auth_routes.rs`); revoked/absent agent link → restricted session,
  `agent_id=None`.
- PostgreSQL/Keycloak-backed: agent identity resolution exercised live via
  Agent Service during `/auth/callback` (link status logged `LINKED_ACTIVE`).

## G. Deployment

- Source: `iaop-api-contracts` branch `feature/portal-scalar-v2-sso`,
  HEAD `c097815` (build artifact `de4fd89`).
- Image: `iaop/api-docs:dev` = `sha256:cc56d87cb4aaf63e384b717cf46a42a6adba1a100950c6df78d0bb4615feed02`.
- Container `iaop-dev-api-docs` recreated via `docker compose up -d docs`
  only; Keycloak, Postgres, Kafka, agent/agency/eligibility services,
  volumes and sessions untouched.
- InstaBox checkout note: `/data/iaop-dev/src/iaop-api-contracts` is a dirty
  `development` checkout; the portal was built from a clean worktree
  `/data/iaop-dev/src/iaop-contracts-portal` @ `de4fd89` to avoid the
  unrelated local modifications.
- Sandbox root filesystem is ~100% full — flagged; resize/cleanup advised.

## H. Limitations

- **Fully functional:** portal SSO, tester-gated proxy, BFF OIDC + session,
  agent-link resolution, CSRF logout, connect panel, cookie isolation.
- **Functional with isolated browser context:** authenticating as a
  *different* agent than the developer's Keycloak SSO user requires a
  separate browser profile/context (or Keycloak logout first). `prompt=login`
  is not used as an isolation mechanism. The panel documents this.
- **Not supported (by design):** same-page persona switching, token paste,
  impersonation, developer→agent privilege carry-over.
- **Planned:** Test Personas (multiple synthetic agents) — requires secure
  persona auth/delegation, session isolation, audit of both identities,
  DEV-only enforcement; ADR recommended before implementation.
- Existing portal sessions issued before the UserInfo mapper fix hold empty
  roles until re-login.

## I. Final verdict

**CONDITIONALLY DEV READY**

All Definition-of-DoD items verified live except full same-browser
dual-identity UX, which is an explicit, documented architectural
limitation (isolated browser context is the supported mechanism). A
developer can log into the portal, connect an agent identity through real
Keycloak OIDC, and execute authorized BFF APIs (`GET /api/v1/me` → 200)
while unauthorized paths correctly return 401/403.
