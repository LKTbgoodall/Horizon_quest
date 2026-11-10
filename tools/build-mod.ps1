<#
.SYNOPSIS
  Packages Horizon Quests as a .hnmod, installs it in a Horizon instance and builds the game.

.DESCRIPTION
  1. Packs build/horizon_quests.hnmod from mod/mixins/horizon_quests (.rs, .toml and the
     data/ files that native_data! compiles in).
  2. Unless -NoInstall: copies it into the instance's horizon-native-mods and turns it on,
     so the launcher lists it and keeps it in every build.
  3. Unless -NoBuild: runs horizon-launcher native-build on the instance (all its native mods).

  Step 3 passes --trust-native-code: only the player runs this script, never an agent.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1
  powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1 -NoBuild
  powershell -ExecutionPolicy Bypass -File tools\build-mod.ps1 -Uninstall
#>
param(
    [string]$Instance = "mon-horizon",
    [switch]$NoInstall,
    [switch]$NoBuild,
    [switch]$Uninstall
)
$ErrorActionPreference = "Stop"

$ModId = "horizon_quests"
$Version = "0.0.1"
$Root = Split-Path -Parent $PSScriptRoot
$Bundle = Join-Path $Root "mod\mixins\$ModId"
$Out = Join-Path $Root "build"
$Hnmod = Join-Path $Out "$ModId.hnmod"
$InstanceDir = Join-Path $env:APPDATA "horizon\instances\$Instance"
$NativeMods = Join-Path $InstanceDir "horizon-native-mods"
$ModsList = Join-Path $InstanceDir "horizon-mods.txt"
$Launcher = Join-Path $env:LOCALAPPDATA "Programs\Horizon Launcher\horizon-launcher.exe"

# The commits the mod was built and tested on (the launcher's native/built.json).
$HorizonRev = "fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca"
$MixinsRev = "5a64af938ec86b00d53d16a408aabc0038fb3e57"

$Utf8 = New-Object System.Text.UTF8Encoding($false)

function Set-ModLine([bool]$On) {
    $lines = @()
    if (Test-Path $ModsList) {
        $lines = @(Get-Content $ModsList | Where-Object { $_ -notmatch "^mod\.${ModId}:" })
    }
    if ($On) { $lines += "mod.${ModId}:true" }
    [IO.File]::WriteAllText($ModsList, (($lines -join "`n") + "`n"), $Utf8)
}

if ($Uninstall) {
    Remove-Item (Join-Path $NativeMods "$ModId.hnmod") -ErrorAction SilentlyContinue
    Set-ModLine $false
    Write-Host "Horizon Quests removed from $Instance. Run native-build again to rebuild the game without it."
    exit 0
}

# 1. The .hnmod package: a zip with / separators (Compress-Archive on PowerShell 5.1 writes \).
New-Item -ItemType Directory -Force $Out | Out-Null
Remove-Item $Hnmod -ErrorAction SilentlyContinue
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$manifest = [ordered]@{
    game = "26.3"; hooks = 313; horizon = $HorizonRev; id = $ModId; mixins = $MixinsRev; schema = 1; version = $Version
} | ConvertTo-Json -Compress
$zip = [IO.Compression.ZipFile]::Open($Hnmod, "Create")
try {
    $entry = $zip.CreateEntry("horizon-native.json")
    $w = New-Object IO.StreamWriter($entry.Open(), $Utf8)
    $w.Write($manifest); $w.Close()
    $files = Get-ChildItem $Bundle -Recurse -File | Where-Object {
        ($_.Extension -in ".rs", ".toml" -or $_.FullName -match '\\data\\') -and $_.FullName -notmatch '\\target\\'
    }
    foreach ($f in $files) {
        $rel = $f.FullName.Substring($Bundle.Length + 1).Replace('\', '/')
        [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $f.FullName, "mixins/$ModId/$rel") | Out-Null
    }
    $license = Join-Path $Root "mod\LICENSE"
    if (Test-Path $license) {
        [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $license, "mixins/$ModId/LICENSE") | Out-Null
    }
} finally {
    $zip.Dispose()
}
Write-Host "Package: $Hnmod"
if ($NoInstall) { exit 0 }

# 2. Install it in the instance.
if (-not (Test-Path $NativeMods)) { throw "Instance not found: $NativeMods" }
Copy-Item $Hnmod (Join-Path $NativeMods "$ModId.hnmod") -Force
Set-ModLine $true
Write-Host "Installed in $Instance."
if ($NoBuild) { exit 0 }

# 3. Build the game with every native mod turned on in the instance.
if (-not (Test-Path $Launcher)) { throw "horizon-launcher.exe not found: $Launcher" }
# The launcher is a windowed app: `&` does not wait for it and gives no exit code.
$p = Start-Process -FilePath $Launcher -ArgumentList "native-build", "--instance", $Instance, "--trust-native-code" `
    -NoNewWindow -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "native-build failed (code $($p.ExitCode)). Log: $env:APPDATA\horizon-launcher\native\build.log" }
Write-Host "Ready: start Horizon from the launcher."
