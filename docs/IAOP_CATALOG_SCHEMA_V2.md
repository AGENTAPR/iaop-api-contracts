# IAOP Catalog Schema V2 — Field Definitions & Migration Guide

## Schema version

`catalog/catalog.schema.json` now allows `schemaVersion: 1` or `schemaVersion: 2`. All new catalogs must use `2`. The JSON Schema constraint is `"minimum": 1, "maximum": 2`.

---

## V2 required fields (new)

### `catalog`
- Type: `string`
- Enum: `"frontend"` | `"backend"`
- Required in V2.
- Purpose: separates BFF/portal APIs from domain service APIs. The portal UI renders them in separate sections.

### `lifecycle`
- Type: `string`
- Enum: `"active"` | `"proposed"` | `"deprecated"` | `"blocked"`
- Required in V2.
- Purpose: drives sort order and status badges in the portal landing page. Active services sort before proposed.

---

## V2 optional fields (new)

### `channel`
- Type: `string`
- Enum: `"agent"` | `"office"` | `"customer"` | `"shared"` | `"platform"`
- Used for grouping frontend services in the UI. `agent-bff` → `agent`, `operations-bff` → `office`.

### `boundedContext`
- Type: `string` (free text)
- Used for grouping backend services in the UI. Examples: `"Sales & Insurance"`, `"Agent & Distribution"`.

### `implementationStatus`
- Type: `string`
- Enum: `"implemented_verified"` | `"implemented_unverified"` | `"proposed"` | `"not_started"`
- Provides finer-grained implementation readiness than the legacy `maturity` field.

### `deploymentStatus`
- Type: `string`
- Enum: `"deployed"` | `"not_deployed"` | `"unknown"`

### `verificationStatus`
- Type: `string`
- Enum: `"e2e_verified"` | `"unit_verified"` | `"not_verified"` | `"unknown"`

---

## V1 fields kept (backward compatible)

All existing V1 fields (`id`, `name`, `repository`, `environment`, `visibility`, `maturity`, `specPath`, `sourceRef`, `sourceCommit`, `apiVersion`, `enabled`, `domainOwner`, `serverUrl`, `description`) are retained unchanged. `maturity` is not removed; it continues to work alongside the new `implementationStatus`.

---

## Migration from V1 to V2

1. Change `schemaVersion: 1` → `schemaVersion: 2`
2. Add `catalog: frontend` to BFF services; `catalog: backend` to domain services.
3. Add `lifecycle` to every service.
4. Optionally add `channel`, `boundedContext`, `implementationStatus`, `deploymentStatus`, `verificationStatus`.
5. Remove any `agent-bff-proposed` entries.

---

## Bounded context assignments (reference)

| Bounded Context | Services |
|---|---|
| Agent & Distribution | agent-service, agency-service |
| Recruitment & Onboarding | recruitment-service, onboarding-service |
| Learning & Certification | learning-service, certification-service |
| Sales & Insurance | sales-service, portfolio-service, commission-service |
| Compliance & Performance | compliance-service, performance-service |
| Workflow & Documents | workflow-service, document-service |
| Notifications | notification-service |
| Integration & Platform | integration-service, eligibility-service, api-gateway |

---

## Schema validation

The catalog is validated against `catalog/catalog.schema.json` on every server start and in every test run. Invalid catalogs prevent the server from starting.

Run validation manually:

```bash
cargo test -p iaop-developer-portal loads_and_validates_catalog_v2
```
