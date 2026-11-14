[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BaseDir = Split-Path -Parent $ScriptDir
$CatalogFile = Join-Path $BaseDir "catalog\services.yaml"
$SchemaFile = Join-Path $BaseDir "catalog\catalog.schema.json"

Write-Host "Verifying service catalog against schema..." -ForegroundColor Cyan

if (-not (Test-Path $CatalogFile)) {
    Write-Error "Catalog file not found: $CatalogFile"
}
if (-not (Test-Path $SchemaFile)) {
    Write-Error "Catalog schema not found: $SchemaFile"
}

$pyScript = @"
import sys, json, yaml, jsonschema, os

base_dir = r'$BaseDir'
cat_file = r'$CatalogFile'
schema_file = r'$SchemaFile'

with open(schema_file, 'r', encoding='utf-8') as sf:
    schema = json.load(sf)

with open(cat_file, 'r', encoding='utf-8') as cf:
    catalog = yaml.safe_load(cf)

try:
    jsonschema.validate(instance=catalog, schema=schema)
except jsonschema.ValidationError as e:
    print(f'SCHEMA ERROR: {e.message}')
    sys.exit(1)

services = catalog.get('services', [])
seen_ids = set()
missing_specs = []

for s in services:
    sid = s.get('id')
    if sid in seen_ids:
        print(f'DUPLICATE SERVICE ID: {sid}')
        sys.exit(1)
    seen_ids.add(sid)
    
    spec_path = s.get('specPath')
    full_path = os.path.join(base_dir, spec_path)
    if not os.path.exists(full_path):
        missing_specs.append((sid, spec_path))

if missing_specs:
    print(f'MISSING SPEC FILES ({len(missing_specs)}):')
    for sid, sp in missing_specs:
        print(f'  - {sid}: {sp}')
    sys.exit(1)

print(f'SUCCESS: Verified catalog with {len(services)} services. All spec files exist on disk.')
"@

python -c $pyScript
if ($LASTEXITCODE -ne 0) {
    Write-Error "Catalog verification failed"
} else {
    Write-Host "Service catalog is 100% verified." -ForegroundColor Green
}
