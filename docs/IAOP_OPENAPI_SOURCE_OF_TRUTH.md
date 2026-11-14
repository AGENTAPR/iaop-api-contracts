# IAOP OpenAPI Source of Truth

This document describes where each OpenAPI specification file comes from, the extraction/copy mechanism, and which specs are pinned vs. freshly generated.

---

## Spec locations

All spec files consumed by the portal live under:
```
iaop-api-contracts/specs/development/{service-id}/openapi.json
```

There are no UAT or production spec directories managed by this repository's generator.

---

## Per-service source of truth

### `agent-bff` — `specs/development/agent-bff/openapi.json`

**Source:** `iaop-agent-bff` repository, built artifact at `target/agent_bff_openapi.json`.

**How to refresh:**
```bash
cd ../iaop-agent-bff
cargo run --bin export-openapi          # generates target/agent_bff_openapi.json
cd ../iaop-api-contracts
python tools/generate_specs_and_catalog.py
```

**Fallback:** If the built artifact is not found, the generator retains the existing pinned spec (`specs/development/agent-bff/openapi.json`) with a warning. The pinned spec was last generated from commit `c5a2056d369e60cc8bb87e0667f42d769418d2a2` and covers 52 endpoints.

**What was removed:** The V1 generator contained `build_agent_bff_spec()` — a 300-line Python function that invented endpoint paths. This has been replaced with the extraction approach above. No spec fabrication occurs in V2.

---

### `agent-service` — `specs/development/agent-service/openapi.json`

**Source:** `iaop-agent-service` repository, built artifact at `target/agent_service_openapi.json`.

**How to refresh:**
```bash
cd ../iaop-agent-service
cargo build --release                   # or cargo run --bin export-openapi
cd ../iaop-api-contracts
python tools/generate_specs_and_catalog.py
```

**Fallback:** Pinned spec retained if artifact not found.

---

### All other services

**Source:** `iaop-api-contracts/services/{service-id}/openapi.json` (or `.yaml`).

These are hand-authored proposed contracts checked into this repository. The generator copies them to `specs/development/` unchanged.

If no source file exists in `services/`, the existing pinned spec in `specs/development/` is kept. If neither exists, the portal's `/specs/...` route returns 404 for that service.

---

## `agent-bff-proposed` — REMOVED

The V1 generator produced `specs/development/agent-bff-proposed/openapi.json` by calling `build_agent_bff_spec()` and changing only the title. This was not a real proposed contract — it was an identical copy. The entry has been removed from `services.yaml` and the generator. The directory `specs/development/agent-bff-proposed/` remains on disk but is no longer registered in the catalog and will not appear in the portal.

---

## What the generator does NOT do

- Does not write to `specs/uat/` or `specs/production/`.
- Does not fabricate endpoint definitions.
- Does not modify `catalog/services.yaml` (the catalog is maintained manually or by CI).
- Does not fail silently: missing required specs cause a non-zero exit.
