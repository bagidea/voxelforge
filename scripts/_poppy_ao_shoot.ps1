# ===========================================================================
# BEFORE/AFTER plates for the voxel ambient-occlusion pass (riverbend).
#
# ONE binary, three plates. The A/B is the VOXELFORGE_AO lever, not two builds:
#   off  - every vertex fully lit. This is the "before" the pass is measured
#          against, and it is a real no-AO frame, not a weaker version of the
#          after (see voxel::ao_strength).
#   1    - the shipped strength (lever unset would do the same; it is passed
#          explicitly so the log line says which plate is which).
#   2    - diagnostic only. If this plate is pixel-identical to "1" then the
#          vertex attribute never reaches the shader, and no amount of tuning
#          the tables would have shown up in a frame.
#
# NO CARGO IS RUN HERE. Shoots with whatever exe is passed in (-Exe), and
# REFUSES an exe that has no VOXELFORGE_AO string in it - an exe built before
# the lever cannot produce an honest before/after out of one binary.
#
# USAGE  powershell -File scripts/_poppy_ao_shoot.ps1 [-Exe path] [-Out dir]
# ===========================================================================
param(
  [string]$Exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-poppy\perf\voxelforge.exe",
  [string]$Out = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_ao"
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

function Fail([string]$m) { Write-Output "REFUSED  $m"; exit 2 }
if (-not (Test-Path $Exe)) { Fail "$Exe not found" }

# ---- the exe must carry the lever AND the capture sweep -------------------
$bytes = [IO.File]::ReadAllBytes($Exe)
$text  = [Text.Encoding]::GetEncoding(28591).GetString($bytes)
foreach ($needle in @("VOXELFORGE_AO", "VOXELFORGE_AO_STATS", "VOXELFORGE_NOHUD",
                      "VOXELFORGE_CINE", "VOXELFORGE_SHOT", "VOXELFORGE_MAP_LOAD")) {
  $n = ([regex]::Matches($text, [regex]::Escape($needle))).Count
  Write-Output ("exe-has  {0,-24} {1}" -f $needle, $n)
  if ($n -lt 1) { Fail "$Exe has no $needle - it cannot shoot this A/B out of one binary" }
}
Write-Output ("exe      {0}" -f $Exe)
Write-Output ("         {0} bytes  mtime {1}" -f $bytes.Length, (Get-Item $Exe).LastWriteTime)
Write-Output ("         md5 {0}" -f (Get-FileHash $Exe -Algorithm MD5).Hash)
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# riverbend, exactly the camera scripts/_poppy_outdoor_plates.ps1 uses:
# eye and aim repeated so the camera is parked. SHOT fires at t>3.2s.
$map  = "maps/river_sunset.json"
$cine = "32,18,-12, 32,18,-12, 34,8,50, 1"

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
$env:VOXELFORGE_AO_STATS     = "1"
$env:VOXELFORGE_MAP_LOAD     = $map
$env:VOXELFORGE_CINE         = $cine
Remove-Item Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_ATLAS_MESH, Env:VOXELFORGE_LOOK_SUN, `
            Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE, `
            Env:VOXELFORGE_LOOK_AMBIENT, Env:VOXELFORGE_LOOK_FILL, `
            Env:VOXELFORGE_FLAT_MATERIAL, Env:VOXELFORGE_FLAT_INSTANCE, `
            Env:VOXELFORGE_SKY_GAIN, Env:VOXELFORGE_SKY_CURVE `
            -ErrorAction SilentlyContinue

$plates = @(
  @{ tag = "riverbend-BEFORE-ao-off"; ao = "off" },
  @{ tag = "riverbend-AFTER-ao-on";   ao = "1"   },
  @{ tag = "riverbend-DIAG-ao-2x";    ao = "2"   }
)

Write-Output ""
foreach ($p in $plates) {
  $png = Join-Path $Out ("{0}.png" -f $p.tag)
  $log = Join-Path $Out ("{0}.log" -f $p.tag)
  Remove-Item $png -Force -ErrorAction SilentlyContinue

  $env:VOXELFORGE_AO   = $p.ao
  $env:VOXELFORGE_SHOT = $png

  & $Exe --play *> $log
  $ec = $LASTEXITCODE

  # The log is written by a redirect, which on PS 5.1 lands as UTF-16LE.
  # Select-String decodes it; grep on the raw bytes never would.
  $err   = @(Select-String -Path $log -Pattern '^error').Count
  $cinen = @(Select-String -Path $log -Pattern '^CINE eye').Count
  $saved = @(Select-String -Path $log -Pattern '^SHOT saved to').Count
  $lever = @(Select-String -Path $log -Pattern '^AO strength').Count
  $stat  = @(Select-String -Path $log -Pattern '^AO_STATS').Count
  $lastst= (Select-String -Path $log -Pattern '^AO_STATS' | Select-Object -Last 1).Line
  $size  = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
  $md5   = if ($size -gt 0) { (Get-FileHash $png -Algorithm MD5).Hash } else { "-" }

  $note = ""
  if ($size -lt 1)   { $note += "  !!NO-FRAME" }
  if ($err -gt 0)    { $note += "  !!ERRORS=$err" }
  if ($cinen -lt 1)  { $note += "  !!CINE-IGNORED" }
  if ($saved -lt 1)  { $note += "  !!NO-SHOT-LINE" }
  if ($lever -lt 1)  { $note += "  !!LEVER-SILENT" }
  if ($stat -lt 1)   { $note += "  !!NO-AO-STATS" }
  Write-Output ("{0,-26} AO={1,-3} ec={2} {3,9} bytes  md5={4}{5}" -f `
    $p.tag, $p.ao, $ec, $size, $md5, $note)
  Write-Output ("    {0}" -f (Select-String -Path $log -Pattern '^AO strength' | Select-Object -First 1).Line)
  Write-Output ("    {0}" -f $lastst)
}
Write-Output ""
Write-Output "plates in $Out"
