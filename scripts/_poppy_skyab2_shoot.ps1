# ===========================================================================
# Sky A/B, 4 plates x 2 binaries, ZERO levers.
#
# The pair comes out of scripts/_poppy_skyab2_build.cmd: ONE clean worktree
# (_poppy_skyab_wt, detached at 676e2c5), ONE target dir (target-poppysky), the
# same toolchain, minutes apart. The ONLY difference between the two exes is
# client/src/look.rs -- tip vs 0e54d85 (e4ceb80's parent), i.e. the sky patch.
#
# Everything else is held still on purpose:
#   - cwd is the worktree, so assets/ + maps/ are the COMMITTED 676e2c5 tree for
#     both arms (the main working tree carries other lanes' uncommitted texture
#     edits -- those must not leak into a look A/B)
#   - the four plates are the SAME map + cine cameras grade_final.json graded
#   - every VOXELFORGE_LOOK_* / SKY_* lever is unset before either arm runs
#
# The run REFUSES rather than shooting a meaningless pair when:
#   - either exe is missing or is not a world-render exe
#   - BEFORE contains VOXELFORGE_LOOK_FOGCOOL (a string only e4ceb80 introduces)
#   - AFTER does not contain it
# i.e. the straddle is proven by scanning the BYTES of each exe, never by mtime
# or by commit order (2026-08-14 scar).
#
# USAGE  powershell -File scripts/_poppy_skyab2_shoot.ps1
# ===========================================================================
param(
  [string]$Root      = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge",
  [string]$BeforeExe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_skyab2\voxelforge_SKY_BEFORE_676e2c5-minus-e4ceb80.exe",
  [string]$AfterExe  = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_skyab2\voxelforge_SKY_AFTER_676e2c5.exe",
  [string]$Wt        = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_skyab_wt",
  [string]$Out       = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_skyab2"
)
$ErrorActionPreference = "Continue"

function Fail([string]$m) { Write-Output "REFUSED  $m"; exit 2 }
foreach ($p in @($BeforeExe, $AfterExe)) { if (-not (Test-Path $p)) { Fail "$p not found" } }
if (-not (Test-Path (Join-Path $Wt "assets"))) { Fail "$Wt has no assets/" }
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# ---- straddle proof: scan the bytes ---------------------------------------
function Count-Ascii([string]$file, [string]$needle) {
  $bytes = [IO.File]::ReadAllBytes($file)
  $text  = [Text.Encoding]::GetEncoding(28591).GetString($bytes)
  return ([regex]::Matches($text, [regex]::Escape($needle))).Count
}
foreach ($arm in @(@{t="BEFORE";e=$BeforeExe}, @{t="AFTER";e=$AfterExe})) {
  $i = Get-Item $arm.e
  Write-Output ("exe      {0,-6} {1,10} bytes  mtime {2:yyyy-MM-dd HH:mm:ss}  md5 {3}" -f `
    $arm.t, $i.Length, $i.LastWriteTime, (Get-FileHash $arm.e -Algorithm MD5).Hash)
}
$bFog = Count-Ascii $BeforeExe "VOXELFORGE_LOOK_FOGCOOL"
$aFog = Count-Ascii $AfterExe  "VOXELFORGE_LOOK_FOGCOOL"
$bSun = Count-Ascii $BeforeExe "VOXELFORGE_LOOK_SUN"
$aSun = Count-Ascii $AfterExe  "VOXELFORGE_LOOK_SUN"
Write-Output ("straddle BEFORE FOGCOOL={0} SUN={1}   AFTER FOGCOOL={2} SUN={3}" -f $bFog,$bSun,$aFog,$aSun)
if ($bSun -lt 1 -or $aSun -lt 1) { Fail "one of these is not a world-render exe (LOOK_SUN missing)" }
if ($bFog -ne 0)  { Fail "BEFORE contains FOGCOOL - the sky patch is IN it, the pair does not straddle" }
if ($aFog -lt 1)  { Fail "AFTER has no FOGCOOL - the sky patch is NOT in it, the pair does not straddle" }
foreach ($needle in @("VOXELFORGE_NOHUD", "VOXELFORGE_CINE", "VOXELFORGE_SHOT", "VOXELFORGE_MAP_LOAD")) {
  if ((Count-Ascii $BeforeExe $needle) -lt 1 -or (Count-Ascii $AfterExe $needle) -lt 1) {
    Fail "an arm has no $needle - it cannot produce a HUD-free scripted plate"
  }
}

# ---- the four graded plates, verbatim from _poppy_outdoor_plates.ps1 -------
$plates = @(
  @{ tag = "highland";     map = "maps/edhari.json";       cine = "-14,28,30, -14,28,30, 48,12,32, 1" },
  @{ tag = "riverbend";    map = "maps/river_sunset.json"; cine = "32,18,-12, 32,18,-12, 34,8,50, 1" },
  @{ tag = "shorehorizon"; map = "maps/beach_dusk.json";   cine = "8,14,-10, 8,14,-10, 40,5,44, 1" },
  @{ tag = "valleyridge";  map = "maps/river_sunset.json"; cine = "-16,26,32, -16,26,32, 46,20,32, 1" }
)

Set-Location $Wt
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "1.0"
Remove-Item Env:VOXELFORGE_ATLAS_DIR, Env:VOXELFORGE_LOOK_SUN, `
            Env:VOXELFORGE_LOOK_LIGHT, Env:VOXELFORGE_LOOK_EXPOSURE, `
            Env:VOXELFORGE_LOOK_AMBIENT, Env:VOXELFORGE_LOOK_FILL, `
            Env:VOXELFORGE_LOOK_FOGCOOL, Env:VOXELFORGE_LOOK_SKYGRAD, `
            Env:VOXELFORGE_LOOK_SKYGAIN, `
            Env:VOXELFORGE_SKY_GAIN, Env:VOXELFORGE_SKY_CURVE `
            -ErrorAction SilentlyContinue

$rows = @()
Write-Output ""
foreach ($p in $plates) {
  foreach ($arm in @(@{t="before";e=$BeforeExe}, @{t="after";e=$AfterExe})) {
    $png = Join-Path $Out ("{0}-{1}.png" -f $p.tag, $arm.t)
    $log = Join-Path $Out ("{0}-{1}.log" -f $p.tag, $arm.t)
    Remove-Item $png -Force -ErrorAction SilentlyContinue

    $env:VOXELFORGE_MAP_LOAD = $p.map
    $env:VOXELFORGE_CINE     = $p.cine
    $env:VOXELFORGE_SHOT     = $png

    & $arm.e --play *> $log
    $ec = $LASTEXITCODE

    $err   = @(Select-String -Path $log -Pattern '^error').Count
    $cine  = @(Select-String -Path $log -Pattern '^CINE eye').Count
    $saved = @(Select-String -Path $log -Pattern '^SHOT saved to').Count
    $size  = if (Test-Path $png) { (Get-Item $png).Length } else { 0 }
    $md5   = if ($size -gt 0) { (Get-FileHash $png -Algorithm MD5).Hash } else { "-" }

    $note = ""
    if ($size -lt 1)  { $note += "  !!NO-FRAME" }
    if ($err -gt 0)   { $note += "  !!ERRORS=$err" }
    if ($cine -lt 1)  { $note += "  !!CINE-IGNORED" }
    if ($saved -lt 1) { $note += "  !!NO-SHOT-LINE" }
    Write-Output ("{0,-13} {1,-6} ec={2} {3,9} bytes  cine={4} shot={5}  md5 {6}{7}" -f `
      $p.tag, $arm.t, $ec, $size, $cine, $saved, $md5, $note)
    $rows += [pscustomobject]@{ tag = $p.tag; arm = $arm.t; md5 = $md5; size = $size }
  }
}

Write-Output ""
$bad = 0
foreach ($p in $plates) {
  $b = ($rows | Where-Object { $_.tag -eq $p.tag -and $_.arm -eq "before" }).md5
  $a = ($rows | Where-Object { $_.tag -eq $p.tag -and $_.arm -eq "after"  }).md5
  if ($b -eq "-" -or $a -eq "-") { Write-Output ("pair {0,-13} UNMEASURABLE - an arm produced no frame" -f $p.tag); $bad++ }
  elseif ($b -eq $a)             { Write-Output ("pair {0,-13} SUSPECT - byte-identical PNGs" -f $p.tag); $bad++ }
  else                           { Write-Output ("pair {0,-13} ok - two distinct frames" -f $p.tag) }
}
Write-Output ""
if ($bad -gt 0) { Write-Output "VERDICT  $bad of 4 pairs unusable"; exit 3 }
Write-Output "VERDICT  4/4 pairs OK - distinct frames from two straddling binaries"
Write-Output "frames in $Out"
