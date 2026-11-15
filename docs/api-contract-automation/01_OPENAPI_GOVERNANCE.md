# OpenAPI Governance

## Source of truth
`catalog/services.yaml` is the single canonical source of service metadata.
`tools/generate_specs_and_catalog.py` derives service ids, spec paths, and
manifest entries from it — there is no second SERVICES_META list to drift.

## Contract artifacts
- `specs/development/<service>/openapi.json` — reviewed consumer contract.
  Synced from live runtime `/api/docs/openapi.json` where the service emits a
  complete document (agency-service: 15 paths, agent-service: 16 paths).
- `specs/schemas/` — vendored shared schemas so relative `$ref` resolves
  locally during validation.
- `catalog/manifest.development.json` — generated manifest consumed by the
  Developer Portal / Scalar.

## Runtime vs catalog
Runtime OpenAPI is generated from utoipa annotations in each service repo.
The catalog spec is the reviewed contract; drift between them is a defect
tracked per service (see completion report). Services whose runtime doc has
empty `paths()` must not be marked verified.

## Validation
`python tools/validate_contracts.py --env development` enforces:
- valid `openapi`/`info`/`paths` structure on every document
- unique `operationId` within each document
- resolvable local and file-relative `$ref`s
- example payloads type-check against their schemas
- controlled vocabularies matching `catalog.schema.json`
- `implemented_verified` maturity requires a non-empty spec
Empty `paths` is permitted only for services explicitly marked `proposed`
(currently: customer-bff) and emits a warning.
