<#
.SYNOPSIS
  Packages Horizon Quests as one .hmod (code and content), installs it in a Horizon instance and builds the game.

.DESCRIPTION
  1. Packs build/horizon_quests.hmod, the one file of the mod: its native code
     (horizon-native.json and mixins/horizon_quests/: the .rs, .toml and data/ files
     native_data! compiles in) and its content (horizon-mod.toml and assets/ from
     mod/content: the Quest Book's model and texture), as Horizon's "one .hmod for
     content and code" format describes.
  2. Unless -NoInstall: installs it. The launcher of the pinned Horizon still builds
     native code only from horizon-native-mods/<id>.hnmod, so the code half goes there
     and the content half to horizon-mods/<id>.hmod, both cut from the same file; the
     mod is turned on.
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
$Root = Split-Path -Parent $PSScriptRoot
$Bundle = Join-Path $Root "mod\mixins\$ModId"
$Content = Join-Path $Root "mod\content"
# The version is the bundle manifest's, so the two never disagree.
$Version = (Select-String -Path (Join-Path $Bundle "horizon-mixin.toml") -Pattern '^version = "([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$Out = Join-Path $Root "build"
$Hmod = Join-Path $Out "$ModId.hmod"
$Parts = Join-Path $Out "install"
$NativePart = Join-Path $Parts "$ModId.hnmod"
$ContentPart = Join-Path $Parts "$ModId.hmod"
$InstanceDir = Join-Path $env:APPDATA "horizon\instances\$Instance"
$NativeMods = Join-Path $InstanceDir "horizon-native-mods"
$ContentMods = Join-Path $InstanceDir "horizon-mods"
$ModsList = Join-Path $InstanceDir "horizon-mods.txt"
$Launcher = Join-Path $env:LOCALAPPDATA "Programs\Horizon Launcher\horizon-launcher.exe"

# The commits the mod was built and tested on (the launcher's native/built.json).
$HorizonRev = "fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca"
$MixinsRev = "5a64af938ec86b00d53d16a408aabc0038fb3e57"

$Utf8 = New-Object System.Text.UTF8Encoding($false)
# Every entry gets this date, so the same sources make the same file on any PC.
$Stamp = [DateTimeOffset]::new(2026, 1, 1, 0, 0, 0, [TimeSpan]::Zero)

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
    Remove-Item (Join-Path $ContentMods "$ModId.hmod") -ErrorAction SilentlyContinue
    Set-ModLine $false
    Write-Host "Horizon Quests removed from $Instance. Run native-build again to rebuild the game without it."
    exit 0
}

# 1. What goes in: (entry name, source file or $null with text), the native half and the content half.
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
# The hook API the bundle needs is compatibility.json's, so the two never disagree.
$Hooks = [int](Get-Content (Join-Path $Root "mod\compatibility.json") -Raw | ConvertFrom-Json).hooks
$manifest = [ordered]@{
    game = "26.3"; hooks = $Hooks; horizon = $HorizonRev; id = $ModId; mixins = $MixinsRev; schema = 1; version = $Version
} | ConvertTo-Json -Compress

$nativeEntries = New-Object System.Collections.ArrayList
[void]$nativeEntries.Add(@("horizon-native.json", $null, $manifest))
Get-ChildItem $Bundle -Recurse -File | Where-Object {
    ($_.Extension -in ".rs", ".toml" -or $_.FullName -match '\\data\\') -and $_.FullName -notmatch '\\target\\'
} | Sort-Object FullName | ForEach-Object {
    $rel = $_.FullName.Substring($Bundle.Length + 1).Replace('\', '/')
    [void]$nativeEntries.Add(@("mixins/$ModId/$rel", $_.FullName, $null))
}
$license = Join-Path $Root "mod\LICENSE"
if (Test-Path $license) { [void]$nativeEntries.Add(@("mixins/$ModId/LICENSE", $license, $null)) }

$contentManifest = Join-Path $Content "horizon-mod.toml"
$contentVersion = (Select-String -Path $contentManifest -Pattern '^version = "([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
if ($contentVersion -ne $Version) { throw "mod/content/horizon-mod.toml says $contentVersion, the bundle $Version" }
$contentEntries = New-Object System.Collections.ArrayList
[void]$contentEntries.Add(@("horizon-mod.toml", $contentManifest, $null))
foreach ($top in "assets", "data") {
    $dir = Join-Path $Content $top
    if (Test-Path $dir) {
        Get-ChildItem $dir -Recurse -File | Sort-Object FullName | ForEach-Object {
            $rel = $_.FullName.Substring($Content.Length + 1).Replace('\', '/')
            [void]$contentEntries.Add(@($rel, $_.FullName, $null))
        }
    }
}

# A zip with / separators (Compress-Archive on PowerShell 5.1 writes \), files only.
function New-Zip([string]$Path, $Entries) {
    Remove-Item $Path -ErrorAction SilentlyContinue
    $zip = [IO.Compression.ZipFile]::Open($Path, "Create")
    try {
        foreach ($e in $Entries) {
            $entry = $zip.CreateEntry($e[0])
            $entry.LastWriteTime = $Stamp
            $stream = $entry.Open()
            try {
                if ($null -ne $e[1]) {
                    $bytes = [IO.File]::ReadAllBytes($e[1])
                } else {
                    $bytes = $Utf8.GetBytes($e[2])
                }
                $stream.Write($bytes, 0, $bytes.Length)
            } finally {
                $stream.Dispose()
            }
        }
    } finally {
        $zip.Dispose()
    }
}

New-Item -ItemType Directory -Force $Out, $Parts | Out-Null
New-Zip $Hmod (@($contentEntries) + @($nativeEntries))
New-Zip $NativePart $nativeEntries
New-Zip $ContentPart $contentEntries
Write-Host "The mod: $Hmod (version $Version, $($nativeEntries.Count + $contentEntries.Count) files)"
if ($NoInstall) { exit 0 }

# 2. Install it in the instance: the code where the launcher builds it, the content where the game reads it.
if (-not (Test-Path $NativeMods)) { throw "Instance not found: $NativeMods" }
Copy-Item $NativePart (Join-Path $NativeMods "$ModId.hnmod") -Force
New-Item -ItemType Directory -Force $ContentMods | Out-Null
Copy-Item $ContentPart (Join-Path $ContentMods "$ModId.hmod") -Force
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
