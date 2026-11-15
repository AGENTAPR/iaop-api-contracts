# IAOP Central Developer Portal Operations Guide

## 1. Local Development
### Running Locally via Cargo
```powershell
cd iaop-api-contracts/portal
cargo run --bin iaop-developer-portal -- --port 8080 --catalog-path ../catalog/services.yaml --specs-dir ../specs
```
Access the portal at: `http://localhost:8080/swagger-ui/`

### Running via Docker Compose
```powershell
cd iaop-local-dev
docker compose --profile docs up -d
```
Access the portal at: `http://localhost:8082/swagger-ui/`

## 2. Synchronization & Publication Tooling
The `tools/` directory contains automated PowerShell scripts for Windows environments:

### Validate Specifications
Validates syntax, required OpenAPI fields, and ensures 100% unique `operationId`s across all documents:
```powershell
.\tools\validate-contracts.ps1 -Environment development
```

### Verify Service Catalog
Validates `catalog/services.yaml` against `catalog/catalog.schema.json` and ensures all spec files exist on disk:
```powershell
.\tools\verify-catalog.ps1
```

### Synchronize Contracts from Services
Scans workspace repositories, extracts updated OpenAPI specifications, and stages them into `specs/development/`:
```powershell
.\tools\sync-contracts.ps1 -Environment development
```

### Publish Release Manifest
Validates catalog, regenerates SHA-256 artifact checksums, and updates `catalog/manifest.{env}.json`:
```powershell
.\tools\publish-catalog.ps1 -Environment development
```

## 3. Production Deployment & Kubernetes
Manifests are managed in `iaop-infrastructure/k8s/apps/api-docs/`:
- **Deployment:** 2 replicas, rolling updates, non-root security context, read-only root filesystem.
- **Service:** ClusterIP on port 80.
- **Ingress:** Nginx ingress with TLS termination at `api-docs.iaop.internal`.
- **NetworkPolicy:** Ingress restricted to ingress controller; egress restricted to internal DNS.

Deploy using Kustomize:
```bash
kubectl apply -k iaop-infrastructure/k8s/apps/api-docs
```

## 4. Rollback Procedure
If an invalid specification is published:
1. Revert `catalog/services.yaml` to the previous Git commit.
2. Re-run `.\tools\publish-catalog.ps1`.
3. Roll back the Kubernetes deployment:
   ```bash
   kubectl rollout undo deployment/iaop-developer-portal -n iaop
   ```
