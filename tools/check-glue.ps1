<#
.SYNOPSIS
  Type-checks the bundle against the pinned Horizon source, without building or running the game.

.DESCRIPTION
  1. horizon-mixins apply: stages the launcher's own copy of the pinned Horizon source with this
     mod applied (a source transform, nothing else).
  2. cargo check on the staged horizon-client: the glue (hq_*_glue.rs and the hook helpers) is
     compiled against the real hooks and helpers, so a wrong signature, a private method or a
     warning shows here, before anyone builds the game.

  Nothing is built to run and the mod's code is never executed: this is not the launcher's
  native-build and passes no --trust-native-code. Horizon's own build scripts do run, as in any
  cargo check of Horizon.

  Needs the Horizon launcher to have built once on this PC (its source copy and its
  horizon-mixins tool), Rust, and the network the first time (cargo fetches Horizon's crates).
  The first run takes a few minutes; build/check/ then keeps the checked crates.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools\check-glue.ps1
#>
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$Compat = Get-Content (Join-Path $Root "mod\compatibility.json") -Raw | ConvertFrom-Json
$Native = Join-Path $env:APPDATA "horizon-launcher\native"
$Source = Join-Path $Native "source\$($Compat.horizon_revision)"
if (-not (Test-Path $Source)) {
    throw "The launcher has no copy of Horizon $($Compat.horizon_revision) in $Native\source: build once with the launcher first."
}
$Tool = Get-ChildItem (Join-Path $Native "tools") -Directory |
    Where-Object { $Compat.horizon_mixins_revision.StartsWith($_.Name) } |
    ForEach-Object { Join-Path $_.FullName "bin\horizon-mixins.exe" } |
    Where-Object { Test-Path $_ } |
    Select-Object -First 1
if (-not $Tool) {
    throw "The launcher has no horizon-mixins $($Compat.horizon_mixins_revision) in $Native\tools."
}

$Out = Join-Path $Root "build\check"
$Stage = Join-Path $Out "stage"
New-Item -ItemType Directory -Force $Out | Out-Null
# horizon-mixins only stages into a folder that does not exist yet.
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }

Write-Host "Staging Horizon $($Compat.horizon_revision.Substring(0, 7)) with the mod applied..."
& $Tool apply --workspace $Source --mods (Join-Path $Root "mod\mixins") --out $Stage
if ($LASTEXITCODE -ne 0) { throw "horizon-mixins apply failed: the manifest or a module is wrong (see above)." }

Write-Host "Type-checking the staged game (cargo check, nothing is run)..."
$env:CARGO_TARGET_DIR = Join-Path $Out "target"
Push-Location $Stage
try {
    cargo check -p horizon-client --bin horizon-client --locked --message-format short
    $code = $LASTEXITCODE
} finally {
    Pop-Location
}
if ($code -eq 0) {
    Write-Host "The glue type-checks against Horizon $($Compat.horizon_revision.Substring(0, 7)). Warnings above that name game.rs are the mod's."
} else {
    Write-Host "The glue does NOT compile against Horizon $($Compat.horizon_revision.Substring(0, 7)): see the errors above (game.rs holds the mod's modules)."
}
exit $code
