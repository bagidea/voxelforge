# ===========================================================================
# Flamingo/pixel - the SKY ladder. ONE binary, env levers only, no cargo.
#
# ROOT CAUSE (measured by scripts/_pixel_diag_shoot.ps1, 2026-08-20): the painted
# dome is NOT dark. The play FogVolume is eating it. At PLAY_FOG_DENSITY 0.030
# over a 400-unit box the optical depth along a horizontal view ray is ~12, so
# transmittance to the dome is e^-12 ~ 0 and the sky is replaced, wholesale, by
# flat golden fog: sky_median_L 75.8 / ratio 0.788. VOXELFORGE_LOOK_VFOG=off on
# the SAME binary -> sky_median_L 227.4 / ratio 1.919 (target 1.80).
#
# "off" is not the fix - it deletes the god rays another lane shipped. This
# ladder finds the density that lets the authored sky through while the medium
# still carries shafts, and pairs it with SKY_GAIN rungs so the sky can be
# BRIGHTENED from its own base_color rather than merely un-fogged.
#
# USAGE  powershell -File scripts/_pixel_sky_ladder.ps1 [-Exe path] [-Out dir]
# ===========================================================================
param(
  [string]$Exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-sun\perf\voxelforge.exe",
  [string]$Out = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_pixel_skyladder"
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root
if (-not (Test-Path $Exe)) { Write-Output "REFUSED  $Exe not found"; exit 2 }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
foreach ($n in @("VOXELFORGE_ATLAS_DIR","VOXELFORGE_LOOK_SUN","VOXELFORGE_LOOK_LIGHT",
                 "VOXELFORGE_LOOK_AMBIENT","VOXELFORGE_LOOK_FILL","VOXELFORGE_LOOK_SKYGAIN",
                 "VOXELFORGE_LOOK_EXPOSURE","VOXELFORGE_LOOK_FOG")) {
  Remove-Item "Env:$n" -ErrorAction SilentlyContinue
}

$SWEPT = @("VOXELFORGE_LOOK_VFOG","VOXELFORGE_SKY_GAIN","VOXELFORGE_SKY_CURVE")

function Shoot([string]$tag, [hashtable]$levers) {
  $png = Join-Path $Out "$tag.png"
  $log = Join-Path $Out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  foreach ($n in $SWEPT) {
    if ($levers.ContainsKey($n)) { Set-Item -Path "Env:$n" -Value $levers[$n] }
    else { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
  }
  $env:VOXELFORGE_SHOT = $png
  & $Exe --play *> $log
  $ec = $LASTEXITCODE
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $note = ""; if ($size -lt 1) { $note = "  !!NO-FRAME" }
  Write-Output ("{0,-14} ec={1}  {2,8} bytes{3}" -f $tag, $ec, $size, $note)
}

Write-Output ("exe : {0}" -f $Exe)
Write-Output ("size: {0} bytes  mtime {1}" -f (Get-Item $Exe).Length, (Get-Item $Exe).LastWriteTime)
Write-Output ""

Shoot "v020"    @{ VOXELFORGE_LOOK_VFOG = "0.020" }
Shoot "v014"    @{ VOXELFORGE_LOOK_VFOG = "0.014" }
Shoot "v010"    @{ VOXELFORGE_LOOK_VFOG = "0.010" }
Shoot "v006"    @{ VOXELFORGE_LOOK_VFOG = "0.006" }
Shoot "v010g6"  @{ VOXELFORGE_LOOK_VFOG = "0.010"; VOXELFORGE_SKY_GAIN = "6" }
Shoot "v006g3"  @{ VOXELFORGE_LOOK_VFOG = "0.006"; VOXELFORGE_SKY_GAIN = "3" }
Shoot "v014g6"  @{ VOXELFORGE_LOOK_VFOG = "0.014"; VOXELFORGE_SKY_GAIN = "6" }

Write-Output ""
Write-Output "done. frames in $Out"
