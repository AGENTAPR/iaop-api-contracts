# IAOP API Contract Governance & Compatibility Standard

## 1. Scope & Ownership
- **Owner Repository:** `iaop-api-contracts`
- **Architecture Governance:** `iaop-architecture`
- **Service Implementation:** Domain microservices (`iaop-agent-bff`, `iaop-agent-service`, etc.)

## 2. API Contract Rules
1. **OpenAPI 3.1 Standard:** All published specifications must adhere strictly to OpenAPI 3.1.0+.
2. **Operation Identifiers:**
   - Every operation MUST have a unique `operationId` formatted in `camelCase` (e.g. `getMe`, `listProposals`, `calculateIllustration`).
   - Reusing permission strings or arbitrary tags as `operationId` is strictly forbidden.
3. **Response Envelopes:**
   - Frontend-facing BFF APIs MUST wrap responses in `{ "data": ..., "meta": ... }` or `{ "error": ..., "meta": ... }`.
   - Microservices MUST document RFC-7807 `ProblemDetails` for error codes.
4. **Monetary Values:**
   - Monetary amounts (premiums, commissions, balances) MUST be represented as string-encoded decimals (e.g., `"15000000.00"` IDR), never floating-point numbers.
5. **PII Masking:**
   - Indonesian National ID (NIK / KTP) must be masked (`3171************`).
   - Customer phone numbers and bank account numbers must be redacted in non-privileged operations.

## 3. Backward Compatibility & Breaking Changes
Any contract modification is classified into one of three tiers:
- **BACKWARD_COMPATIBLE:** Adding optional request fields, adding new endpoints, adding response fields. Permitted without major version bump.
- **POTENTIALLY_BREAKING:** Deprecating endpoints with sunset headers, changing error detail structures.
- **BREAKING:** Removing endpoints, changing HTTP methods, adding mandatory request fields, changing property types, removing enum options.
  - Breaking changes require formal Architectural Review Board (ARB) approval and a semantic version increment.

## 4. Route-to-Spec Parity Verification
Before marking a specification `implemented_verified`, the engineering team must run route parity checks comparing Axum router registrations (`Router::route(...)`) with OpenAPI `paths` definitions.
