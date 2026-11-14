# Pagination Standard

Two supported styles:

## Offset/page (bounded datasets)

Request: `?page=1&size=20` (size clamped to `MAX_PAGE_SIZE = 200`).
Response envelope:

```json
{ "items": [], "page": 1, "size": 20, "total": null }
```

`total` may be omitted/null — do not require exact counts on large sets.
Ordering MUST be stable (explicit `sort` param or documented default).

## Cursor/keyset (high-volume ordered datasets)

Request: `?cursor=<opaque>&size=20`. Response includes `nextCursor`.
Cursor is opaque to clients; servers encode sort key + id.
