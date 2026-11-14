[CmdletBinding()]
param(
    [string]$Environment = "development"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BaseDir = Split-Path -Parent $ScriptDir

Write-Host "Publishing catalog and generating checksum manifest for $Environment..." -ForegroundColor Cyan

# 1. Validate
& (Join-Path $ScriptDir "verify-catalog.ps1")
& (Join-Path $ScriptDir "validate-contracts.ps1") -Environment $Environment

# 2. Generate manifest.json with SHA256 checksums
$pyManifest = @"
import hashlib, json, os, glob, datetime

base_dir = r'$BaseDir'
env = '$Environment'
specs_dir = os.path.join(base_dir, 'specs', env)
files = glob.glob(os.path.join(specs_dir, '*', 'openapi.json'))

manifest = {
    'environment': env,
    'publishedAt': datetime.datetime.utcnow().isoformat() + 'Z',
    'totalSpecs': len(files),
    'artifacts': {}
}

for f in sorted(files):
    rel = os.path.relpath(f, base_dir).replace('\\', '/')
    with open(f, 'rb') as fp:
        data = fp.read()
    sha = hashlib.sha256(data).hexdigest()
    manifest['artifacts'][rel] = {
        'bytes': len(data),
        'sha256': sha
    }

out_path = os.path.join(base_dir, 'catalog', f'manifest.{env}.json')
with open(out_path, 'w', encoding='utf-8') as out_f:
    json.dump(manifest, out_f, indent=2)

print(f'PUBLISHED: Manifest written to {out_path} with {len(files)} verified checksums.')
"@

python -c $pyManifest
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to generate release manifest"
}

Write-Host "Catalog publication successful for $Environment." -ForegroundColor Green
