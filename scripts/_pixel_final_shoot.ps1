# ===========================================================================
# Flamingo/pixel - THE DELIVERABLE. before/after for both scenes, ONE binary
# each, env levers only, no cargo.
#
# BEFORE is not an old file. It is TODAY's binary with the OLD constants pushed
# back in through the env hooks those constants already had, so the only thing
# that differs between the two frames of a pair is the constant named in the
# caption - not the exe, not the atlas, not the scene, not the camera.
# (verify-default-reproduce, run in the honest direction: the AFTER arm passes
# ZERO levers, so it is the baked default and not an env crutch.)
#
#   scene 1  INTERIOR   hero.rs kitchen, the 2026-08-05 hero framing
#   scene 2  VISTA      look.rs play stack, pinned beach_dusk vista
#
# USAGE  powershell -File scripts/_pixel_final_shoot.ps1
# ===========================================================================
param(
  [string]$ShotExe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-pixel\release\voxelforge_shot.exe",
  [string]$PlayExe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-pixel\release\voxelforge.exe",
  [string]$Out     = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_pixel_final"
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root
foreach ($e in @($ShotExe, $PlayExe)) {
  if (-not (Test-Path $e)) { Write-Output "REFUSED  $e not found"; exit 2 }
}
New-Item -ItemType Directory -Force -Path $Out | Out-Null

Write-Output ("shot exe : {0}  {1} bytes  {2}" -f $ShotExe, (Get-Item $ShotExe).Length, (Get-Item $ShotExe).LastWriteTime)
Write-Output ("play exe : {0}  {1} bytes  {2}" -f $PlayExe, (Get-Item $PlayExe).Length, (Get-Item $PlayExe).LastWriteTime)
Write-Output ""

# ---------------------------------------------------------------- scene 1 --
$HERO_CAM = "7.6,5.9,-5.2,7.6,3.2,6.0,52"
$HERO_ENV = @("VOXELFORGE_CAM","VOXELFORGE_SUN","VOXELFORGE_DOF","VOXELFORGE_FOG",
              "VOXELFORGE_DFOG","VOXELFORGE_EXPOSURE","VOXELFORGE_GRADE",
              "VOXELFORGE_AMBIENT","VOXELFORGE_AMBCOLOR","VOXELFORGE_SHOULDER",
              "VOXELFORGE_BLUESCALE","VOXELFORGE_BOUNCE","VOXELFORGE_BOUNCE2",
              "VOXELFORGE_BOUNCE1COLOR","VOXELFORGE_BOUNCE2COLOR","VOXELFORGE_RIM",
              "VOXELFORGE_DUST","VOXELFORGE_WIDE","VOXELFORGE_CLEAR",
              "VOXELFORGE_SUNCOLOR","VOXELFORGE_PANEHI","VOXELFORGE_PANELO")
$PLAY_ENV = @("VOXELFORGE_LOOK_VFOG","VOXELFORGE_SKY_GAIN","VOXELFORGE_SKY_CURVE",
              "VOXELFORGE_LOOK_EXPOSURE","VOXELFORGE_LOOK_AMBIENT","VOXELFORGE_LOOK_FILL",
              "VOXELFORGE_LOOK_SUN","VOXELFORGE_LOOK_LIGHT","VOXELFORGE_LOOK_FOG",
              "VOXELFORGE_LOOK_SKYGAIN","VOXELFORGE_ATLAS_DIR","VOXELFORGE_CLOUDS")

function ShootHero([string]$tag, [hashtable]$levers) {
  $png = Join-Path $Out "$tag.png"; $log = Join-Path $Out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  foreach ($n in $HERO_ENV) {
    if ($levers.ContainsKey($n)) { Set-Item -Path "Env:$n" -Value $levers[$n] }
    else { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
  }
  Set-Item -Path "Env:VOXELFORGE_CAM" -Value $HERO_CAM   # framing is held on BOTH arms
  $env:VOXELFORGE_SHOT = $png
  & $ShotExe *> $log
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $note = ""; if ($size -lt 1) { $note = "  !!NO-FRAME" }
  Write-Output ("{0,-22} ec={1}  {2,8} bytes{3}" -f $tag, $LASTEXITCODE, $size, $note)
}

function ShootVista([string]$tag, [hashtable]$levers) {
  $png = Join-Path $Out "$tag.png"; $log = Join-Path $Out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  $env:VOXELFORGE_PLAY         = "1"
  $env:VOXELFORGE_NOHUD        = "1"
  $env:VOXELFORGE_LOOK_QUALITY = "ultra"
  $env:VOXELFORGE_CINE_START   = "1.0"
  $env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
  $env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
  foreach ($n in $PLAY_ENV) {
    if ($levers.ContainsKey($n)) { Set-Item -Path "Env:$n" -Value $levers[$n] }
    else { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
  }
  $env:VOXELFORGE_SHOT = $png
  & $PlayExe --play *> $log
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $note = ""; if ($size -lt 1) { $note = "  !!NO-FRAME" }
  Write-Output ("{0,-22} ec={1}  {2,8} bytes{3}" -f $tag, $LASTEXITCODE, $size, $note)
}

Write-Output "-- scene 1  INTERIOR (hero.rs kitchen) --"
# BEFORE = the PILE A constants this commit replaces, pushed back in by env.
ShootHero "interior_before" @{
  VOXELFORGE_AMBIENT   = "3900"
  VOXELFORGE_AMBCOLOR  = "0.30,0.45,0.80"
  VOXELFORGE_BOUNCE    = "1.0"
  VOXELFORGE_BOUNCE2   = "1.9"
  VOXELFORGE_RIM       = "0.46,0.62,1.0,5200"
  VOXELFORGE_GRADE     = "-0.06,0.80,1.16"
  VOXELFORGE_SHOULDER  = "0.92"
  VOXELFORGE_EXPOSURE  = "8.15"
}
# AFTER = the baked default. NO colour lever passed.
ShootHero "interior_after" @{}

Write-Output ""
Write-Output "-- scene 2  VISTA (look.rs play stack, beach_dusk) --"
# BEFORE = PLAY_FOG_DENSITY 0.030, the shipped value this commit replaces.
ShootVista "vista_before" @{ VOXELFORGE_LOOK_VFOG = "0.030" }
# AFTER = the baked default. NO lever passed.
ShootVista "vista_after"  @{}

Write-Output ""
Write-Output "done. frames in $Out"
