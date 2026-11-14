[CmdletBinding()]
param(
    [string]$Environment = "development"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BaseDir = Split-Path -Parent $ScriptDir
$GeneratorScript = Join-Path $ScriptDir "generate_specs_and_catalog.py"

Write-Host "Synchronizing contracts from workspace into $Environment..." -ForegroundColor Cyan

python $GeneratorScript
if ($LASTEXITCODE -ne 0) {
    Write-Error "Contract synchronization failed during generation"
}

# Run validation after sync
& (Join-Path $ScriptDir "validate-contracts.ps1") -Environment $Environment
& (Join-Path $ScriptDir "verify-catalog.ps1")

Write-Host "Contract synchronization complete for $Environment." -ForegroundColor Green
