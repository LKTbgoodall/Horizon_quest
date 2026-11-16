<#
.SYNOPSIS
  Checks the mod as the launcher will compile it: cargo test and cargo check with every warning an
  error, against Horizon's mod API.

.DESCRIPTION
  Needs mod\.horizon\sdk\ (the pinned SDK from `cargo horizon-mod`, or tools\link-sdk.ps1, which
  this runs when the folder is missing). The mod's code is compiled and its tests run; the game is
  neither built nor started, and no --trust-native-code is passed. The first run takes a few
  minutes (the API crates); later runs reuse mod\target\.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools\check-mod.ps1
#>
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$Mod = Join-Path $Root "mod"
if (-not (Test-Path (Join-Path $Mod ".horizon\sdk\api\crates\horizon-mod-api\Cargo.toml"))) {
    & (Join-Path $PSScriptRoot "link-sdk.ps1")
}
$env:RUSTFLAGS = "-D warnings"
cargo test --manifest-path (Join-Path $Mod "Cargo.toml")
if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
cargo check --manifest-path (Join-Path $Mod "Cargo.toml") --lib
if ($LASTEXITCODE -ne 0) { throw "cargo check failed" }
Write-Host "The mod builds against the mod API with no warning, and its tests pass."
