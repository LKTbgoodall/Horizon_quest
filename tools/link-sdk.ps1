<#
.SYNOPSIS
  Points mod\.horizon\sdk\api at a Horizon checkout, so cargo check and cargo test build the mod
  against Horizon's mod API without the published SDK.

.DESCRIPTION
  The mod's Cargo.toml takes the API crate from .horizon/sdk/api/crates/horizon-mod-api, where
  `cargo horizon-mod` puts the pinned SDK. Until a Nightly with the mod API publishes one (and
  for checking against Horizon main), this makes that folder a junction to a Horizon checkout:
  ext\horizon by default, cloned (main, shallow) when missing. Nothing is built to run.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools\link-sdk.ps1
  powershell -ExecutionPolicy Bypass -File tools\link-sdk.ps1 -Horizon D:\src\horizon
#>
param([string]$Horizon = "")
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
if (-not $Horizon) {
    $Horizon = Join-Path $Root "ext\horizon"
    if (-not (Test-Path (Join-Path $Horizon "Cargo.toml"))) {
        Write-Host "Cloning Horizon main into $Horizon (shallow)..."
        git clone --depth 1 --filter=blob:none https://github.com/horizon-rust/horizon.git $Horizon
        if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
    }
}
if (-not (Test-Path (Join-Path $Horizon "crates\horizon-mod-api\Cargo.toml"))) {
    throw "$Horizon has no crates\horizon-mod-api: not a Horizon checkout with the mod API."
}
$Sdk = Join-Path $Root "mod\.horizon\sdk"
$Link = Join-Path $Sdk "api"
New-Item -ItemType Directory -Force $Sdk | Out-Null
if (Test-Path $Link) {
    # A junction is removed without touching what it points to.
    (Get-Item $Link).Delete()
}
New-Item -ItemType Junction -Path $Link -Target (Resolve-Path $Horizon) | Out-Null
$Commit = git -C $Horizon rev-parse HEAD
Write-Host "mod\.horizon\sdk\api -> $Horizon ($Commit)"
