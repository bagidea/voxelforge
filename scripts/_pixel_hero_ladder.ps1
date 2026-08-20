# ===========================================================================
# Flamingo/pixel - INTERIOR warm/cool BALANCE ladder. One binary, env only.
#
# Two failure modes are already measured on this scene, and they are opposites:
#
#   hero-look-final-nohud2.png (2026-08-05, the frame the CEO opened)
#       hue bins 2/24 · cool 0.00 % · L_mean 0.259   -> SEPIA. One hue, no split.
#   the shipped baked recipe (PILE A, 048fc04, shot today)
#       hue bins 18/24 · cool 86.8 % · L_mean 0.426  -> BLUE WASH. PILE A flipped
#       the FLAT fill from amber to sky-blue at 3900 lux, so the same "one flat
#       term paints the whole room" defect came back wearing the other hue.
#
# What a golden-hour interior actually is: a WARM key confined to what the window
# throws, COOL shade where only the sky reaches, and albedo surviving in between.
# That is a RATIO between the flat fill, the warm floor-bounce card and the cool
# sky cards - not a hue choice on any single one of them. This ladder walks it.
#
# USAGE  powershell -File scripts/_pixel_hero_ladder.ps1 [-Exe path] [-Out dir]
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
  $png = Join-Path $Out "$tag.png"
  $log = Join-Path $Out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  foreach ($n in $SWEPT) {
    if ($levers.ContainsKey($n)) { Set-Item -Path "Env:$n" -Value $levers[$n] }
    else { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
  }
  $env:VOXELFORGE_SHOT = $png
  & $Exe *> $log
  $ec = $LASTEXITCODE
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $note = ""; if ($size -lt 1) { $note = "  !!NO-FRAME" }
  Write-Output ("{0,-10} ec={1}  {2,8} bytes{3}" -f $tag, $ec, $size, $note)
}

Write-Output ("exe : {0}  mtime {1}" -f $Exe, (Get-Item $Exe).LastWriteTime)
Write-Output ""

# Common to every rung: the 08-05 framing the CEO's frame was shot on.
$C = @{ VOXELFORGE_CAM = $HERO_CAM }

function Rung([string]$tag, [hashtable]$extra) {
  $h = @{}; foreach ($k in $C.Keys) { $h[$k] = $C[$k] }
  foreach ($k in $extra.Keys) { $h[$k] = $extra[$k] }
  Shoot $tag $h
}

# r1 - halve the flat fill, hand the room back to the warm floor bounce.
Rung "r1" @{ VOXELFORGE_AMBIENT="1800"; VOXELFORGE_AMBCOLOR="0.40,0.52,0.78";
             VOXELFORGE_BOUNCE="2.6";   VOXELFORGE_BOUNCE2="1.1";
             VOXELFORGE_RIM="0.46,0.62,1.0,3200";
             VOXELFORGE_GRADE="-0.01,0.95,1.24"; VOXELFORGE_SHOULDER="0.80";
             VOXELFORGE_EXPOSURE="8.55" }

# r2 - same shape, flat fill cut harder + key up: max warm/cool CONTRAST.
Rung "r2" @{ VOXELFORGE_AMBIENT="1200"; VOXELFORGE_AMBCOLOR="0.42,0.54,0.80";
             VOXELFORGE_BOUNCE="3.2";   VOXELFORGE_BOUNCE2="1.0";
             VOXELFORGE_SUN="19,196,34000";
             VOXELFORGE_RIM="0.46,0.62,1.0,3000";
             VOXELFORGE_GRADE="0.00,1.00,1.26"; VOXELFORGE_SHOULDER="0.78";
             VOXELFORGE_EXPOSURE="8.60" }

# r3 - middle rung, cooler shade retained but warm bounce doubled.
Rung "r3" @{ VOXELFORGE_AMBIENT="2200"; VOXELFORGE_AMBCOLOR="0.36,0.49,0.80";
             VOXELFORGE_BOUNCE="2.2";   VOXELFORGE_BOUNCE2="1.3";
             VOXELFORGE_RIM="0.46,0.62,1.0,3600";
             VOXELFORGE_GRADE="-0.02,0.92,1.22"; VOXELFORGE_SHOULDER="0.84";
             VOXELFORGE_EXPOSURE="8.45" }

# r4 - r2's balance with a WARMER bounce card so the mid-room reads honey.
Rung "r4" @{ VOXELFORGE_AMBIENT="1200"; VOXELFORGE_AMBCOLOR="0.42,0.54,0.80";
             VOXELFORGE_BOUNCE="3.2";   VOXELFORGE_BOUNCE2="1.0";
             VOXELFORGE_BOUNCE1COLOR="1.0,0.58,0.22";
             VOXELFORGE_SUN="19,196,34000";
             VOXELFORGE_RIM="0.46,0.62,1.0,3000";
             VOXELFORGE_GRADE="0.01,1.02,1.26"; VOXELFORGE_SHOULDER="0.78";
             VOXELFORGE_EXPOSURE="8.60" }

# r5 - control: PILE A default with ONLY the flat fill power pulled back.
Rung "r5" @{ VOXELFORGE_AMBIENT="1500" }

Write-Output ""
Write-Output "done. frames in $Out"
