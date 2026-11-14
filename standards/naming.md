# Naming Standard

- Resources: plural, kebab-case (`/api/v1/agents`, `/api/v1/policies`).
- Fields: camelCase in JSON (`agentId`, `createdAt`).
- Query params: camelCase (`pageSize`, `sortBy`).
- Headers: standard HTTP + `Idempotency-Key`, `X-Correlation-Id`,
  `X-Request-Id`.
- OperationIds: `<verb><Resource>` camelCase, unique per spec.
