<#
.SYNOPSIS
  Builds Horizon Quests into a Horizon instance with the launcher (the mod API format, ADR-018).

.DESCRIPTION
  mod\ is the mod's flat package (horizon-mod.toml with [native], Cargo.toml, src\, assets\,
  data\). The launcher compiles it against the prebuilt SDK of the Nightly horizon-mod.toml pins
  and makes that build the one the instance plays; content and code come from the one package.

  The launcher builds only on the commit of the version it builds from, and refuses a mod that
  pins another one. By default this passes --update, so the instance moves to the newest
  installed Nightly: it must be the one horizon-mod.toml pins (Nightly 2026-11-18, 0b14f8a).
  -NoUpdate keeps the version the instance already plays.

  It passes --trust-native-code: only the player runs this script, never an agent.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1
  powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1 -Instance "My Instance" -NoUpdate
#>
param([string]$Instance = "mon-horizon", [switch]$NoUpdate)
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$Mod = Join-Path $Root "mod"
$Launcher = Join-Path $env:LOCALAPPDATA "Programs\Horizon Launcher\horizon-launcher.exe"
if (-not (Test-Path $Launcher)) { throw "horizon-launcher.exe not found: $Launcher" }

$Arguments = @("native-build", "`"$Mod`"", "--instance", "`"$Instance`"")
if (-not $NoUpdate) { $Arguments += "--update" }
$Arguments += "--trust-native-code"
# The launcher is a windowed app: `&` does not wait for it and gives no exit code.
$p = Start-Process -FilePath $Launcher -ArgumentList $Arguments -NoNewWindow -Wait -PassThru
if ($p.ExitCode -ne 0) {
    throw "native-build failed (code $($p.ExitCode)). Log: $env:APPDATA\horizon-launcher\native\build.log"
}
Write-Host "Built. Start the instance '$Instance' in the launcher."
