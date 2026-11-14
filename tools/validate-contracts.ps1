[CmdletBinding()]
param(
    [string]$Environment = "development"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BaseDir = Split-Path -Parent $ScriptDir
$SpecsDir = Join-Path $BaseDir "specs\$Environment"

Write-Host "Validating OpenAPI specifications in $SpecsDir..." -ForegroundColor Cyan

if (-not (Test-Path $SpecsDir)) {
    Write-Error "Specs directory not found: $SpecsDir"
}

$files = Get-ChildItem -Path $SpecsDir -Recurse -Filter "openapi.json"
if ($files.Count -eq 0) {
    Write-Error "No openapi.json files found in $SpecsDir"
}

$pyScript = @"
import sys, json, os, glob

base_dir = r'$SpecsDir'
files = glob.glob(os.path.join(base_dir, '*', 'openapi.json'))
if not files:
    print('No spec files found')
    sys.exit(1)

total_ops = 0
for f in files:
    with open(f, 'r', encoding='utf-8') as fp:
        data = json.load(fp)
    if 'openapi' not in data or 'info' not in data or 'paths' not in data:
        print(f'FAIL: {f} missing required top-level OpenAPI keys')
        sys.exit(1)
    
    op_ids = set()
    for path, methods in data.get('paths', {}).items():
        for m, op in methods.items():
            if isinstance(op, dict):
                total_ops += 1
                op_id = op.get('operationId')
                if op_id:
                    if op_id in op_ids:
                        print(f'FAIL: Duplicate operationId \"{op_id}\" in {f}')
                        sys.exit(1)
                    op_ids.add(op_id)

print(f'SUCCESS: Validated {len(files)} OpenAPI documents containing {total_ops} operations.')
"@

python -c $pyScript
if ($LASTEXITCODE -ne 0) {
    Write-Error "Contract validation failed"
} else {
    Write-Host "All contracts in $Environment environment are valid." -ForegroundColor Green
}
