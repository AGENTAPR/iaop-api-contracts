# Canonical Example Registry

Examples live inside the OpenAPI documents under
`paths.<path>.<method>.responses.<status>.content.<media>.examples` (or
`example`), with `x-evidence` pointing at a captured `EvidenceRecord`.

Rules:
- An example may only be marked verified when backed by an evidence record
  (live DEV call or isolated DB fixture) whose payload validates against the
  schema.
- `tools/validate_contracts.py` type-checks every inline example against its
  schema; unverified/AI-drafted examples must live under `proposals/` until a
  human promotes them.
- GROQ-drafted examples are proposals only (see 05_GROQ_INTEGRATION.md).
