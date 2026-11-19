<#
.SYNOPSIS
  Builds Horizon Quests into a Horizon instance with the launcher (the mod API format, ADR-018).

.DESCRIPTION
  mod\ is the mod's flat package (horizon-mod.toml with [native], Cargo.toml, src\, assets\,
  data\). The launcher compiles it against the prebuilt SDK of the Nightly horizon-mod.toml pins
  and makes that build the one the instance plays; content and code come from the one package.

  The launcher builds only on the commit of the version it builds from, and refuses a mod that
  pins another one. By default this passes --update, so the instance moves to the newest
  installed Nightly: it must be the one horizon-mod.toml pins (Nightly 2026-11-18, dd67b76).
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

# A newly installed Nightly must be prepared (its game resources made from the player's own
# Minecraft) before the launcher can build on it. Unprepared, native-build says so once, then
# later runs hang without a word. Preparing runs no mod code.
if (-not $NoUpdate) {
    $Versions = Join-Path $env:APPDATA "horizon-launcher\versions"
    $Newest = Get-ChildItem $Versions -Directory -Filter "nightly-*" -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending | Select-Object -First 1
    if ($Newest -and -not (Test-Path (Join-Path $Newest.FullName ".local\horizon-client"))) {
        Write-Host "Preparing $($Newest.Name) from your Minecraft first..."
        $q = Start-Process -FilePath $Launcher -ArgumentList @("prepare", "--version", $Newest.Name) `
            -NoNewWindow -Wait -PassThru
        if ($q.ExitCode -ne 0) { throw "prepare failed (code $($q.ExitCode))." }
    }
}

$Arguments = @("native-build", "`"$Mod`"", "--instance", "`"$Instance`"")
if (-not $NoUpdate) { $Arguments += "--update" }
$Arguments += "--trust-native-code"
# The launcher is a windowed app: `&` does not wait for it and gives no exit code.
$p = Start-Process -FilePath $Launcher -ArgumentList $Arguments -NoNewWindow -Wait -PassThru
if ($p.ExitCode -ne 0) {
    throw "native-build failed (code $($p.ExitCode)). Log: $env:APPDATA\horizon-launcher\native\build.log"
}
Write-Host "Built. Start the instance '$Instance' in the launcher."
