# ===========================================================================
# Outdoor vista plates for the P0-ENV art-gap gate.
#
# The gate was reading GAP because it compared an INDOOR kitchen frame against
# docs/refs/ceo_ref_sunset_valley.jpg - an outdoor valley. Different scene class,
# so sky / atmospheric-perspective / silhouette axes had nothing to measure.
# These plates are the matching class: long view, sky in frame, ground + water +
# foliage in frame.
#
# NO CARGO IS RUN HERE. Shoots with whatever exe is passed in (-Exe).
#
# HUD: VOXELFORGE_NOHUD=1 is scene.rs's capture sweep - it hides every
# screen-space widget, and quest.rs never spawns the [E] prompt block under it.
# So these frames are HUD-free by construction (same footing as shot_main.rs),
# which is what the -nohud2 suffix claims. The script REFUSES to write a plate
# from an exe that has no VOXELFORGE_NOHUD string in it.
#
# USAGE  powershell -File scripts/_poppy_outdoor_plates.ps1 [-Exe path] [-Out dir]
# ===========================================================================
param(
  [string]$Exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-poppy\release\voxelforge.exe",
  [string]$Out = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_artgap_outdoor",
  [string[]]$Only = @()
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

function Fail([string]$m) { Write-Output "REFUSED  $m"; exit 2 }
if (-not (Test-Path $Exe)) { Fail "$Exe not found" }

# ---- the exe must actually support the capture sweep ----------------------
$bytes = [IO.File]::ReadAllBytes($Exe)
$text  = [Text.Encoding]::GetEncoding(28591).GetString($bytes)
foreach ($needle in @("VOXELFORGE_NOHUD", "VOXELFORGE_CINE", "VOXELFORGE_SHOT", "VOXELFORGE_MAP_LOAD")) {
  $n = ([regex]::Matches($text, [regex]::Escape($needle))).Count
  Write-Output ("exe-has  {0,-24} {1}" -f $needle, $n)
  if ($n -lt 1) { Fail "$Exe has no $needle - it cannot produce a HUD-free scripted plate" }
}
$fog = ([regex]::Matches($text, [regex]::Escape("VOXELFORGE_LOOK_FOGCOOL"))).Count
Write-Output ("exe-has  {0,-24} {1}   (0 = pre-e4ceb80: sky regrade is NOT in these pixels)" -f "VOXELFORGE_LOOK_FOGCOOL", $fog)
Write-Output ("exe      {0}  {1} bytes  mtime {2}  md5 {3}" -f $Exe, $bytes.Length, (Get-Item $Exe).LastWriteTime, (Get-FileHash $Exe -Algorithm MD5).Hash)
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# ---- plate table: static cine cameras, class-matched to the valley ref ----
# eye and aim are repeated (eye_a == eye_b) so the camera is parked, not moving:
# VOXELFORGE_SHOT fires at t>3.2s and the app quits at 4.4s.
$plates = @(
  @{ tag = "valleywide";  map = "maps/river_sunset.json"; cine = "-16,24,32, -16,24,32, 46,10,32, 1" },
  @{ tag = "riverbend";   map = "maps/river_sunset.json"; cine = "32,18,-12, 32,18,-12, 34,8,50, 1" },
  @{ tag = "highland";    map = "maps/edhari.json";       cine = "-14,28,30, -14,28,30, 48,12,32, 1" },
  @{ tag = "shorehorizon";map = "maps/beach_dusk.json";   cine = "8,14,-10, 8,14,-10, 40,5,44, 1" },
  # valleywide's aim sits ON the far bank, so under haze the whole frame is one
  # low-gradient region: sky_mask() floods from the top edge straight down to the
  # bottom, every column gets dropped as "a curtain, not a lid", and 5 far-field
  # axes come back UNMEASURABLE. Same viewpoint, aim raised 10 blocks so the
  # skyline (the far tree/rock columns) cuts a hard edge under the dome.
  @{ tag = "valleyridge"; map = "maps/river_sunset.json"; cine = "-16,26,32, -16,26,32, 46,20,32, 1" }
)
if ($Only.Count -gt 0) { $plates = $plates | Where-Object { $Only -contains $_.tag } }

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
Remove-Item Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_LOOK_SUN, `
            Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE, `
            Env:VOXELFORGE_LOOK_AMBIENT, Env:VOXELFORGE_LOOK_FILL, `
            Env:VOXELFORGE_SKY_GAIN, Env:VOXELFORGE_SKY_CURVE `
            -ErrorAction SilentlyContinue

Write-Output ""
foreach ($p in $plates) {
  $png = Join-Path $Out ("{0}-nohud2.png" -f $p.tag)
  $log = Join-Path $Out ("{0}.log" -f $p.tag)
  Remove-Item $png -Force -ErrorAction SilentlyContinue

  $env:VOXELFORGE_MAP_LOAD = $p.map
  $env:VOXELFORGE_CINE     = $p.cine
  $env:VOXELFORGE_SHOT     = $png

  & $Exe --play *> $log
  $ec = $LASTEXITCODE

  $err  = @(Select-String -Path $log -Pattern '^error').Count
  $cine = @(Select-String -Path $log -Pattern '^CINE eye').Count
  $saved= @(Select-String -Path $log -Pattern '^SHOT saved to').Count
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }

  $note = ""
  if ($size -lt 1)  { $note += "  !!NO-FRAME" }
  if ($err -gt 0)   { $note += "  !!ERRORS=$err" }
  if ($cine -lt 1)  { $note += "  !!CINE-IGNORED" }
  if ($saved -lt 1) { $note += "  !!NO-SHOT-LINE" }
  Write-Output ("{0,-13} ec={1} {2,9} bytes  cine={3} shot={4} map={5}{6}" -f `
    $p.tag, $ec, $size, $cine, $saved, $p.map, $note)
}
Write-Output ""
Write-Output "plates in $Out"
