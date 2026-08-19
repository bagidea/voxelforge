# Shoot the two before/after pairs for the block-art pass, from the REAL
# gameplay binary. One exe, one seed, one camera per pair — the only thing that
# changes between the two runs of a pair is a single environment variable.
#
#   pair 1  world atlas   VOXELFORGE_ATLAS_MODE=off  vs  (unset)
#           procedural tiles vs the file-backed art set, on procedural terrain
#
#   pair 2  glass         VOXELFORGE_GLASS_OPAQUE=1  vs  (unset)
#           the pane before it had an alpha channel vs after, on maps/glass_demo.json
#
# Run from the repo root:  powershell -File scripts\_poppy_blockart_shoot.ps1

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$exe = Join-Path $root "client\target-poppy\debug\voxelforge.exe"
if (-not (Test-Path $exe)) { throw "no exe at $exe - build it first" }

$out = Join-Path $root "docs\assets\blockart"
New-Item -ItemType Directory -Force $out | Out-Null

function Shoot {
    param([string]$Name, [hashtable]$Env)

    # A fresh env per run: a leftover flag from the previous shot is exactly how
    # a "before" quietly becomes a second "after".
    foreach ($k in @(
        "VOXELFORGE_ATLAS_MODE", "VOXELFORGE_GLASS_OPAQUE", "VOXELFORGE_MAP_LOAD",
        "VOXELFORGE_LOOK_CAM", "VOXELFORGE_SHOT", "VOXELFORGE_NOHUD", "VOXELFORGE_SEED",
        "VOXELFORGE_LOOK_FORCE"
    )) { Remove-Item "env:$k" -ErrorAction SilentlyContinue }

    $png = Join-Path $out "$Name.png"
    Remove-Item $png -ErrorAction SilentlyContinue
    $env:VOXELFORGE_SHOT = $png
    $env:VOXELFORGE_NOHUD = "1"
    $env:VOXELFORGE_SEED = "42"
    # `look::enabled_for` is off outside play mode, and an ungraded frame is not
    # the frame a player sees. FORCE turns the shipped look stack on without
    # dragging the whole play-mode scene (NPCs, quests, husk) into the shot.
    $env:VOXELFORGE_LOOK_FORCE = "1"
    foreach ($k in $Env.Keys) { Set-Item "env:$k" $Env[$k] }

    $log = Join-Path $out "$Name.log"
    Write-Host "=== $Name ==="
    & $exe *> $log
    $code = $LASTEXITCODE
    if (Test-Path $png) {
        $len = (Get-Item $png).Length
        Write-Host "  ok exit=$code  $png  ($len bytes)"
    }
    else {
        Write-Host "  FAIL exit=$code  no png; tail of log:"
        Get-Content $log -Tail 15 | ForEach-Object { Write-Host "    $_" }
    }
}

# ---- pair 1: the world atlas on procedural terrain -------------------------
$worldCam = "35,-14,16"
Shoot "world_before" @{ VOXELFORGE_ATLAS_MODE = "off"; VOXELFORGE_LOOK_CAM = $worldCam }
Shoot "world_after"  @{ VOXELFORGE_LOOK_CAM = $worldCam }

# ---- pair 2: the pane, on the glass-demo map -------------------------------
$glassMap = "maps/glass_demo.json"
$glassCam = "0,-8,5"
Shoot "glass_before" @{ VOXELFORGE_MAP_LOAD = $glassMap; VOXELFORGE_LOOK_CAM = $glassCam; VOXELFORGE_GLASS_OPAQUE = "1" }
Shoot "glass_after"  @{ VOXELFORGE_MAP_LOAD = $glassMap; VOXELFORGE_LOOK_CAM = $glassCam }

Write-Host ""
Write-Host "shots in $out"
