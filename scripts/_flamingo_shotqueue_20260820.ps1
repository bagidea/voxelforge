# ===========================================================================
# Flamingo shot queue - 2026-08-20.  TEN frames, ONE binary, env levers only.
#
# NO CARGO IS RUN HERE. Pass the exe in with -Exe; the script refuses if it is
# missing or if it lacks the env strings each set needs (a lever that is not in
# the binary silently makes BEFORE == AFTER, which is the trap this repo has
# paid for more than once).
#
# Why one binary: a rebuilt exe is not a controlled comparison. Every pair below
# differs by exactly one env var, on the same pixels, same camera, same hour.
#
# HUD: VOXELFORGE_NOHUD=1 is scene.rs's capture sweep - no screen-space widget
# is drawn, so the frames are HUD-free by construction and may carry the
# -nohud2 suffix (same footing as scripts/_poppy_outdoor_plates.ps1).
#
# USAGE
#   powershell -File scripts/_flamingo_shotqueue_20260820.ps1 `
#       -Exe "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-yamamoto\release\voxelforge.exe"
#   ... or add  -Only tex          (or water / godray) to shoot one set.
# ===========================================================================
param(
  [string]$Exe  = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-yamamoto\release\voxelforge.exe",
  [string]$Out  = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_fl_shotqueue_20260820",
  [string[]]$Only = @()
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

function Fail([string]$m) { Write-Output "REFUSED  $m"; exit 2 }
if (-not (Test-Path $Exe)) { Fail "$Exe not found - build it first, do not shoot an older exe under a new name" }

# ---- the exe must carry every lever this queue pulls ----------------------
$bytes = [IO.File]::ReadAllBytes($Exe)
$text  = [Text.Encoding]::GetEncoding(28591).GetString($bytes)
foreach ($needle in @("VOXELFORGE_NOHUD","VOXELFORGE_CINE","VOXELFORGE_SHOT","VOXELFORGE_MAP_LOAD",
                      "VOXELFORGE_ATLAS_DIR","VOXELFORGE_WATER","VOXELFORGE_LOOK_VFOG")) {
  $n = ([regex]::Matches($text, [regex]::Escape($needle))).Count
  Write-Output ("exe-has  {0,-24} {1}" -f $needle, $n)
  if ($n -lt 1) { Fail "$Exe has no $needle - that set cannot be an honest A/B against this exe" }
}
Write-Output ("exe      {0}" -f $Exe)
Write-Output ("         {0} bytes  mtime {1}  md5 {2}" -f $bytes.Length, (Get-Item $Exe).LastWriteTime, (Get-FileHash $Exe -Algorithm MD5).Hash)

$AtlasBefore = Join-Path $root "_fl_atlas_before"
if (-not (Test-Path (Join-Path $AtlasBefore "atlas.json"))) {
  Fail "$AtlasBefore\atlas.json missing - rebuild it with:  git archive be365e9 assets/textures/blocks | tar -x -C _fl_atlas_before --strip-components=3"
}
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# ---- the queue -----------------------------------------------------------
# eye and aim repeat (eye_a == eye_b) so the camera is parked, not moving.
# Cameras are the ones already in the 16-plate P0-ENV baseline, so these frames
# are class-matched to docs/refs/ceo_ref_sunset_valley.jpg and gradeable.
$shots = @(
  # SET A - do today's re-textured blocks read better IN THE WORLD, not just as tiles?
  @{ set="tex";    tag="tex-before-valleyridge";  map="maps/river_sunset.json"; cine="-16,26,32, -16,26,32, 46,20,32, 1"; env=@{ VOXELFORGE_ATLAS_DIR=$AtlasBefore } },
  @{ set="tex";    tag="tex-after-valleyridge";   map="maps/river_sunset.json"; cine="-16,26,32, -16,26,32, 46,20,32, 1"; env=@{} },
  @{ set="tex";    tag="tex-before-shorehorizon"; map="maps/beach_dusk.json";   cine="8,14,-10, 8,14,-10, 40,5,44, 1";   env=@{ VOXELFORGE_ATLAS_DIR=$AtlasBefore } },
  @{ set="tex";    tag="tex-after-shorehorizon";  map="maps/beach_dusk.json";   cine="8,14,-10, 8,14,-10, 40,5,44, 1";   env=@{} },

  # SET B - water.wgsl has never been seen. W1 vs W0 is the feature; W2/W3 isolate the terms.
  @{ set="water";  tag="w0-water-off";            map="maps/river_sunset.json"; cine="32,18,-12, 32,18,-12, 34,8,50, 1"; env=@{ VOXELFORGE_WATER="off" } },
  @{ set="water";  tag="w1-water-default";        map="maps/river_sunset.json"; cine="32,18,-12, 32,18,-12, 34,8,50, 1"; env=@{} },
  @{ set="water";  tag="w2-wave-0";               map="maps/river_sunset.json"; cine="32,18,-12, 32,18,-12, 34,8,50, 1"; env=@{ VOXELFORGE_WATER_WAVE="0" } },
  @{ set="water";  tag="w3-depth-0p5";            map="maps/river_sunset.json"; cine="32,18,-12, 32,18,-12, 34,8,50, 1"; env=@{ VOXELFORGE_WATER_DEPTH="0.5" } },

  # SET C - god rays. The camera IS the experiment: the sun must sit behind
  # geometry. If G1 == G0 here, that is a camera verdict first, code second.
  @{ set="godray"; tag="g0-vfog-off";             map="maps/edhari.json";       cine="-14,28,30, -14,28,30, 48,12,32, 1"; env=@{ VOXELFORGE_LOOK_VFOG="off" } },
  @{ set="godray"; tag="g1-vfog-default";         map="maps/edhari.json";       cine="-14,28,30, -14,28,30, 48,12,32, 1"; env=@{} }
)
if ($Only.Count -gt 0) { $shots = $shots | Where-Object { $Only -contains $_.set -or $Only -contains $_.tag } }

$LEVERS = @("VOXELFORGE_ATLAS_DIR","VOXELFORGE_WATER","VOXELFORGE_WATER_WAVE",
            "VOXELFORGE_WATER_DEPTH","VOXELFORGE_LOOK_VFOG")

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"   # VolumetricLight only exists at High/Ultra
$env:VOXELFORGE_CINE_START   = "1.0"

Write-Output ""
foreach ($s in $shots) {
  $png = Join-Path $Out ("{0}-nohud2.png" -f $s.tag)
  $log = Join-Path $Out ("{0}.log" -f $s.tag)
  Remove-Item $png -Force -ErrorAction SilentlyContinue

  # every lever is set or explicitly cleared on EVERY shot: a lever left over
  # from the previous frame is how an A/B turns into two of the same arm.
  foreach ($n in $LEVERS) {
    if ($s.env.ContainsKey($n)) { Set-Item -Path "Env:$n" -Value $s.env[$n] }
    else { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
  }
  $env:VOXELFORGE_MAP_LOAD = $s.map
  $env:VOXELFORGE_CINE     = $s.cine
  $env:VOXELFORGE_SHOT     = $png

  & $Exe --play *> $log
  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $note = ""; if ($size -lt 1) { $note = "  !!NO-FRAME  (read $log)" }
  $lev = ($s.env.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join " "
  if (-not $lev) { $lev = "(baked default, no lever)" }
  Write-Output ("{0,-26} ec={1}  {2,9} bytes  {3}{4}" -f $s.tag, $LASTEXITCODE, $size, $lev, $note)
}

foreach ($n in $LEVERS) { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
Write-Output ""
Write-Output "frames in $Out"
Write-Output "next:  python scripts/_pixel_artgap_grade.py $Out\tex-after-valleyridge-nohud2.png --json $Out\grade.json"
