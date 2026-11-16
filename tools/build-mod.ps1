<#
.SYNOPSIS
  Builds Horizon Quests into a Horizon instance with the launcher (the mod API format, ADR-018).

.DESCRIPTION
  mod\ is the mod's flat package (horizon-mod.toml with [native], Cargo.toml, src\, assets\,
  data\). The launcher compiles it against the prebuilt SDK of the build horizon-mod.toml pins
  and makes that build the one the instance plays; content and code come from the one package.

  Needs a launcher and a Nightly built with the mod API (newer than nightly-2026-11-13-9f42064).
  It passes --trust-native-code: only the player runs this script, never an agent.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1
  powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1 -Instance "My Instance"
#>
param([string]$Instance = "mon-horizon")
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$Mod = Join-Path $Root "mod"
$Launcher = Join-Path $env:LOCALAPPDATA "Programs\Horizon Launcher\horizon-launcher.exe"
if (-not (Test-Path $Launcher)) { $Launcher = "horizon-launcher" }

& $Launcher native-build $Mod --instance $Instance --trust-native-code
if ($LASTEXITCODE -ne 0) {
    throw "native-build failed: see %APPDATA%\horizon-launcher\native\build.log"
}
Write-Host "Built. Start the instance '$Instance' in the launcher."
