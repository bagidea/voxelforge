# Rose weather-lane A/B shooter (2026-08-20). ASCII only - PS 5.1 trap.
# One exe, env-swap arms: BEFORE = no weather env at all, AFTER = the feature
# lever. Camera / map / framing identical across every arm, so each pair is a
# single-binary A/B like every other VOXELFORGE_* lane proof.
#
#   1-wet     : preset rain with the streaks OFF  -> wet ground only
#   2-rain    : preset rain                       -> streaks + ripples + wet
#   3-godrays : preset evening                    -> god-ray state
#   4-fog     : preset morning                    -> low fog + DistanceFog densify
#   fps-*     : 8s FPS_BENCH, baseline vs everything on (worst case)
param(
  [string]$Exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\_rose_weather_wt\target-rose\release\voxelforge.exe",
  [string]$Out = "E:\Projects\bagidea-ai-agents-office\workspace\projects\_rose_weather_wt\_rose_weather_shots",
  [string]$Map = "maps/beach_dusk.json",
  [string]$Cine = "41,15,37, 41,15,37, 29,3,20, 1"
)

$WEATHER_ARMS = @(
  "VOXELFORGE_WEATHER", "VOXELFORGE_WEATHER_RAIN", "VOXELFORGE_WEATHER_WET",
  "VOXELFORGE_WEATHER_GODRAYS", "VOXELFORGE_WEATHER_FOG",
  "VOXELFORGE_WEATHER_RAIN_COUNT"
)

function Clear-Weather {
  foreach ($n in $WEATHER_ARMS) { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }
}

function Shoot([string]$tag, [hashtable]$levers) {
  Clear-Weather
  foreach ($k in $levers.Keys) { Set-Item -Path "Env:$k" -Value $levers[$k] }
  $png = Join-Path $Out "$tag.png"
  $log = Join-Path $Out "$tag.log"
  Remove-Item $png, $log -Force -ErrorAction SilentlyContinue
  $env:VOXELFORGE_SHOT = $png
  & $Exe --play *> $log
  $ec = $LASTEXITCODE
  Clear-Weather
  Remove-Item "Env:VOXELFORGE_SHOT" -ErrorAction SilentlyContinue

  $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $err  = @(Select-String -Path $log -Pattern '^error').Count
  $wl   = @(Select-String -Path $log -Pattern '^WEATHER ').Count
  $note = ""
  if ($size -lt 1) { $note += "  !!NO-FRAME" }
  if ($err  -gt 0) { $note += "  !!ERRORS=$err" }
  if ($wl   -eq 0 -and $levers.Count -gt 0) { $note += "  !!WEATHER-SILENT" }
  if ($wl   -gt 0 -and $levers.Count -eq 0) { $note += "  !!WEATHER-LEAKED" }
  Write-Output ("{0,-22} ec={1}  {2,9} bytes  weather-lines={3}{4}" -f $tag, $ec, $size, $wl, $note)
}

function Bench([string]$tag, [hashtable]$levers) {
  Clear-Weather
  foreach ($k in $levers.Keys) { Set-Item -Path "Env:$k" -Value $levers[$k] }
  $log = Join-Path $Out "$tag.log"
  Remove-Item $log -Force -ErrorAction SilentlyContinue
  Remove-Item "Env:VOXELFORGE_SHOT" -ErrorAction SilentlyContinue
  $env:VOXELFORGE_FPS_BENCH = "8"
  & $Exe --play *> $log
  $ec = $LASTEXITCODE
  Remove-Item "Env:VOXELFORGE_FPS_BENCH" -ErrorAction SilentlyContinue
  Clear-Weather
  $line = (Select-String -Path $log -Pattern 'FPS_BENCH' | Select-Object -First 1).Line
  if (-not $line) { $line = "!!NO-BENCH-LINE" }
  Write-Output ("{0,-22} ec={1}  {2}" -f $tag, $ec, $line)
}

# Base env every arm shares: identical framing = honest A/B.
$env:VOXELFORGE_PLAY       = "1"
$env:VOXELFORGE_NOHUD      = "1"
$env:VOXELFORGE_MAP_LOAD   = $Map
$env:VOXELFORGE_CINE       = $Cine
$env:VOXELFORGE_CINE_START = "1.0"

New-Item -ItemType Directory -Force $Out | Out-Null
Write-Output "exe : $Exe"
Write-Output ("    {0} bytes  mtime {1}" -f (Get-Item $Exe).Length, (Get-Item $Exe).LastWriteTime)
Write-Output "out : $Out"
Write-Output ""

Shoot "1-wet-BEFORE"     @{}
Shoot "1-wet-AFTER"      @{ "VOXELFORGE_WEATHER" = "rain"; "VOXELFORGE_WEATHER_RAIN" = "off" }
Shoot "2-rain-BEFORE"    @{}
Shoot "2-rain-AFTER"     @{ "VOXELFORGE_WEATHER" = "rain" }
Shoot "2-rain-WIND"      @{ "VOXELFORGE_WEATHER" = "rain"; "VOXELFORGE_WEATHER_WIND" = "3.0,-1.5" }
Shoot "2-rain-SPARSE"    @{ "VOXELFORGE_WEATHER" = "rain"; "VOXELFORGE_WEATHER_RAIN_COUNT" = "300" }
Shoot "3-godrays-BEFORE" @{}
Shoot "3-godrays-AFTER"  @{ "VOXELFORGE_WEATHER" = "evening" }
Shoot "4-fog-BEFORE"     @{}
Shoot "4-fog-AFTER"      @{ "VOXELFORGE_WEATHER" = "morning" }

Write-Output ""
Bench "fps-BASELINE" @{}
Bench "fps-ALL-ON"   @{ "VOXELFORGE_WEATHER" = "all" }

Write-Output ""
Write-Output "done."
