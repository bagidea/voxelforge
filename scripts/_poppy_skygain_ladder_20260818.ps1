# ===========================================================================
# Poppy - calibrate SKY_PAINT_GAIN against the real frame, one binary, no relink.
#
# WHY A LADDER AND NOT ARITHMETIC. The painted dome is UNLIT, so its base_color
# lands in the HDR target verbatim - and then passes TonyMcMapface plus the
# ColorGrading stack, a 3-D LUT this lane cannot invert on paper. So the gain
# that puts the sky's median L where the reference has it (sky_ground_ratio 1.80,
# sky median ~150 on the CEO plate) is MEASURED, not derived.
#
# Every rung is the same exe, the same map, the same camera, the same hour, with
# only VOXELFORGE_SKY_GAIN moving. The number that lands is then written into
# look.rs's SKY_PAINT_GAIN so the shipped default needs no env at all.
#
# The grader is Flamingo's `_pixel_artgap_grade.py` - the same instrument the
# art-gap document was written with, so a rung's number is comparable to the
# document's numbers instead of to a metric invented here.
#
# USAGE  powershell -File scripts/_poppy_skygain_ladder_20260818.ps1 [-Rungs "1.6,2.2,2.6,3.2,4.0"]
# ===========================================================================
param([string]$Rungs = "1.6,2.2,2.6,3.2,4.0")

$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

$exe = Join-Path $root "target-poppy\release\voxelforge.exe"
if (-not (Test-Path $exe)) { Write-Output "REFUSED  $exe not found"; exit 2 }

$bytes = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
foreach ($m in @("VOXELFORGE_SKY_GAIN", "sky-dome PAINTED")) {
  $n = ([regex]::Matches($bytes, [regex]::Escape($m))).Count
  Write-Output ("marker {0,-24} = {1}" -f $m, $n)
  if ($n -lt 1) { Write-Output "REFUSED  exe has no $m - a sweep of a lever it never reads is a null result"; exit 2 }
}
Write-Output ""

$out = Join-Path $root "_poppy_sky\ladder"
New-Item -ItemType Directory -Force -Path $out | Out-Null

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = "maps/beach_dusk.json"
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
Remove-Item Env:VOXELFORGE_LOOK_ATMOS, Env:VOXELFORGE_SKY_PAINT, `
            Env:VOXELFORGE_CLOUDS, Env:VOXELFORGE_SUN_DISC, `
            Env:VOXELFORGE_LOOK_SUN, Env:VOXELFORGE_LOOK_EXPOSURE `
            -ErrorAction SilentlyContinue

$plates = @()
foreach ($g in ($Rungs -split ",")) {
  $g = $g.Trim()
  if ($g -eq "") { continue }
  $tag = "gain_" + ($g -replace "\.", "p")
  $png = Join-Path $out "$tag.png"
  $log = Join-Path $out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  $env:VOXELFORGE_SKY_GAIN = $g
  $env:VOXELFORGE_SHOT     = $png
  & $exe --play *> $log
  $dome = @(Select-String -Path $log -Pattern "sky-dome PAINTED").Count
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $seen = (Select-String -Path $log -Pattern "gain=([0-9.]+)" | Select-Object -First 1).Matches.Groups[1].Value
  $ok = ($dome -ge 1 -and $size -gt 0 -and [math]::Abs([double]$seen - [double]$g) -lt 0.005)
  Write-Output ("[{0}] gain={1,-5} png={2,9}B dome={3} log_gain={4} {5}" -f `
    (Get-Date -Format HH:mm:ss), $g, $size, $dome, $seen, $(if ($ok) { "" } else { "!!LEVER-DID-NOT-BITE" }))
  if ($ok) { $plates += $png }
}

Remove-Item Env:VOXELFORGE_SKY_GAIN -ErrorAction SilentlyContinue

if ($plates.Count -eq 0) { Write-Output "no usable rungs"; exit 1 }
Write-Output ""
Write-Output "grading the ladder ..."
& python scripts/_pixel_artgap_grade.py @plates --json (Join-Path $out "ladder.json") --label "poppy sky-gain ladder"
