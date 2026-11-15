# Standard Error Codes

| Code | HTTP | Meaning |
|---|---|---|
| VALIDATION_ERROR | 422 | Request failed validation |
| NOT_FOUND | 404 | Resource does not exist |
| CONFLICT | 409 | State conflict / duplicate |
| UNAUTHORIZED | 401 | Missing/invalid credentials |
| FORBIDDEN | 403 | Authenticated but not permitted |
| RATE_LIMITED | 429 | Quota exceeded |
| REQUEST_TIMEOUT | 504 | Processing exceeded deadline |
| IDEMPOTENCY_CONFLICT | 409 | Same Idempotency-Key, different payload |
| INTERNAL_ERROR | 500 | Unexpected server error |
