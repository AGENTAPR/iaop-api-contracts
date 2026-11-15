# Contract Metadata

Canonical fields per service in `catalog/services.yaml` (schema:
`catalog/catalog.schema.json`):

- `id`, `name`, `repository`, `environment`, `visibility`
- `maturity`: implemented_verified | implemented_unverified | proposed_contract | deprecated | blocked
- `lifecycle`: active | proposed | deprecated | blocked
- `implementationStatus`, `deploymentStatus`, `verificationStatus`
- `specPath`, `sourceRef`, `sourceCommit`, `apiVersion`
- `domainOwner`, `serverUrl`, `description`, `catalog`, `channel`, `boundedContext`

Rules:
- `implemented_verified` requires a populated spec and passing validation.
- `proposed`/`proposed_contract` entries may ship empty specs (warning only).
- `sourceCommit` records the service repo SHA the spec was derived from.
- The obsolete `iaop-api-gateway` entry was removed; the repository is archived
  and must not reappear.
