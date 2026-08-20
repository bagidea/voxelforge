# ===========================================================================
# Flamingo - one-binary env ladder for the "kill the all-frame orange" order.
#
# NO CARGO IS RUN HERE. Every arm is the SAME exe with a different env, so any
# difference between two plates is the lever and nothing else. The arms that
# win here get BAKED into look.rs and re-shot with the env UNSET, because an
# env-override result is not a default (see the baked-defaults scar).
#
# ASCII ONLY on purpose: PS 5.1 mojibakes a UTF-8 em-dash into a curly quote it
# treats as a string delimiter, which has silently bricked a script in this repo
# before.
#
# USAGE
#   powershell -File scripts/_flamingo_huegap_ladder.ps1 -Exe <exe> -Out <dir>
#              [-Plates riverbend,shorehorizon] [-Only A0,A4]
# ===========================================================================
param(
  [string]$Exe    = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-pixel\release\voxelforge.exe",
  [string]$Out    = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_flamingo_huegap\ladder",
  [string[]]$Plates = @("riverbend","shorehorizon"),
  [string[]]$Only = @()
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

function Fail([string]$m) { Write-Output "REFUSED  $m"; exit 2 }
if (-not (Test-Path $Exe)) { Fail "$Exe not found" }

# ---- the exe must support the capture sweep AND carry this lane's levers ---
$bytes = [IO.File]::ReadAllBytes($Exe)
$text  = [Text.Encoding]::GetEncoding(28591).GetString($bytes)
foreach ($needle in @("VOXELFORGE_NOHUD","VOXELFORGE_CINE","VOXELFORGE_SHOT",
                      "VOXELFORGE_MAP_LOAD","VOXELFORGE_LOOK_FOGCOOL",
                      "VOXELFORGE_LOOK_GRADE","VOXELFORGE_LOOK_FOG",
                      "VOXELFORGE_LOOK_LIGHT","VOXELFORGE_SKY_GAIN",
                      "VOXELFORGE_CLOUDS","VOXELFORGE_LOOK_VFOG")) {
  $n = ([regex]::Matches($text, [regex]::Escape($needle))).Count
  if ($n -lt 1) { Fail "$Exe has no $needle - this ladder cannot be swept on it" }
}
Write-Output ("exe      {0}" -f $Exe)
Write-Output ("         {0} bytes  mtime {1}  md5 {2}" -f $bytes.Length, (Get-Item $Exe).LastWriteTime, (Get-FileHash $Exe -Algorithm MD5).Hash)
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# ---- cameras: the two plates the order names ------------------------------
$cams = @{
  riverbend    = @{ map = "maps/river_sunset.json"; cine = "32,18,-12, 32,18,-12, 34,8,50, 1" }
  shorehorizon = @{ map = "maps/beach_dusk.json";   cine = "8,14,-10, 8,14,-10, 40,5,44, 1" }
}

# ---- the ladder -----------------------------------------------------------
# Each arm is a hashtable of env overrides ON TOP of the clean baseline.
# GRADE is temperature,post_saturation,midtone_contrast,highlight_gain - the
# v3 shipped tuple is 0.07,1.02,1.10,1.06, so only field 1 moves in A1.
# LIGHT is key_rgb then ambient_rgb - the shipped key 1.00,0.92,0.62 is held.
#
# READ THE A-SERIES NOTES AS PRE-BAKE. On 2026-08-20 three of the values this
# ladder was sweeping were BAKED into look.rs as the new defaults:
#   HAZE_COOL     0.35 -> 0.72   (was an inline literal at the FOGCOOL read site)
#   HAZE_FULL     72   -> 240
#   AMBIENT_LUX_V3 620 -> 420
# so A0 "baseline, no env" is now the AFTER frame, not the before one. The
# B-series below restores the old values from the SAME binary and is therefore
# the BEFORE arm; B1/B2/B3 peel one constant off that restore at a time, so each
# constant's own contribution is separable instead of only the bundle.
$arms = @(
  @{ id="A0";  note="AFTER: baked defaults, no env";       env=@{} },

  # ---- the before/after pair, one binary ----------------------------------
  @{ id="B0";  note="BEFORE: all three restored";          env=@{ VOXELFORGE_LOOK_FOGCOOL="0.35"; VOXELFORGE_LOOK_FOG="20,72"; VOXELFORGE_LOOK_AMBIENT="620" } },
  @{ id="B1";  note="BEFORE but HAZE_COOL baked (0.72)";   env=@{ VOXELFORGE_LOOK_FOG="20,72"; VOXELFORGE_LOOK_AMBIENT="620" } },
  @{ id="B2";  note="BEFORE but HAZE_FULL baked (240)";    env=@{ VOXELFORGE_LOOK_FOGCOOL="0.35"; VOXELFORGE_LOOK_AMBIENT="620" } },
  @{ id="B3";  note="BEFORE but AMBIENT baked (420)";      env=@{ VOXELFORGE_LOOK_FOGCOOL="0.35"; VOXELFORGE_LOOK_FOG="20,72" } },

  # ---- the 2x2 that proves haze_color() feeds the CLOUD AMBIENT -----------
  # P0/P1 are B0/B1 with the deck off. The claim under test is that FOGCOOL
  # reaches the SKY only through `CloudKnobs::amb_warm = haze_color()`, i.e.
  # that the sky-region delta B0->B1 should COLLAPSE to the noise floor at
  # P0->P1. If the sky moves just as much with no deck in the frame, the claim
  # is wrong and the path is something else - that is the falsifying outcome.
  @{ id="P0";  note="2x2: FOGCOOL 0.35, deck OFF";         env=@{ VOXELFORGE_LOOK_FOGCOOL="0.35"; VOXELFORGE_LOOK_FOG="20,72"; VOXELFORGE_LOOK_AMBIENT="620"; VOXELFORGE_CLOUDS="off" } },
  @{ id="P1";  note="2x2: FOGCOOL 0.72, deck OFF";         env=@{ VOXELFORGE_LOOK_FOG="20,72"; VOXELFORGE_LOOK_AMBIENT="620"; VOXELFORGE_CLOUDS="off" } },

  # ---- N0/N1: same arm twice = this session's capture noise floor ---------
  # Nothing is compared against a delta smaller than what these two disagree by.
  @{ id="N0";  note="noise floor take 1 (== A0)";          env=@{} },
  @{ id="N1";  note="noise floor take 2 (== A0)";          env=@{} },

  @{ id="A1";  note="grade temperature 0.07 -> 0.00";      env=@{ VOXELFORGE_LOOK_GRADE="0.0,1.02,1.10,1.06" } },
  @{ id="A2";  note="haze FOGCOOL 0.35 -> 0.75";           env=@{ VOXELFORGE_LOOK_FOGCOOL="0.75" } },
  @{ id="A3";  note="haze ramp 20,72 -> 20,240";           env=@{ VOXELFORGE_LOOK_FOG="20,240" } },
  @{ id="D1";  note="diagnostic: cloud deck off";          env=@{ VOXELFORGE_CLOUDS="off" } },
  @{ id="D2";  note="diagnostic: play FogVolume off";      env=@{ VOXELFORGE_LOOK_VFOG="off" } },
  @{ id="D3";  note="diagnostic: ambient flat term 620->120"; env=@{ VOXELFORGE_LOOK_AMBIENT="120" } },
  @{ id="A4";  note="A1+A2+A3";                            env=@{ VOXELFORGE_LOOK_GRADE="0.0,1.02,1.10,1.06"; VOXELFORGE_LOOK_FOGCOOL="0.75"; VOXELFORGE_LOOK_FOG="20,240" } },
  @{ id="A5";  note="A4 + ambient hue cool (lux held)";    env=@{ VOXELFORGE_LOOK_GRADE="0.0,1.02,1.10,1.06"; VOXELFORGE_LOOK_FOGCOOL="0.75"; VOXELFORGE_LOOK_FOG="20,240"; VOXELFORGE_LOOK_LIGHT="1.00,0.92,0.62,0.78,0.86,1.00" } },
  @{ id="A6";  note="A5 + ambient lux 620 -> 420";         env=@{ VOXELFORGE_LOOK_GRADE="0.0,1.02,1.10,1.06"; VOXELFORGE_LOOK_FOGCOOL="0.75"; VOXELFORGE_LOOK_FOG="20,240"; VOXELFORGE_LOOK_LIGHT="1.00,0.92,0.62,0.78,0.86,1.00"; VOXELFORGE_LOOK_AMBIENT="420" } },
  @{ id="A7";  note="A6 + sky paint gain 4 -> 7";          env=@{ VOXELFORGE_LOOK_GRADE="0.0,1.02,1.10,1.06"; VOXELFORGE_LOOK_FOGCOOL="0.75"; VOXELFORGE_LOOK_FOG="20,240"; VOXELFORGE_LOOK_LIGHT="1.00,0.92,0.62,0.78,0.86,1.00"; VOXELFORGE_LOOK_AMBIENT="420"; VOXELFORGE_SKY_GAIN="7.0" } },
  @{ id="A8";  note="A7 + cloud deck off";                 env=@{ VOXELFORGE_LOOK_GRADE="0.0,1.02,1.10,1.06"; VOXELFORGE_LOOK_FOGCOOL="0.75"; VOXELFORGE_LOOK_FOG="20,240"; VOXELFORGE_LOOK_LIGHT="1.00,0.92,0.62,0.78,0.86,1.00"; VOXELFORGE_LOOK_AMBIENT="420"; VOXELFORGE_SKY_GAIN="7.0"; VOXELFORGE_CLOUDS="off" } },
  @{ id="A9";  note="A7 + play FogVolume 0.030 -> 0.012";  env=@{ VOXELFORGE_LOOK_GRADE="0.0,1.02,1.10,1.06"; VOXELFORGE_LOOK_FOGCOOL="0.75"; VOXELFORGE_LOOK_FOG="20,240"; VOXELFORGE_LOOK_LIGHT="1.00,0.92,0.62,0.78,0.86,1.00"; VOXELFORGE_LOOK_AMBIENT="420"; VOXELFORGE_SKY_GAIN="7.0"; VOXELFORGE_LOOK_VFOG="0.012" } },
  @{ id="A10"; note="A9 + cloud HDR gain 3.2 -> 6.0";      env=@{ VOXELFORGE_LOOK_GRADE="0.0,1.02,1.10,1.06"; VOXELFORGE_LOOK_FOGCOOL="0.75"; VOXELFORGE_LOOK_FOG="20,240"; VOXELFORGE_LOOK_LIGHT="1.00,0.92,0.62,0.78,0.86,1.00"; VOXELFORGE_LOOK_AMBIENT="420"; VOXELFORGE_SKY_GAIN="7.0"; VOXELFORGE_LOOK_VFOG="0.012"; VOXELFORGE_CLOUDS_GAIN="6.0" } },
  @{ id="A11"; note="A10 + cloud cover 0.55 -> 0.32";      env=@{ VOXELFORGE_LOOK_GRADE="0.0,1.02,1.10,1.06"; VOXELFORGE_LOOK_FOGCOOL="0.75"; VOXELFORGE_LOOK_FOG="20,240"; VOXELFORGE_LOOK_LIGHT="1.00,0.92,0.62,0.78,0.86,1.00"; VOXELFORGE_LOOK_AMBIENT="420"; VOXELFORGE_SKY_GAIN="7.0"; VOXELFORGE_LOOK_VFOG="0.012"; VOXELFORGE_CLOUDS_GAIN="6.0"; VOXELFORGE_CLOUDS_COVER="0.32" } }
)
if ($Only.Count -gt 0) { $arms = $arms | Where-Object { $Only -contains $_.id } }

# every lever this ladder touches, so an arm always starts from a clean slate
$sweepKeys = @("VOXELFORGE_LOOK_GRADE","VOXELFORGE_LOOK_FOGCOOL","VOXELFORGE_LOOK_FOG",
               "VOXELFORGE_LOOK_LIGHT","VOXELFORGE_LOOK_AMBIENT","VOXELFORGE_SKY_GAIN",
               "VOXELFORGE_CLOUDS","VOXELFORGE_LOOK_VFOG","VOXELFORGE_LOOK_HAZE",
               "VOXELFORGE_LOOK_HAZECOL","VOXELFORGE_LOOK_HAZEDESAT","VOXELFORGE_SKY_CURVE",
               "VOXELFORGE_LOOK_SUN","VOXELFORGE_LOOK_EXPOSURE","VOXELFORGE_LOOK_FILL",
               "VOXELFORGE_LOOK_IBL","VOXELFORGE_ATLAS_DIR",
               "VOXELFORGE_CLOUDS_GAIN","VOXELFORGE_CLOUDS_COVER")

$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"

Write-Output ""
foreach ($arm in $arms) {
  foreach ($k in $sweepKeys) { Remove-Item ("Env:" + $k) -ErrorAction SilentlyContinue }
  foreach ($k in $arm.env.Keys) { Set-Item -Path ("Env:" + $k) -Value $arm.env[$k] }

  foreach ($tag in $Plates) {
    $c = $cams[$tag]
    if ($null -eq $c) { Write-Output "SKIP unknown plate $tag"; continue }
    $png = Join-Path $Out ("{0}-{1}.png" -f $tag, $arm.id)
    $log = Join-Path $Out ("{0}-{1}.log" -f $tag, $arm.id)
    Remove-Item $png -Force -ErrorAction SilentlyContinue

    $env:VOXELFORGE_MAP_LOAD = $c.map
    $env:VOXELFORGE_CINE     = $c.cine
    $env:VOXELFORGE_SHOT     = $png

    & $Exe --play *> $log
    $ec = $LASTEXITCODE
    $size = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
    $note = ""
    if ($size -lt 1) { $note = "  !!NO-FRAME" }
    Write-Output ("{0,-4} {1,-13} ec={2} {3,9} bytes   {4}{5}" -f $arm.id, $tag, $ec, $size, $arm.note, $note)
  }
}
foreach ($k in $sweepKeys) { Remove-Item ("Env:" + $k) -ErrorAction SilentlyContinue }
Write-Output ""
Write-Output "ladder in $Out"
