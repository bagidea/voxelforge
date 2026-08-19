# ===========================================================================
# Poppy - sky lever sweep, 2026-08-19 evening round.
#
# WHY A SWEEP AND NOT A REBUILD. The 16:01 exe already carries the painted
# dome, and it renders the sky as a FLAT neutral band rgb(71,66,64): the
# violet-to-cream ramp Monanisa authored (L span 187.8, hue arc 147 deg) is
# reaching the frame as L range 17.33 / hue span 20.0 - i.e. essentially one
# LUT row. Before changing source I have to know WHICH of the two things is
# true, and both are already env levers, so neither needs a relink:
#
#   VOXELFORGE_SKY_GAIN   - if the band does not move with gain, that band is
#                           NOT the dome and the dome is not on screen at all.
#   VOXELFORGE_SKY_CURVE  - if it moves with gain but not with curve, the dome
#                           IS on screen but every vertex samples the same v,
#                           so the elevation->ramp mapping is collapsed.
#
# Env block is copied verbatim from _poppy_skyshoot_20260818.ps1 so these
# plates are comparable with _poppy_sky/after.png, which is this sweep's
# control arm (gain 2.6 / curve 0.45 = the shipped defaults).
#
# USAGE  powershell -File scripts/_poppy_skysweep_20260819.ps1
# ===========================================================================
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

$exe = Join-Path $root "target-poppy\release\voxelforge.exe"
if (-not (Test-Path $exe)) { Write-Output "REFUSED  $exe not found"; exit 2 }

# Binary gate: this exe must carry the painted-sky levers by NAME, or a sweep
# of them measures nothing. Scanning the bytes, never the mtime.
$bytes = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
$missing = @()
foreach ($s in @("VOXELFORGE_SKY_GAIN", "VOXELFORGE_SKY_CURVE", "sky-dome PAINTED")) {
  if ($bytes.IndexOf($s) -lt 0) { $missing += $s }
}
if ($missing.Count -gt 0) { Write-Output ("REFUSED  exe missing: {0}" -f ($missing -join ", ")); exit 2 }
Write-Output ("binary-lever gate: PASS   mtime {0}" -f (Get-Item $exe).LastWriteTime)

$out = Join-Path $root "_poppy_sky\sweep"
New-Item -ItemType Directory -Force -Path $out | Out-Null

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
Remove-Item Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_LOOK_SUN, `
            Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE, `
            Env:VOXELFORGE_MAT_MAPS, Env:VOXELFORGE_LOOK_ATMOS, `
            Env:VOXELFORGE_SKY_PAINT, Env:VOXELFORGE_LOOK_SKYGRAD `
            -ErrorAction SilentlyContinue

# tag -> gain, curve. "ctl" repeats the shipped defaults so the sweep carries
# its own reproduction of _poppy_sky/after.png instead of trusting a 2h-old
# plate shot by a different script.
$arms = @(
  @{ tag = "ctl";        gain = "2.6"; curve = "0.45" },
  @{ tag = "gain8";      gain = "8.0"; curve = "0.45" },
  @{ tag = "curve012";   gain = "2.6"; curve = "0.12" },
  @{ tag = "gain8c012";  gain = "8.0"; curve = "0.12" }
)

foreach ($a in $arms) {
  $png = Join-Path $out ("{0}.png" -f $a.tag)
  $log = Join-Path $out ("{0}.log" -f $a.tag)
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  $env:VOXELFORGE_SKY_GAIN  = $a.gain
  $env:VOXELFORGE_SKY_CURVE = $a.curve
  $env:VOXELFORGE_SHOT      = $png

  & $exe --play *> $log

  $dome = @(Select-String -Path $log -Pattern 'sky-dome PAINTED').Count
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $note = ""
  if ($size -lt 1) { $note += "  !!NO-FRAME" }
  if ($dome -lt 1) { $note += "  !!NO-PAINTED-DOME" }
  Write-Output ("[{0,-10}] gain={1,-4} curve={2,-5} png={3}B dome={4}{5}" -f `
                $a.tag, $a.gain, $a.curve, $size, $dome, $note)
}
Write-Output "SWEEP DONE"
