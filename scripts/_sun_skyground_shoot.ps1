# ===========================================================================
# Sun - sky-brighter-than-ground BEFORE/AFTER, ONE binary, env-lever sweep.
#
#   before.png   default env = shipped look constants (ev100 10.3, ambient 620,
#                bounce 1100, sky_fill 1400) at the art-gap vista camera.
#   after*.png   ground-darkening lever(s) applied, SAME camera + SAME hour.
#
# The ONLY thing that differs between before and after is the lever named in
# the filename - everything else (exe, map, CINE camera, hour, quality) is
# byte-identical, so any pixel that moves is the lever's doing.
#
# NO CARGO IS RUN HERE. Build separately (`--target-dir target-sun -j 2`).
#
# USAGE  powershell -File scripts/_sun_skyground_shoot.ps1 [-Exe path] [-Out dir]
# ===========================================================================
param(
  [string]$Exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-sun\perf\voxelforge.exe",
  [string]$Out = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_sun_skyground"
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

if (-not (Test-Path $Exe)) { Write-Output "REFUSED  $Exe not found"; exit 2 }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# ---- shared capture env: byte-identical to _poppy_skyshoot_20260818.ps1 ----
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
Remove-Item Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_LOOK_SUN, `
            Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE, `
            Env:VOXELFORGE_LOOK_AMBIENT, Env:VOXELFORGE_LOOK_FILL, `
            Env:VOXELFORGE_SKY_GAIN, Env:VOXELFORGE_SKY_CURVE `
            -ErrorAction SilentlyContinue

# $levers = hashtable ENV -> value; anything not named is REMOVED so no arm can
# inherit a lever the previous arm set. "before" passes an empty table.
function Shoot([string]$tag, [hashtable]$levers) {
  $png = Join-Path $Out "$tag.png"
  $log = Join-Path $Out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue

  foreach ($n in @("VOXELFORGE_LOOK_EXPOSURE", "VOXELFORGE_LOOK_AMBIENT",
                   "VOXELFORGE_LOOK_FILL")) {
    if ($levers.ContainsKey($n)) { Set-Item -Path "Env:$n" -Value $levers[$n] }
    else { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
  }
  $env:VOXELFORGE_SHOT = $png

  & $Exe --play *> $log
  $ec = $LASTEXITCODE

  $err  = @(Select-String -Path $log -Pattern '^error').Count
  $paint= @(Select-String -Path $log -Pattern 'sky-dome PAINTED').Count
  $dome = @(Select-String -Path $log -Pattern 'sky-dome spawned').Count
  $px64 = @(Select-String -Path $log -Pattern 'BLOCK_ART file-backed').Count
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }

  $note = ""
  if ($size -lt 1)  { $note += "  !!NO-FRAME" }
  if ($err -gt 0)   { $note += "  !!ERRORS=$err" }
  if ($paint+$dome -lt 1) { $note += "  !!NO-DOME" }

  Write-Output ("{0,-12} ec={1}  {2,7} bytes  dome={3} px64={4}{5}" -f `
    $tag, $ec, $size, ($paint+$dome), $px64, $note)
  return $ec
}

Write-Output "exe : $Exe"
Write-Output ("size: {0} bytes  mtime {1}" -f (Get-Item $Exe).Length, (Get-Item $Exe).LastWriteTime)
Write-Output ""

# ---- BEFORE: shipped defaults, no levers ----
Shoot "before" @{}

# ---- sweep arms (ground-darkening candidates; AFTER is picked after measuring) ----
# ev100 UP = darker frame; sky dome is exposure-invariant, lit ground is NOT.
Shoot "ev11.2"   @{ VOXELFORGE_LOOK_EXPOSURE = "11.2" }
Shoot "ev11.8"   @{ VOXELFORGE_LOOK_EXPOSURE = "11.8" }
# ambient DOWN = darker flat bounce-fill (the ground floor term).
Shoot "amb420"   @{ VOXELFORGE_LOOK_AMBIENT = "420" }
Shoot "amb300"   @{ VOXELFORGE_LOOK_AMBIENT = "300" }
# bounce/sky-fill DOWN = darker directional ground fills.
Shoot "fill800"  @{ VOXELFORGE_LOOK_FILL = "900,700" }

Write-Output ""
Write-Output "done. frames in $Out"
