# ===========================================================================
# Flamingo/pixel - INTERIOR ladder round 2. Refines around r1/r3 from round 1.
#
# Round 1 result (scripts/_pixel_measure.py --mode interior):
#   frame                 bins   cluster-hue-spread   warm/cool      L_mean
#   hero-look-final (CEO)  2/24     17.0 deg          100 /  0 %     0.259  SEPIA
#   PILE A baked default  18/24     84.6 deg (all cool) 13 / 87 %    0.426  BLUE WASH
#   r1                    16/24    176.4 deg           78 / 22 %     0.372  <- warm-led
#   r3                    19/24    179.2 deg           67 / 33 %     0.391  <- balanced
#
# r1/r3 both read as a real room again (warm plaster, teal glass, green floor,
# golden window). What they still are is FLAT - pale, low local contrast. Round 2
# holds the warm/cool ratio r1 found and buys contrast + saturation back.
#
# USAGE  powershell -File scripts/_pixel_hero_ladder2.ps1 [-Exe path] [-Out dir]
# ===========================================================================
param(
  [string]$Exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-pixel\release\voxelforge_shot.exe",
  [string]$Out = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_pixel_hero"
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root
if (-not (Test-Path $Exe)) { Write-Output "REFUSED  $Exe not found"; exit 2 }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

$HERO_CAM = "7.6,5.9,-5.2,7.6,3.2,6.0,52"
$SWEPT = @("VOXELFORGE_CAM","VOXELFORGE_SUN","VOXELFORGE_DOF","VOXELFORGE_FOG",
           "VOXELFORGE_DFOG","VOXELFORGE_EXPOSURE","VOXELFORGE_GRADE",
           "VOXELFORGE_AMBIENT","VOXELFORGE_AMBCOLOR","VOXELFORGE_SHOULDER",
           "VOXELFORGE_BLUESCALE","VOXELFORGE_BOUNCE","VOXELFORGE_BOUNCE2",
           "VOXELFORGE_BOUNCE1COLOR","VOXELFORGE_BOUNCE2COLOR",
           "VOXELFORGE_RIM","VOXELFORGE_DUST","VOXELFORGE_WIDE","VOXELFORGE_CLEAR",
           "VOXELFORGE_SUNCOLOR","VOXELFORGE_PANEHI","VOXELFORGE_PANELO")

function Shoot([string]$tag, [hashtable]$levers) {
  $png = Join-Path $Out "$tag.png"; $log = Join-Path $Out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  foreach ($n in $SWEPT) {
    if ($levers.ContainsKey($n)) { Set-Item -Path "Env:$n" -Value $levers[$n] }
    else { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
  }
  $levers["VOXELFORGE_CAM"] = $HERO_CAM
  Set-Item -Path "Env:VOXELFORGE_CAM" -Value $HERO_CAM
  $env:VOXELFORGE_SHOT = $png
  & $Exe *> $log
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $note = ""; if ($size -lt 1) { $note = "  !!NO-FRAME" }
  Write-Output ("{0,-10} ec={1}  {2,8} bytes{3}" -f $tag, $LASTEXITCODE, $size, $note)
}

Write-Output ("exe : {0}  mtime {1}" -f $Exe, (Get-Item $Exe).LastWriteTime)
Write-Output ""

# s1 - r1's ratio, contrast + saturation restored, exposure pulled back so the
#      plaster stops washing out.
Shoot "s1" @{ VOXELFORGE_AMBIENT="1700"; VOXELFORGE_AMBCOLOR="0.38,0.51,0.80";
              VOXELFORGE_BOUNCE="2.6";   VOXELFORGE_BOUNCE2="1.1";
              VOXELFORGE_RIM="0.46,0.62,1.0,3400";
              VOXELFORGE_GRADE="-0.01,1.00,1.30"; VOXELFORGE_SHOULDER="0.74";
              VOXELFORGE_EXPOSURE="8.75" }

# s2 - s1 + a punchier key, so the window throw is unmistakably the light source.
Shoot "s2" @{ VOXELFORGE_AMBIENT="1700"; VOXELFORGE_AMBCOLOR="0.38,0.51,0.80";
              VOXELFORGE_BOUNCE="2.6";   VOXELFORGE_BOUNCE2="1.1";
              VOXELFORGE_SUN="19,196,32000";
              VOXELFORGE_RIM="0.46,0.62,1.0,3400";
              VOXELFORGE_GRADE="-0.01,1.00,1.32"; VOXELFORGE_SHOULDER="0.72";
              VOXELFORGE_EXPOSURE="8.80" }

# s3 - s2 with the cool side given more room (r3's ratio) - the balanced arm.
Shoot "s3" @{ VOXELFORGE_AMBIENT="2100"; VOXELFORGE_AMBCOLOR="0.34,0.48,0.80";
              VOXELFORGE_BOUNCE="2.3";   VOXELFORGE_BOUNCE2="1.25";
              VOXELFORGE_SUN="19,196,32000";
              VOXELFORGE_RIM="0.46,0.62,1.0,3800";
              VOXELFORGE_GRADE="-0.02,0.98,1.30"; VOXELFORGE_SHOULDER="0.74";
              VOXELFORGE_EXPOSURE="8.70" }

Write-Output ""
Write-Output "done. frames in $Out"
