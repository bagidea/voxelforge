# ===========================================================================
# Poppy - the two plates the Director asked for, from ONE binary.
#
#   _matmaps_after.png    VOXELFORGE_MAT_MAPS unset  -> Monanisa's authored _n/_r
#   _matmaps_before.png   VOXELFORGE_MAT_MAPS=off    -> derived maps only
#
# Everything else (exe, map, camera, hour, quality, look stack) is identical
# between the two runs, so the only thing that can move a pixel is the lever.
#
# SCENE: maps/beach_dusk.json is the only shipped map that carries every
# file-art block kind that exists as a real sim BlockId at once - grass, sand,
# dirt, wood/log, leaves, stone, limestone, brick, red_sand, snow, glass, lamp.
# (`water` and `metal` have tiles in atlas.json but NO BlockId in
# sim/src/block.rs::name(), so no map can contain them and no frame can show
# them. That is a content gap, not a capture failure.)
#
# CAMERA: pose F out of scripts/_poppy_matmaps_camprobe{,2}.ps1 - the pose that
# put brick, plaster, glass, log end-grain, leaves and sand in one frame at a
# distance where 64px structure is actually resolvable.
#
# NO CARGO. The exe is a pre-gated snapshot (_matmaps_ab_exe.exe): it carries
# 'LOD atlas capped' (added by 55e08e3) and NOT 'file set ignored, procedural
# tiles kept' (deleted by it), so it can load the 64px manifest. target/release
# /voxelforge.exe FAILS that gate - it still has the old marker.
#
# Each arm asserts its own log: tile_px=64 must appear, and BLOCK_PBR authored
# must be >0 on `after` and exactly 0 on `before`. A plate that renders with the
# maps silently dropped looks like a result and is not one.
#
# USAGE
#   powershell -File scripts/_poppy_matmaps_ab_final.ps1
# ===========================================================================
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

$exe = Join-Path $root "_matmaps_ab_exe.exe"
if (-not (Test-Path $exe)) { Write-Output "REFUSED  $exe not found"; exit 2 }

# ---- binary-provenance gate: does this exe CONTAIN the fix? --------------
$bytes = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
$hasNew = ([regex]::Matches($bytes, [regex]::Escape("LOD atlas capped"))).Count
$hasOld = ([regex]::Matches($bytes, [regex]::Escape("file set ignored, procedural tiles kept"))).Count
$hasLever = ([regex]::Matches($bytes, [regex]::Escape("VOXELFORGE_MAT_MAPS"))).Count
Write-Output ("exe    : {0}" -f $exe)
Write-Output ("sha256 : {0}" -f (Get-FileHash $exe -Algorithm SHA256).Hash)
Write-Output ("marker 'LOD atlas capped'                    = {0}  (want >=1)" -f $hasNew)
Write-Output ("marker 'file set ignored, procedural tiles'   = {0}  (want 0)" -f $hasOld)
Write-Output ("marker 'VOXELFORGE_MAT_MAPS'                  = {0}  (want >=1)" -f $hasLever)
if ($hasNew -lt 1 -or $hasOld -ne 0 -or $hasLever -lt 1) {
  Write-Output "REFUSED  this binary cannot render the 64px set / has no lever."
  exit 2
}
Write-Output "binary-provenance gate: PASS"
Write-Output ""

# ---- shared capture env --------------------------------------------------
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
Remove-Item Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_LOOK_SUN, `
            Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE `
            -ErrorAction SilentlyContinue

$bad = 0

function Shoot([string]$tag, [string]$lever) {
  $png = Join-Path $root "_matmaps_$tag.png"
  $log = Join-Path $root "_matmaps_$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  if ($lever -eq "off") { $env:VOXELFORGE_MAT_MAPS = "off" }
  else { Remove-Item Env:VOXELFORGE_MAT_MAPS -ErrorAction SilentlyContinue }
  $env:VOXELFORGE_SHOT = $png

  & $exe --play *> $log
  $ec = $LASTEXITCODE

  $px   = @(Select-String -Path $log -Pattern 'BLOCK_ART file-backed.*tile_px=64').Count
  $pbr  = @(Select-String -Path $log -Pattern '^BLOCK_PBR authored').Count
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $note = ""
  if ($px -lt 1)   { $note += "  !!NO-64PX-ART" }
  if ($size -lt 1) { $note += "  !!NO-FRAME" }
  if ($lever -eq "on"  -and $pbr -lt 1)  { $note += "  !!NO-AUTHORED-PBR" }
  if ($lever -eq "off" -and $pbr -ne 0)  { $note += "  !!LEVER-DID-NOT-BITE" }
  if ($note -ne "") { $script:bad++ }
  Write-Output ("[{0}] {1,-6} MAT_MAPS={2,-8} exit={3} png={4}B tile_px64={5} authored_pbr={6}{7}" -f `
    (Get-Date -Format HH:mm:ss), $tag, $(if ($lever -eq "off") { "off" } else { "<unset>" }), $ec, $size, $px, $pbr, $note)
}

Shoot "after"  "on"
Shoot "before" "off"
Write-Output ""
Write-Output ("ALL DONE - {0} bad plate(s)" -f $bad)
