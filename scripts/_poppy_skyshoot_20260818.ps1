# ===========================================================================
# Poppy - the painted-sky BEFORE/AFTER pair, from ONE binary.
#
#   _poppy_sky\before.png   ATMOS=lut SKY_PAINT=off CLOUDS=off SUN_DISC=off
#                           = the shipped default of 2026-08-17, byte for byte:
#                             Bevy's physical atmosphere owns the sky, no dome,
#                             no cloud deck, no solar billboard. This is the
#                             frame docs/art-gap-vs-ceo-ref-2026-08-18.md graded.
#   _poppy_sky\after.png    every lever unset = the new shipped default:
#                             painted dome + cloud deck + sun disc.
#
# Two attribution arms so a win can be assigned to a layer instead of to "the
# sky change":
#   _poppy_sky\after_noclouds.png   CLOUDS=off  (dome + sun only)
#   _poppy_sky\after_nosun.png      SUN_DISC=off (dome + clouds only)
#
# Everything else - exe, map, camera, hour, quality, look stack - is identical
# across all four runs, so the only thing that can move a pixel is the lever.
#
# CAMERA / SCENE: byte-identical to scripts/_poppy_matmaps_ab_final.ps1, which
# produced `_matmaps_after.png` - the frame every number in the art-gap document
# was measured on. Any other pose would be measuring a different picture.
#
# NO CARGO IS RUN HERE. The caller checks `tasklist` for cargo/rustc and builds
# separately; this script only shoots, and it REFUSES if the exe does not carry
# the levers it is about to set (an env var a binary has never heard of is not
# an A/B, it is a null result that looks like one).
#
# USAGE  powershell -File scripts/_poppy_skyshoot_20260818.ps1
# ===========================================================================
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

$exe = Join-Path $root "target-poppy\release\voxelforge.exe"
if (-not (Test-Path $exe)) { Write-Output "REFUSED  $exe not found"; exit 2 }

$out = Join-Path $root "_poppy_sky"
New-Item -ItemType Directory -Force -Path $out | Out-Null

# ---- binary gate: does this exe carry BOTH the levers and the 64px art? ----
# 'LOD atlas capped' is the 55e08e3 marker: without it the exe silently drops
# Monanisa's 64px block set and the ground would differ between this pair and
# the graded baseline for a reason that has nothing to do with the sky.
$bytes = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
$need = @("VOXELFORGE_SKY_PAINT", "VOXELFORGE_SUN_DISC", "VOXELFORGE_CLOUDS",
          "VOXELFORGE_LOOK_ATMOS", "sky-dome PAINTED", "cloud-deck spawned",
          "sun-disc spawned", "LOD atlas capped")
$missing = @()
foreach ($m in $need) {
  $n = ([regex]::Matches($bytes, [regex]::Escape($m))).Count
  Write-Output ("marker {0,-26} = {1}" -f $m, $n)
  if ($n -lt 1) { $missing += $m }
}
$stale = ([regex]::Matches($bytes, [regex]::Escape("file set ignored, procedural tiles kept"))).Count
Write-Output ("marker {0,-26} = {1}  (want 0)" -f "pre-55e08e3 atlas marker", $stale)
Write-Output ("sha256 : {0}" -f (Get-FileHash $exe -Algorithm SHA256).Hash.Substring(0, 32))
Write-Output ("size   : {0} bytes   mtime {1}" -f (Get-Item $exe).Length, (Get-Item $exe).LastWriteTime)
if ($missing.Count -gt 0 -or $stale -ne 0) {
  Write-Output ("REFUSED  exe missing: {0}" -f ($missing -join ", "))
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
            Env:VOXELFORGE_MAT_MAPS, Env:VOXELFORGE_SKY_GAIN, `
            Env:VOXELFORGE_SKY_CURVE, Env:VOXELFORGE_CLOUDS_DRIFT `
            -ErrorAction SilentlyContinue

$bad = 0

# $levers is a hashtable of ENV-NAME -> value; anything not named is REMOVED,
# so an arm can never inherit a lever the previous arm set.
function Shoot([string]$tag, [hashtable]$levers) {
  $png = Join-Path $out "$tag.png"
  $log = Join-Path $out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue

  # VOXELFORGE_LOOK_LIFT is in this set for the same reason the sky levers are:
  # the G8 toe lift (`grade::SHADOW_LIFT_V5`) shipped with this change too, so a
  # `before` that still carried it would already have most of the crushed-blacks
  # fix in it and the pair would under-report what the change did.
  foreach ($n in @("VOXELFORGE_LOOK_ATMOS", "VOXELFORGE_SKY_PAINT",
                   "VOXELFORGE_CLOUDS", "VOXELFORGE_SUN_DISC",
                   "VOXELFORGE_LOOK_SKYGRAD", "VOXELFORGE_LOOK_LIFT")) {
    if ($levers.ContainsKey($n)) { Set-Item -Path "Env:$n" -Value $levers[$n] }
    else { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
  }
  $env:VOXELFORGE_SHOT = $png

  & $exe --play *> $log
  $ec = $LASTEXITCODE

  $dome   = @(Select-String -Path $log -Pattern 'sky-dome PAINTED').Count
  $domeP  = @(Select-String -Path $log -Pattern 'sky-dome spawned').Count
  $cloud  = @(Select-String -Path $log -Pattern 'cloud-deck spawned').Count
  $sun    = @(Select-String -Path $log -Pattern 'sun-disc spawned').Count
  $atm    = @(Select-String -Path $log -Pattern 'LOOK atmosphere spawned').Count
  $px64   = @(Select-String -Path $log -Pattern 'BLOCK_ART file-backed.*tile_px=64').Count
  $size   = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }

  $note = ""
  if ($size -lt 1)  { $note += "  !!NO-FRAME" }
  if ($px64 -lt 1)  { $note += "  !!NO-64PX-ART" }
  # Per-arm expectations. A plate that renders with the lever silently ignored
  # looks like a result and is not one.
  switch ($tag) {
    "before"        { if ($atm -lt 1)   { $note += "  !!NO-ATMOSPHERE" }
                      if ($dome + $domeP + $cloud + $sun -ne 0) { $note += "  !!NEW-SKY-LEAKED-IN" } }
    "after"         { if ($dome -lt 1)  { $note += "  !!NO-PAINTED-DOME" }
                      if ($cloud -lt 1) { $note += "  !!NO-CLOUD-DECK" }
                      if ($sun -lt 1)   { $note += "  !!NO-SUN-DISC" }
                      if ($atm -ne 0)   { $note += "  !!ATMOSPHERE-STILL-ON" } }
    "after_noclouds"{ if ($dome -lt 1)  { $note += "  !!NO-PAINTED-DOME" }
                      if ($cloud -ne 0) { $note += "  !!CLOUDS-LEVER-DID-NOT-BITE" } }
    "after_nosun"   { if ($dome -lt 1)  { $note += "  !!NO-PAINTED-DOME" }
                      if ($sun -ne 0)   { $note += "  !!SUN-LEVER-DID-NOT-BITE" } }
  }
  if ($note -ne "") { $script:bad++ }

  Write-Output ("[{0}] {1,-15} exit={2} png={3}B dome={4} clouds={5} sun={6} atmos={7} px64={8}{9}" -f `
    (Get-Date -Format HH:mm:ss), $tag, $ec, $size, $dome, $cloud, $sun, $atm, $px64, $note)
}

Shoot "before"         @{ VOXELFORGE_LOOK_ATMOS = "lut"; VOXELFORGE_SKY_PAINT = "off";
                          VOXELFORGE_CLOUDS = "off"; VOXELFORGE_SUN_DISC = "off";
                          VOXELFORGE_LOOK_LIFT = "0" }
Shoot "after"          @{ }
Shoot "after_noclouds" @{ VOXELFORGE_CLOUDS = "off" }
Shoot "after_nosun"    @{ VOXELFORGE_SUN_DISC = "off" }

Write-Output ""
Write-Output ("ALL DONE - {0} bad plate(s).  plates in {1}" -f $bad, $out)
if ($bad -gt 0) { exit 1 }
