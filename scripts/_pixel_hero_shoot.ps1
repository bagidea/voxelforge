# ===========================================================================
# Flamingo/pixel - the INTERIOR hue-separation A/B, one binary, env levers only.
#
# The frame the CEO opened (hero-look-final-nohud2.png) is dated 2026-08-05 and
# was shot on the pre-PILE-A recipe (amber flat fill, temperature +0.02,
# saturation 1.00, warm bounce card 2, golden window pane). PILE A landed
# 2026-08-18 (048fc04) and baked the warm/cool split into hero.rs's own consts.
# So the BEFORE arm below RE-CREATES the 08-05 recipe from env on TODAY's binary
# and the AFTER arm takes the baked default - same exe, same camera, same scene,
# only the colour recipe moving.
#
# NO CARGO IS RUN HERE.
# USAGE  powershell -File scripts/_pixel_hero_shoot.ps1 [-Exe path] [-Out dir]
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

# The 2026-08-05 hero framing (docs/note-hero-look-final-recipe.md line 16).
$HERO_CAM = "7.6,5.9,-5.2,7.6,3.2,6.0,52"

$SWEPT = @("VOXELFORGE_CAM","VOXELFORGE_SUN","VOXELFORGE_DOF","VOXELFORGE_FOG",
           "VOXELFORGE_DFOG","VOXELFORGE_EXPOSURE","VOXELFORGE_GRADE",
           "VOXELFORGE_AMBIENT","VOXELFORGE_AMBCOLOR","VOXELFORGE_SHOULDER",
           "VOXELFORGE_BLUESCALE","VOXELFORGE_BOUNCE","VOXELFORGE_BOUNCE2",
           "VOXELFORGE_RIM","VOXELFORGE_DUST","VOXELFORGE_WIDE","VOXELFORGE_CLEAR")

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
  Write-Output ("{0,-22} ec={1}  {2,8} bytes{3}" -f $tag, $ec, $size, $note)
}

Write-Output ("exe : {0}" -f $Exe)
Write-Output ("size: {0} bytes  mtime {1}" -f (Get-Item $Exe).Length, (Get-Item $Exe).LastWriteTime)
Write-Output ""

# ---- BEFORE: the 2026-08-05 recipe, reconstructed from env ----------------
#      cam/fog/dof/sun/ambient/exposure = docs/note-hero-look-final-recipe.md
#      colour consts = the pre-PILE-A values recorded in hero.rs's own recipe mod
Shoot "before_2026-08-05" @{
  VOXELFORGE_CAM        = $HERO_CAM
  VOXELFORGE_FOG        = "0.032"
  VOXELFORGE_DFOG       = "0.008"
  VOXELFORGE_DOF        = "11,3.2"
  VOXELFORGE_SUN        = "18,196,22000"
  VOXELFORGE_AMBIENT    = "2400"
  VOXELFORGE_EXPOSURE   = "9.9"
  VOXELFORGE_AMBCOLOR   = "0.70,0.60,0.44"
  VOXELFORGE_GRADE      = "0.02,1.00,1.30"
  VOXELFORGE_SHOULDER   = "0.64"
  VOXELFORGE_BLUESCALE  = "0.85"
  VOXELFORGE_BOUNCE2    = "1.7"
}

# ---- AFTER: the shipped baked recipe, SAME camera, zero colour levers ------
Shoot "after_baked"       @{ VOXELFORGE_CAM = $HERO_CAM }

# ---- the baked default's own framing too (no env at all) -------------------
Shoot "after_baked_widecam" @{}

Write-Output ""
Write-Output "done. frames in $Out"
