# Conformance Gates & Publication Readiness

## What exists today
- `tools/validate_contracts.py` — static contract validation (structure,
  refs, examples, catalog vocabularies, verified-maturity spec gate).
- Live-runtime spec sync evidence for agency-service and agent-service
  (15 and 16 paths respectively, fetched from `/api/docs/openapi.json`).
- Catalog `verificationStatus`/`maturity` fields act as the publication
  readiness signal consumed by `publish-catalog.ps1` / `verify-catalog.ps1`.

## Publication readiness rules (enforced by validator)
- A service may only be `implemented_verified` with a non-empty, valid spec.
- Catalog manifest generation reads `services.yaml`; nothing is published
  automatically — publication remains a manual step via the publish scripts.
- Breaking-change detection: not yet automated. Recommended next step:
  diff-based check comparing `specs/<env>/<service>/openapi.json` against the
  previously published revision, flagging removed paths/operations and
  narrowed schemas.

## Backend conformance
Runtime OpenAPI is source-derived via utoipa. Conformance = runtime doc ==
catalog spec modulo server URLs. Drift is currently detected manually during
spec sync; automated diff gating is pending (see completion report risks).
