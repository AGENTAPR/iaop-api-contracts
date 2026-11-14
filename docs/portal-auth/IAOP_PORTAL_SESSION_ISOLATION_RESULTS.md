# Session Isolation

- portal_session, __Host-iaop_session, and OAuth access tokens are three independent security contexts — no conversion between them
- BFF cookie is host-bound to portal origin through proxy; cannot be used as Bearer token (BFF expects cookie session)
- Backend tokens cannot create portal or BFF sessions (different clients/audiences)
- Cross-user isolation: sessions keyed by opaque random IDs server-side; cookie values never expose subject
- Limitation: in-memory portal store → restart logs out all portal users (DEV acceptable)
