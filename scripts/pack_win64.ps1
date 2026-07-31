<#
.SYNOPSIS
  Pack an already-built Voxelforge client into a self-contained Windows folder
  that can be uploaded as a Steam depot (or zipped and handed to a playtester).

.DESCRIPTION
  This script NEVER builds. It takes the release exe that is already on disk and
  copies it, plus exactly the runtime data the client reads at boot, into a clean
  output folder. It then re-verifies the result against the same required-file
  list, so "pack succeeded" means "every file the game hard-depends on is in the
  folder", not "the copy exited 0".

  ASCII only, on purpose: Windows PowerShell 5.1 reads .ps1 as ANSI unless the
  file has a BOM, so a stray em dash turns into a parser error.

  What goes in the pack, and why (see docs/ship-layout.md section 2 for the audit):
    voxelforge.exe          the client
    assets\audio\*.wav      bevy_audio loads these by name, relative to the EXE dir
    assets\audio\CREDITS.md audio licence attribution - required by the licences
    assets\models\*.vox     import.rs scans assets\models at startup (CWD-relative)
    assets\story\act1.json  quest.rs reads this with std::fs (CWD-relative);
                            MISSING = panic on the first --play frame
    maps\*.json             scene.rs boots maps\edhari.json (CWD-relative); the
                            others are loadable from the editor
    run-voxelforge.cmd      launcher that pins CWD to the exe dir (see section 4)

  Deliberately NOT packed: *.pdb (debug symbols), assets\story\*.py + *.log
  (dev validators), maps\FORMAT.md (dev doc), the server exe (the client is fully
  offline - it links no networking crate), and everything else in the repo.

.PARAMETER Bin
  Path to the client exe to pack. Default: target\release\voxelforge.exe

.PARAMETER Out
  Output folder. Wiped only when -Clean is passed. Default: _ship\voxelforge-win64

.PARAMETER Clean
  Delete the output folder before packing.

.PARAMETER IncludeVCRuntime
  Also copy VCRUNTIME140.dll from System32 next to the exe (app-local
  deployment). Off by default: on Steam the right answer is to tick the
  "Visual C++ Redist 2015-2022 (x64)" install requirement instead. Turn this on
  for a zip you hand to someone directly. See docs\ship-layout.md section 3.

.PARAMETER Zip
  Also produce <Out>.zip beside the folder.

.EXAMPLE
  powershell -File scripts\pack_win64.ps1 -Clean
  powershell -File scripts\pack_win64.ps1 -Clean -IncludeVCRuntime -Zip
#>
[CmdletBinding()]
param(
    [string] $Bin = "target\release\voxelforge.exe",
    [string] $Out = "_ship\voxelforge-win64",
    [switch] $Clean,
    [switch] $IncludeVCRuntime,
    [switch] $Zip
)

$ErrorActionPreference = "Stop"

# Repo root = parent of scripts\, so the script works from any CWD.
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo

function Fail($msg) {
    Write-Host "PACK FAIL  $msg" -ForegroundColor Red
    exit 1
}

# -- 0. Inputs ---------------------------------------------------------------
if (-not (Test-Path $Bin)) {
    Fail "binary not found: $Bin  (this script does not build - run cargo yourself first)"
}
$binItem = Get-Item $Bin
Write-Host "PACK  bin=$($binItem.FullName)"
Write-Host "PACK  bin mtime=$($binItem.LastWriteTime)  size=$([math]::Round($binItem.Length/1MB,1)) MiB"

if ($Clean -and (Test-Path $Out)) {
    Remove-Item $Out -Recurse -Force
    Write-Host "PACK  cleaned $Out"
}
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# -- 1. The pack list --------------------------------------------------------
# Each entry: Src (repo-relative file or glob), Dest (folder inside the pack),
# Required (pack fails if the source matches nothing).
$packList = @(
    @{ Src = $Bin;                        Dest = ".";              Required = $true  },
    @{ Src = "assets\audio\*.wav";        Dest = "assets\audio";   Required = $true  },
    @{ Src = "assets\audio\CREDITS.md";   Dest = "assets\audio";   Required = $true  },
    @{ Src = "assets\models\*.vox";       Dest = "assets\models";  Required = $true  },
    @{ Src = "assets\story\act1.json";    Dest = "assets\story";   Required = $true  },
    @{ Src = "maps\*.json";               Dest = "maps";           Required = $true  }
)

# Files the game hard-depends on at boot. Re-checked AFTER the copy (section 3)
# so a silently-empty glob cannot ship a folder that dies on the first frame.
$requiredInPack = @(
    "voxelforge.exe",
    "assets\story\act1.json",
    "maps\edhari.json",
    "assets\audio\footstep_grass.wav",
    "assets\audio\swing_light.wav",
    "assets\models\sample.vox"
)

foreach ($entry in $packList) {
    $matched = @(Get-ChildItem -Path $entry.Src -File -ErrorAction SilentlyContinue)
    if ($matched.Count -eq 0) {
        if ($entry.Required) { Fail "nothing matched required source: $($entry.Src)" }
        Write-Host "PACK  skip (no match) $($entry.Src)"
        continue
    }
    $destDir = Join-Path $Out $entry.Dest
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    foreach ($f in $matched) {
        Copy-Item $f.FullName -Destination $destDir -Force
    }
    Write-Host ("PACK  + {0,-28} {1} file(s)" -f $entry.Src, $matched.Count)
}

# -- 2. Launcher + runtime ---------------------------------------------------
# The client mixes two path roots: bevy_asset resolves `assets\` against the EXE
# directory, but quest.rs / scene.rs / import.rs use std::fs against the CURRENT
# WORKING DIRECTORY. Steam and Explorer both launch with CWD = the exe folder, so
# they agree - but a shortcut with a different "Start in" silently breaks the
# std::fs half. This launcher removes that whole class of bug.
$launcher = @'
@echo off
rem Voxelforge launcher - pins the working directory to this folder.
rem The client reads maps\ and assets\story\ relative to the CWD, so launching
rem from anywhere else (a shortcut with a different "Start in", a drag-and-drop)
rem would boot into procedural terrain and then panic on the story load.
cd /d "%~dp0"
start "" "%~dp0voxelforge.exe" %*
'@
Set-Content -Path (Join-Path $Out "run-voxelforge.cmd") -Value $launcher -Encoding ASCII

if ($IncludeVCRuntime) {
    # VCRUNTIME140.dll is a HARD import of the exe (see scripts\ship_audit_imports.py).
    # Without it the process dies at load time with 0xC0000135 before main() runs.
    $vc = Join-Path $env:SystemRoot "System32\VCRUNTIME140.dll"
    if (-not (Test-Path $vc)) { Fail "VCRUNTIME140.dll not found at $vc" }
    Copy-Item $vc -Destination $Out -Force
    Write-Host "PACK  + VCRUNTIME140.dll (app-local)"
}

# -- 3. Verify the pack ------------------------------------------------------
$missing = @()
foreach ($rel in $requiredInPack) {
    if (-not (Test-Path (Join-Path $Out $rel))) { $missing += $rel }
}
if ($missing.Count -gt 0) { Fail ("required file(s) missing from pack: " + ($missing -join ", ")) }

# Nothing that leaks the dev box should ride along.
$leaks = @(Get-ChildItem $Out -Recurse -File | Where-Object { $_.Extension -in ".pdb", ".py", ".log", ".rs" })
if ($leaks.Count -gt 0) { Fail ("dev artifacts leaked into pack: " + (($leaks | ForEach-Object { $_.Name }) -join ", ")) }

# -- 4. Manifest -------------------------------------------------------------
$commit = "unknown"
try { $commit = (git rev-parse --short HEAD).Trim() } catch { }
$outFull = (Resolve-Path $Out).Path
$files = Get-ChildItem $Out -Recurse -File | Sort-Object FullName
$totalBytes = ($files | Measure-Object -Property Length -Sum).Sum

$manifest = @()
$manifest += "# Voxelforge win64 depot manifest"
$manifest += "# packed-at   : $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss'))"
$manifest += "# repo-commit : $commit"
$manifest += "# exe-mtime   : $($binItem.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss'))"
$manifest += "# files       : $($files.Count)   total: $([math]::Round($totalBytes/1MB,1)) MiB"
$manifest += "#"
$manifest += "# sha256  size  path"
foreach ($f in $files) {
    if ($f.Name -eq "MANIFEST.txt") { continue }
    $rel = $f.FullName.Substring($outFull.Length).TrimStart("\")
    $h = (Get-FileHash $f.FullName -Algorithm SHA256).Hash.ToLower()
    $manifest += ("{0}  {1,10}  {2}" -f $h, $f.Length, $rel)
}
Set-Content -Path (Join-Path $Out "MANIFEST.txt") -Value $manifest -Encoding UTF8

Write-Host ""
Write-Host "PACK OK   $outFull"
Write-Host ("PACK OK   {0} files, {1} MiB, commit {2}" -f $files.Count, [math]::Round($totalBytes/1MB,1), $commit)

# -- 5. Optional zip ---------------------------------------------------------
if ($Zip) {
    $zipPath = "$Out.zip"
    if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
    Compress-Archive -Path (Join-Path $Out "*") -DestinationPath $zipPath
    Write-Host "PACK OK   $((Resolve-Path $zipPath).Path)  ($([math]::Round((Get-Item $zipPath).Length/1MB,1)) MiB)"
}
