# Evidence Capture

Module: `iaop-test-kit` -> `crates/iaop-test-kit/src/evidence.rs`

`evidence::capture()` builds an `EvidenceRecord` containing:
- `meta`: service, operationId, source (`live` | `fixture`), source_commit
  (from `GIT_COMMIT`), captured_at (UTC RFC3339), environment
- sanitized `request`, `response_status`, `response_body`

Sanitization recursively redacts keys matching password/secret/token/
authorization/cookie/api_key/private_key/credential (exact, `_`-suffixed,
or suffixed matches). `to_canonical_json()` emits sorted-key JSON for stable
diffs and provenance tracing.

Usage: call `capture()` from integration tests or capture scripts after a
real request; persist the record under `evidence/<service>/<operationId>.json`
and reference it from example proposals and conformance runs.
