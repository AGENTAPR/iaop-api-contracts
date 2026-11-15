# IAOP Centralized Swagger UI & OpenAPI Developer Portal Architecture

## 1. Executive Overview
The IAOP OpenAPI Developer Portal provides a centralized, unified API documentation experience for the entire Insurance Agent Operating Platform (IAOP) microservice ecosystem. It eliminates the operational overhead of deploying separate Swagger UI instances inside individual microservices, while decoupling documentation availability from the live runtime state of backend services.

## 2. Core Architectural Principles
1. **Single Origin, Single Ingress:** All API documentation is served from a single host (`https://api-docs.iaop.internal` or `http://localhost:8080/swagger-ui/`).
2. **Native Multi-Spec Dropdown:** The topbar features Swagger UI's official multi-URL dropdown, dynamically populated from the machine-readable service catalog (`catalog/services.yaml`).
3. **Decoupled Availability:** Specifications are compiled and versioned at build/release time. If a domain microservice is down, offline, or undergoing maintenance, its documentation remains 100% available to frontend developers, architects, and integration partners.
4. **Security by Default:**
   - **Try It Out Disabled:** `supportedSubmitMethods: []` ensures that documentation does not leak credentials, send unauthorized queries, or perform destructive mutations from public browsers.
   - **No Arbitrary Remote Spec Fetching:** `queryConfigEnabled: false` prevents attackers from manipulating Swagger query parameters to probe arbitrary network hosts.
   - **Offline Operation:** `validatorUrl: "none"` disables external network calls to third-party schema validators.
   - **Directory Traversal Protection:** Every spec request (`/specs/*`) is strictly validated against path traversal (`..`, null bytes, symlink breakouts).
5. **Clear Maturity Classification:**
   - `implemented_verified`: Endpoints are fully verified against runtime routes and integration tests.
   - `implemented_unverified`: Endpoints exist in codebase but lack formal parity certification.
   - `proposed_contract`: Planned design contracts (e.g. AGENA v1 proposed contracts).
   - `deprecated`: Endpoints scheduled for decommissioning.
   - `blocked`: Endpoints blocked by downstream dependencies.

## 3. Data Flow Diagram
```
                     +---------------------------+
                     |  Developer / Browser UI   |
                     +-------------+-------------+
                                   |
             1. GET /swagger-ui/   |   2. GET /api/v1/swagger-config
                                   v
                     +---------------------------+
                     |   iaop-developer-portal   |
                     |       (Axum Server)       |
                     +-------------+-------------+
                                   |
            +----------------------+----------------------+
            | 3. Read Catalog                             | 4. Fetch Spec
            v                                             v
+-----------------------+                     +-----------------------+
| catalog/services.yaml |                     | specs/{env}/{svc}/    |
| (Validated Metadata)  |                     | openapi.json          |
+-----------------------+                     +-----------------------+
```

## 4. Dynamic Catalog Registration
Registering a new service does not require changing Rust code. Operators simply:
1. Export or generate the OpenAPI 3.1 specification (`openapi.json`).
2. Place it into `specs/{environment}/{service_id}/openapi.json`.
3. Add a validated entry into `catalog/services.yaml`.
4. Run `.\tools\verify-catalog.ps1` and `.\tools\publish-catalog.ps1`.
5. Deploy the container image.
