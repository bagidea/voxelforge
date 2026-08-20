# ===========================================================================
# Flamingo - DIAGNOSTIC: what is flattening the painted sky dome on beach_dusk?
#
# The dome logs "PAINTED ... gain=4.00 curve=0.30" and its plate is a real
# violet->cream sunset ramp (measured: row0 58,47,82 -> row63 255,225,168), yet
# the shipped frame's sky is a flat near-neutral 71,66,64 .. 95,80,73.
# Something between the LUT and the screen is eating it. Arms below remove one
# suspect at a time. ONE binary, env levers only - no cargo is run here.
#
# USAGE  powershell -File scripts/_pixel_diag_shoot.ps1 [-Exe path] [-Out dir]
# ===========================================================================
param(
  [string]$Exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-sun\perf\voxelforge.exe",
  [string]$Out = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_pixel_skydiag"
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

if (-not (Test-Path $Exe)) { Write-Output "REFUSED  $Exe not found"; exit 2 }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# shared capture env - byte-identical to scripts/_sun_skyground_shoot.ps1
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"

$SWEPT = @("VOXELFORGE_SKY_GAIN","VOXELFORGE_SKY_CURVE","VOXELFORGE_CLOUDS",
           "VOXELFORGE_LOOK_VFOG","VOXELFORGE_LOOK_EXPOSURE","VOXELFORGE_LOOK_FOG",
           "VOXELFORGE_LOOK_ATMOS","VOXELFORGE_SUN_DISC")
foreach ($n in @("VOXELFORGE_ATLAS_DIR","VOXELFORGE_LOOK_SUN","VOXELFORGE_LOOK_LIGHT",
                 "VOXELFORGE_LOOK_AMBIENT","VOXELFORGE_LOOK_FILL","VOXELFORGE_LOOK_SKYGAIN")) {
  Remove-Item "Env:$n" -ErrorAction SilentlyContinue
}

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
  $note = ""
  if ($size -lt 1) { $note = "  !!NO-FRAME" }
  Write-Output ("{0,-16} ec={1}  {2,8} bytes{3}" -f $tag, $ec, $size, $note)
}

Write-Output ("exe : {0}" -f $Exe)
Write-Output ("size: {0} bytes  mtime {1}" -f (Get-Item $Exe).Length, (Get-Item $Exe).LastWriteTime)
Write-Output ""

Shoot "base"        @{}
Shoot "gain16"      @{ VOXELFORGE_SKY_GAIN  = "16" }
Shoot "noclouds"    @{ VOXELFORGE_CLOUDS    = "off" }
Shoot "novfog"      @{ VOXELFORGE_LOOK_VFOG = "off" }
# DistanceFog pushed past the whole scene = the no-haze arm ("off" is not a
# value VOXELFORGE_LOOK_HAZE parses; it would silently fall back to the default).
Shoot "nohaze"      @{ VOXELFORGE_LOOK_FOG  = "5000,6000" }
Shoot "allclear16"  @{ VOXELFORGE_CLOUDS = "off"; VOXELFORGE_LOOK_VFOG = "off";
                       VOXELFORGE_LOOK_FOG = "5000,6000"; VOXELFORGE_SKY_GAIN = "16" }

Write-Output ""
Write-Output "done. frames in $Out"
