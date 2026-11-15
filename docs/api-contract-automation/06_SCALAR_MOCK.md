# Scalar & Mock Server

## Scalar Developer Portal
The Developer Portal (`iaop-dev-api-docs`, port 8083 on InstaBox DEV) serves
Scalar reference pages backed by `catalog/manifest.development.json` and the
reviewed specs. Scalar's connect/check/disconnect controls use the
portal-api-tester authorization path established earlier; SSO logout is fixed
(id_token_hint end_session).

## Mock server
Scalar supports a mock server mode that serves example-driven responses from
the spec. Feasibility status: assessed — the catalog/spec pipeline already
produces complete OpenAPI 3.1 documents, so Scalar mock can run against any
`implemented_verified` service spec without backend changes. Full mock
coverage depends on named examples being populated per operation (registry
rules in 04_EXAMPLE_REGISTRY.md). Where examples are absent, Scalar returns
schema-shaped empty responses — acceptable for frontend scaffolding but not a
substitute for conformance testing.
