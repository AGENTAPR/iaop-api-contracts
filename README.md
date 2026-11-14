# iaop-api-contracts

Versioned OpenAPI contracts for IAOP microservices.

## Purpose

iaop-api-contracts is part of the **Insurance Agent Operating Platform (IAOP)**, an enterprise
insurance agent lifecycle platform. It owns the **API Contracts** bounded context
(or platform capability) within the IAOP microservices landscape.

## Bounded Context / Ownership

- **Bounded context:** API Contracts
- **Owner:** AGENTAPR
- **Category:** contracts

## Technical Stack

OpenAPI 3.x

## Architecture Principles

- Domain-Driven Design (DDD) with explicit bounded contexts
- Hexagonal architecture (ports and adapters)
- Event-driven integration via versioned contracts
- PostgreSQL as the system of record where applicable
- Identity and access via Keycloak
- Workflow orchestration via Flowable BPM where applicable

## Responsibilities

- Versioned OpenAPI specifications per microservice
- Request/response schema definitions
- API compatibility and versioning policy

## Non-Responsibilities

- Service implementation
- Event schema definitions
- Infrastructure configuration

## Related Repositories

- [iaop-architecture](https://github.com/AGENTAPR/iaop-architecture) - DDD strategic design, bounded context maps, ADRs
- [iaop-api-contracts](https://github.com/AGENTAPR/iaop-api-contracts) - Versioned OpenAPI contracts
- [iaop-event-contracts](https://github.com/AGENTAPR/iaop-event-contracts) - Versioned integration events
- [iaop-workflow-service](https://github.com/AGENTAPR/iaop-workflow-service) - Flowable BPM orchestration
- [iaop-local-dev](https://github.com/AGENTAPR/iaop-local-dev) - Local development environment

## Status

**INITIALIZED - NOT IMPLEMENTED**

This repository has been bootstrapped with baseline documentation and configuration only.
No business logic or service implementation exists yet.

## Development Conventions

- Default branch: `main`
- Development branch: `development` — all changes are committed and pushed here
- Squash merges preferred; merge commits and rebases disabled
- Pull requests required before merging into `main`
- Topics: iaop, insurance, insurance-agents, enterprise-architecture, openapi, api-contracts, documentation

## Contract Layout

- `standards/` - API design, naming, pagination, errors, auth, versioning
- `schemas/` - shared JSON Schemas (problem details, pagination)
- `services/<domain>/openapi.yaml` - per-service OpenAPI 3.1 contracts (DRAFT)
- `tools/validate.py` - CI validation
