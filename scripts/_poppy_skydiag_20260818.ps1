# ===========================================================================
# Poppy - WHY IS THE SKY DEAD? The one-binary A/B the art-gap doc asked for.
#
# docs/art-gap-vs-ceo-ref-2026-08-18.md G1 left two live hypotheses and named
# the lever that separates them WITHOUT a rebuild:
#
#   (a) the atmosphere LUT returns ~0 at beach_dusk's sun angle, while the
#       directional key still lights the ground as golden hour  => sky and
#       ground are not coming from the same sun.
#   (b) haze/fog writes over the sky first and the target is never reached.
#
#   ATMOS=off re-enables the gradient SkyDome (look.rs:3523). If
#   sky_ground_ratio jumps => (a).  If it does not move => (b).
#
# Four arms, one exe, one camera, one map. Nothing here writes to a lane file.
#
#   A0_default          shipped default  (atmosphere on, dome off, clouds on)
#   A1_atmosoff         ATMOS=off        (dome ON, clouds off by dependency)
#   A2_atmosoff_noclouds ATMOS=off CLOUDS=off
#   A3_default_noclouds  CLOUDS=off      (is the warm brown the cloud deck?)
#
# CAMERA / SCENE: byte-identical to scripts/_poppy_matmaps_ab_final.ps1, which
# is what produced `_matmaps_after.png` - the frame every number in the art-gap
# doc was measured on. Any other pose would be measuring a different frame.
#
# USAGE  powershell -File scripts/_poppy_skydiag_20260818.ps1
# ===========================================================================
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

$exe = Join-Path $root "target-poppy\release\voxelforge.exe"
if (-not (Test-Path $exe)) { Write-Output "REFUSED  $exe not found"; exit 2 }

$out = Join-Path $root "_poppy_sky"
New-Item -ItemType Directory -Force -Path $out | Out-Null

# ---- binary gate: does this exe carry the levers this run depends on? -----
$bytes  = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
$need   = @("VOXELFORGE_LOOK_ATMOS", "VOXELFORGE_LOOK_SKYGRAD", "VOXELFORGE_CLOUDS", "LOD atlas capped")
$missing = @()
foreach ($m in $need) {
  $n = ([regex]::Matches($bytes, [regex]::Escape($m))).Count
  Write-Output ("marker {0,-26} = {1}" -f $m, $n)
  if ($n -lt 1) { $missing += $m }
}
Write-Output ("sha256 : {0}" -f (Get-FileHash $exe -Algorithm SHA256).Hash.Substring(0, 32))
Write-Output ("size   : {0} bytes   mtime {1}" -f (Get-Item $exe).Length, (Get-Item $exe).LastWriteTime)
if ($missing.Count -gt 0) {
  Write-Output ("REFUSED  exe is missing: {0}" -f ($missing -join ", "))
  exit 2
}
Write-Output "binary-lever gate: PASS"
Write-Output ""

# ---- shared capture env (identical to _poppy_matmaps_ab_final.ps1) --------
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
Remove-Item Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_LOOK_SUN, `
            Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE, `
            Env:VOXELFORGE_MAT_MAPS -ErrorAction SilentlyContinue

function Shoot([string]$tag, [string]$atmos, [string]$clouds) {
  $png = Join-Path $out "$tag.png"
  $log = Join-Path $out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  if ($atmos -ne "") { $env:VOXELFORGE_LOOK_ATMOS = $atmos }
  else { Remove-Item Env:VOXELFORGE_LOOK_ATMOS -ErrorAction SilentlyContinue }
  if ($clouds -ne "") { $env:VOXELFORGE_CLOUDS = $clouds }
  else { Remove-Item Env:VOXELFORGE_CLOUDS -ErrorAction SilentlyContinue }
  $env:VOXELFORGE_SHOT = $png

  & $exe --play *> $log
  $ec = $LASTEXITCODE

  $dome  = @(Select-String -Path $log -Pattern 'sky-dome spawned').Count
  $atm   = @(Select-String -Path $log -Pattern 'LOOK atmosphere').Count
  $size  = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  Write-Output ("[{0}] {1,-22} ATMOS={2,-5} CLOUDS={3,-5} exit={4} png={5}B dome={6} atmos={7}" -f `
    (Get-Date -Format HH:mm:ss), $tag, $(if ($atmos -eq "") { "<def>" } else { $atmos }), `
    $(if ($clouds -eq "") { "<def>" } else { $clouds }), $ec, $size, $dome, $atm)
}

Shoot "A0_default"           ""    ""
Shoot "A1_atmosoff"          "off" ""
Shoot "A2_atmosoff_noclouds" "off" "off"
Shoot "A3_default_noclouds"  ""    "off"

Write-Output ""
Write-Output ("plates in {0}" -f $out)
