# ===========================================================================
# Pixel / WORLD lane - shoot ONE plate of maps/beach_dusk.json.
#
# Same binary, same camera, same env as scripts/_poppy_matmaps_ab_final.ps1
# "after" arm (the arm that produced the graded plate _matmaps_after.png).
# The ONLY thing allowed to differ between two runs of this script is the
# CONTENT of maps/beach_dusk.json. That is what makes the before/after a
# single-variable experiment instead of a vibe.
#
# NO CARGO. Poppy holds the build lock this round. This script only runs an
# exe that already exists on disk.
#
# USAGE
#   powershell -File scripts/_pixel_world_shoot.ps1 -Out _pixel_world_after.png
# ===========================================================================
param(
  [Parameter(Mandatory = $true)][string]$Out
)

$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

$exe = Join-Path $root "_matmaps_ab_exe.exe"
if (-not (Test-Path $exe)) { Write-Output "REFUSED  $exe not found"; exit 2 }

# ---- binary-provenance gate (same gate Poppy's script uses) --------------
# A plate shot by a binary that silently drops the 64px art looks like a
# result and is not one.
$bytes   = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
$hasNew  = ([regex]::Matches($bytes, [regex]::Escape("LOD atlas capped"))).Count
$hasOld  = ([regex]::Matches($bytes, [regex]::Escape("file set ignored, procedural tiles kept"))).Count
$hasLoad = ([regex]::Matches($bytes, [regex]::Escape("VOXELFORGE_MAP_LOAD"))).Count
$hasWater= ([regex]::Matches($bytes, [regex]::Escape("cobblestone"))).Count
Write-Output ("exe    : {0}" -f $exe)
Write-Output ("sha256 : {0}" -f (Get-FileHash $exe -Algorithm SHA256).Hash)
Write-Output ("marker 'LOD atlas capped'  = {0}  (want >=1)" -f $hasNew)
Write-Output ("marker 'procedural kept'   = {0}  (want 0)"   -f $hasOld)
Write-Output ("marker VOXELFORGE_MAP_LOAD = {0}  (want >=1)" -f $hasLoad)
Write-Output ("marker 'cobblestone'       = {0}  (want >=1 : map name table present)" -f $hasWater)
if ($hasNew -lt 1 -or $hasOld -ne 0 -or $hasLoad -lt 1 -or $hasWater -lt 1) {
  Write-Output "REFUSED  this binary cannot load a map / cannot render the 64px set."
  exit 2
}
Write-Output "binary-provenance gate: PASS"

# ---- capture env : byte-identical to the graded plate's arm --------------
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
Remove-Item Env:VOXELFORGE_MAT_MAPS, Env:VOXELFORGE_ATLAS_DIR, `
            Env:VOXELFORGE_LOOK_SUN, Env:VOXELFORGE_LOOK_LIGHT, `
            Env:VOXELFORGE_LOOK_EXPOSURE -ErrorAction SilentlyContinue

$png = Join-Path $root $Out
$log = [System.IO.Path]::ChangeExtension($png, ".log")
Remove-Item $png -Force -ErrorAction SilentlyContinue
$env:VOXELFORGE_SHOT = $png

& $exe --play *> $log
$ec = $LASTEXITCODE

# ---- per-plate assertions ------------------------------------------------
$px      = @(Select-String -Path $log -Pattern 'BLOCK_ART file-backed.*tile_px=64').Count
$pbr     = @(Select-String -Path $log -Pattern '^BLOCK_PBR authored').Count
$skipped = (Select-String -Path $log -Pattern 'MAP_LOAD|skipped' | Select-Object -First 4 | ForEach-Object { $_.Line }) -join " | "
$size    = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }

$note = ""
if ($px -lt 1)   { $note += "  !!NO-64PX-ART" }
if ($size -lt 1) { $note += "  !!NO-FRAME" }
if ($pbr -lt 1)  { $note += "  !!NO-AUTHORED-PBR" }

Write-Output ("plate  : {0}  exit={1}  bytes={2}  tile_px64={3}  authored_pbr={4}{5}" -f `
  $Out, $ec, $size, $px, $pbr, $note)
if (Test-Path $png) {
  Write-Output ("sha256 : {0}" -f (Get-FileHash $png -Algorithm SHA256).Hash)
}
Write-Output ("maplogo: {0}" -f $skipped)
if ($note -ne "") { exit 1 }
exit 0
