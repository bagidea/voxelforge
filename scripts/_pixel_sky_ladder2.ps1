# ===========================================================================
# Flamingo/pixel - SKY ladder round 2. Brackets the 1.80 target.
#
# Round 1 (scripts/_pixel_sky_ladder.ps1, one binary, VOXELFORGE_LOOK_VFOG only):
#   density  ratio   sky_med  gnd_med
#   0.030    0.787    75.8     96.3   <- shipped default, the fog wall
#   0.020    0.889    88.2     99.2
#   0.014    1.190   121.5    102.0
#   0.010    1.443   152.8    105.9
#   0.006    1.729   189.0    109.3
#   off      1.916   227.4    118.7
# Monotonic in density, so 1.80 sits between 0.006 and "off". This round walks
# 0.008 / 0.005 / 0.004 to land the rung, and re-shoots 0.006 as the round-to-
# round repeat so the capture's own noise floor is on the record.
#
# USAGE  powershell -File scripts/_pixel_sky_ladder2.ps1
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
                 "VOXELFORGE_LOOK_EXPOSURE","VOXELFORGE_LOOK_FOG","VOXELFORGE_SKY_GAIN",
                 "VOXELFORGE_SKY_CURVE")) {
  Remove-Item "Env:$n" -ErrorAction SilentlyContinue
}

function Shoot([string]$tag, [string]$density) {
  $png = Join-Path $Out "$tag.png"; $log = Join-Path $Out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  Set-Item -Path "Env:VOXELFORGE_LOOK_VFOG" -Value $density
  $env:VOXELFORGE_SHOT = $png
  & $Exe --play *> $log
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $note = ""; if ($size -lt 1) { $note = "  !!NO-FRAME" }
  Write-Output ("{0,-10} vfog={1,-6} ec={2}  {3,8} bytes{4}" -f $tag, $density, $LASTEXITCODE, $size, $note)
}

Write-Output ("exe : {0}  mtime {1}" -f $Exe, (Get-Item $Exe).LastWriteTime)
Write-Output ""
Shoot "v008"       "0.008"
Shoot "v005"       "0.005"
Shoot "v004"       "0.004"
Shoot "v006_rpt"   "0.006"   # noise-floor repeat of round 1's v006
Write-Output ""
Write-Output "done. frames in $Out"
