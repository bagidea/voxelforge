# ===========================================================================
# Flamingo/pixel - light lane. BEFORE/AFTER for "orange-red over the whole image".
#
# THE SYMPTOM. look.rs::spawn_sky_dome loaded sky_gradient_sunset.png (a
# violet -> red -> orange ramp) unconditionally, and SKY_PAINT_GAIN is 4.0, so
# that ramp was the brightest thing in the frame at every hour of every map.
#
# THE FIX (commit c6d3171). sky_plate_name() makes day the default and keeps the
# sunset ramp behind VOXELFORGE_SKY_PLATE=sunset.
#
# WHY ONE BINARY. The boss judges these by eye, side by side. A two-binary A/B
# cannot prove the camera, the hour, the map, the quality tier or the window size
# were the same -- so this shoots BOTH sides from the SAME exe and changes ONE
# env var between them. Everything else is set once, above the loop, and every
# LOOK_* lever is cleared so both frames sit at the binary's own defaults.
#
#   before.png  VOXELFORGE_SKY_PLATE=sunset   <- what ships today
#   after.png   VOXELFORGE_SKY_PLATE=day      <- the new default
#   after_rpt.png                             <- same env as after.png; the
#                                                capture's own noise floor, so
#                                                the before-after delta can be
#                                                read against it instead of
#                                                against zero.
#
# The script REFUSES rather than reporting a half-shot: missing exe, a frame
# that never landed, or a resolution mismatch between the two sides all exit
# non-zero and say so.
#
# USAGE  powershell -File scripts/_pixel_skyplate_ab.ps1
# ===========================================================================
param(
  [string]$Exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_pixel_light_wt\target-pixel-light\release\voxelforge.exe",
  [string]$Out = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_pixel_skyplate_ab",
  [string]$Map = "maps/beach_dusk.json"
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

if (-not (Test-Path $Exe)) { Write-Output "REFUSED  exe not found: $Exe"; exit 2 }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# Provenance: prove this exe carries the fix before trusting a single frame.
# A binary that predates c6d3171 has no day plate wired and would shoot two
# identical sunset frames while the captions claimed otherwise.
$bytes = [System.IO.File]::ReadAllBytes($Exe)
$text  = [System.Text.Encoding]::ASCII.GetString($bytes)
$hasDay = $text.Contains("sky_gradient_day.png")
$hasSunset = $text.Contains("sky_gradient_sunset.png")
Write-Output ("exe        : {0}" -f $Exe)
Write-Output ("mtime      : {0}   {1:N0} bytes" -f (Get-Item $Exe).LastWriteTime, (Get-Item $Exe).Length)
Write-Output ("plates in exe: day={0}  sunset={1}" -f $hasDay, $hasSunset)
if (-not $hasDay) {
  Write-Output "REFUSED  this exe does not contain sky_gradient_day.png - it predates c6d3171, both sides would be the sunset ramp"
  exit 2
}

# --- everything held constant across both sides ---------------------------
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_MAP_LOAD     = $Map
# pos(41,15,37) -> look-at(29,3,20); the framing my sky ladders already used, so
# the sky band that carries this change is known to be in shot.
$env:VOXELFORGE_CINE         = "41,15,37, 41,15,37, 29,3,20, 1"
foreach ($n in @("VOXELFORGE_ATLAS_DIR","VOXELFORGE_LOOK_SUN","VOXELFORGE_LOOK_LIGHT",
                 "VOXELFORGE_LOOK_AMBIENT","VOXELFORGE_LOOK_FILL","VOXELFORGE_LOOK_SKYGAIN",
                 "VOXELFORGE_LOOK_EXPOSURE","VOXELFORGE_LOOK_FOG","VOXELFORGE_LOOK_VFOG",
                 "VOXELFORGE_SKY_GAIN","VOXELFORGE_SKY_CURVE","VOXELFORGE_SKY_PAINT")) {
  Remove-Item "Env:$n" -ErrorAction SilentlyContinue
}

function Shoot([string]$tag, [string]$plate) {
  $png = Join-Path $Out "$tag.png"
  $log = Join-Path $Out "$tag.log"
  Remove-Item $png -Force -ErrorAction SilentlyContinue
  Set-Item -Path "Env:VOXELFORGE_SKY_PLATE" -Value $plate
  $env:VOXELFORGE_SHOT = $png
  & $Exe --play *> $log
  $ec = $LASTEXITCODE
  if (-not (Test-Path $png)) {
    Write-Output ("{0,-10} plate={1,-7} ec={2}   NO-FRAME" -f $tag, $plate, $ec)
    return $null
  }
  # The engine names the plate it actually loaded. Read it back rather than
  # trusting that the env var we set is the one that reached the dome.
  $loaded = (Select-String -Path $log -Pattern "LOOK sky-plate (loaded|MISSING)" -Encoding utf8 |
             Select-Object -First 1).Line
  Add-Type -AssemblyName System.Drawing
  $img = [System.Drawing.Image]::FromFile($png)
  $dim = "{0}x{1}" -f $img.Width, $img.Height
  $img.Dispose()
  $md5 = (Get-FileHash $png -Algorithm MD5).Hash.Substring(0,12)
  Write-Output ("{0,-10} plate={1,-7} ec={2}  {3,10:N0} bytes  {4,-11} md5={5}" -f `
                $tag, $plate, $ec, (Get-Item $png).Length, $dim, $md5)
  if ($loaded) { Write-Output ("             {0}" -f $loaded.Trim()) }
  return [pscustomobject]@{ Tag=$tag; Png=$png; Dim=$dim; Md5=$md5 }
}

Write-Output ""
$b = Shoot "before"    "sunset"
$a = Shoot "after"     "day"
$r = Shoot "after_rpt" "day"
Write-Output ""

if (-not $b -or -not $a) { Write-Output "REFUSED  a side is missing - no pair to judge"; exit 1 }
if ($b.Dim -ne $a.Dim) {
  Write-Output ("REFUSED  resolution mismatch before={0} after={1} - not a valid side-by-side" -f $b.Dim, $a.Dim)
  exit 1
}
if ($b.Md5 -eq $a.Md5) {
  Write-Output "REFUSED  before and after are byte-identical - the lever did not move the frame"
  exit 1
}
Write-Output ("PAIR OK   {0}   before!=after, same resolution" -f $a.Dim)
if ($r) {
  $floor = if ($r.Md5 -eq $a.Md5) { "0 (bit-identical re-shoot)" } else { "non-zero - see after_rpt.png" }
  Write-Output ("noise floor: {0}" -f $floor)
}
Write-Output ""
Write-Output "before : $($b.Png)"
Write-Output "after  : $($a.Png)"
