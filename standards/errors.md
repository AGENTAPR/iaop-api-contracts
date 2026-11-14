# Error Contract Standard

RFC 9457 Problem Details (`application/problem+json`):

```json
{
  "type": "https://api.iaop.internal/problems/validation-error",
  "title": "Validation failed",
  "status": 422,
  "detail": "The request contains invalid fields",
  "instance": "/api/v1/agents",
  "code": "VALIDATION_ERROR",
  "correlationId": "018f3c2a-7b1e-7c3a-9f2d-1a2b3c4d5e6f"
}
```

Rules:
- `code` is a stable SCREAMING_SNAKE machine code; `detail` is client-safe.
- Never expose stack traces, SQL, or internal identifiers.
- Standard codes: `VALIDATION_ERROR`, `NOT_FOUND`, `CONFLICT`,
  `UNAUTHORIZED`, `FORBIDDEN`, `RATE_LIMITED`, `INTERNAL_ERROR`,
  `REQUEST_TIMEOUT`, `IDEMPOTENCY_CONFLICT`.
