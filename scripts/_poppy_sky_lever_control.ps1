# =============================================================================
# Poppy -- ONE-BINARY control for the BLACK SKY on the 2026-08-18 after-plates.
#
# WHY THIS EXISTS. The first report on those plates guessed the black sky was a
# mesh spawned without `Visibility` and handed it to scene.rs. That guess is
# contradicted by the binary's own log line, `LOOK atmosphere spawned mode=lut`,
# and by look.rs:3474-3484: "atmosphere on => dome off, unconditionally /
# Default: atmosphere on, dome off". The dome is OFF ON PURPOSE in that build.
# The LOOK_GEN v3/v4 control that was run does not touch the sky path at all, so
# it proved nothing about this.
#
# The separating lever is already inside the after-binary. Three arms, one exe,
# one camera:
#
#   A  atmos       (no env)                    atmosphere LUT draws the sky, dome off
#   B  dome        LOOK_ATMOS=off              atmosphere off => gradient dome comes back
#   C  flat        LOOK_ATMOS=off SKYGRAD=off  both off => flat ClearColor sky
#
# Reading it:
#   A black, B blue   -> the ATMOSPHERE is what renders black. Not a dome bug,
#                        not a Visibility bug, and the look lane owns it.
#   A black, B black,
#   C blue            -> the DOME is dead too; a Visibility/spawn bug is back on
#                        the table and scene.rs/sky is the right place to hand it.
#   all three black   -> nothing sky-side is drawing; camera/exposure, not sky.
#
# ASSET ROOT. main.rs:517 pins the asset root to `<exe dir>/assets`. The earlier
# plates were shot from a staging dir that held nothing but two .exe files, so
# every AssetServer load 404'd (visible in their .err.log). This script stages a
# junction to the repo `assets/` next to the exe before the first frame and then
# asserts the run logged zero "Path not found" lines.
#
# Camera/sun/light/exposure are copied verbatim from the outdoor-noon scene in
# `_poppy_ab_20260818_shoot.ps1`, so these arms are directly comparable to
# `after_outdoor-noon.png` and to `control-onebinary/`.
#
# ASCII only, on purpose: a UTF-8 em dash mojibakes into a quote character that
# PowerShell 5.1 treats as a string delimiter, which silently bricks the file.
# =============================================================================
$ErrorActionPreference = "Stop"
$Root  = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Out   = Join-Path $Root "docs\look-ab-poppy-2026-08-18\control-sky"
$Stage = Join-Path $Root "_poppy_ab_stage"
$Exe   = Join-Path $Stage "after_voxelforge.exe"
$TimeoutSec = 240

if (-not (Test-Path $Exe)) { throw "no staged exe at $Exe" }
if (-not (Test-Path $Out)) { New-Item -ItemType Directory -Path $Out -Force | Out-Null }

$log = Join-Path $Out "_control.log"
if (Test-Path $log) { Remove-Item $log }
function Say($m) { $m | Tee-Object -FilePath $log -Append }

$i = Get-Item $Exe
Say "EXE  $Exe"
Say "     $($i.Length) bytes  mtime $($i.LastWriteTime.ToString('s'))  md5 $((Get-FileHash $Exe -Algorithm MD5).Hash)"
Say "REV  $(git -C $Root rev-parse --short HEAD)  branch $(git -C $Root branch --show-current)"
Say ""

# ---- gate: the lever has to exist INSIDE this exe, or the arms are theatre ---
$bytes = [System.IO.File]::ReadAllBytes($Exe)
$ascii = [System.Text.Encoding]::ASCII.GetString($bytes)
foreach ($needle in @("VOXELFORGE_LOOK_ATMOS", "VOXELFORGE_LOOK_SKYGRAD", "VOXELFORGE_CINE")) {
    $n = ([regex]::Matches($ascii, [regex]::Escape($needle))).Count
    Say ("GATE  {0,-26} {1} occurrence(s)" -f $needle, $n)
    if ($n -lt 1) { throw "GATE FAILED: $needle is not in this binary -- the arm would be a no-op" }
}
$bytes = $null; $ascii = $null
Say ""

# ---- stage the asset root next to the exe (main.rs:517) ---------------------
$stagedAssets = Join-Path $Stage "assets"
if (-not (Test-Path $stagedAssets)) {
    New-Item -ItemType Junction -Path $stagedAssets -Target (Join-Path $Root "assets") | Out-Null
    Say "ASSETS staged junction $stagedAssets -> $(Join-Path $Root 'assets')"
} else {
    Say "ASSETS already staged at $stagedAssets"
}
if (-not (Test-Path (Join-Path $stagedAssets "textures\blocks\atlas.json"))) {
    throw "asset root staged but atlas.json is not reachable through it"
}
Say ""

# ---- shared: identical to the outdoor-noon scene in the main shoot ----------
$env:VOXELFORGE_PLAY          = "1"
$env:VOXELFORGE_NOHUD         = "1"
$env:VOXELFORGE_LOOK_QUALITY  = "ultra"
$env:VOXELFORGE_CINE_START    = "1.0"
$env:VOXELFORGE_CINE          = "44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
$env:VOXELFORGE_LOOK_SUN      = "66,205,20000"
$env:VOXELFORGE_LOOK_LIGHT    = "1.00,0.98,0.93,0.84,0.88,1.00"
$env:VOXELFORGE_LOOK_EXPOSURE = "10.6"

function Invoke-Arm {
    param([string]$Name, [string]$Atmos, [string]$Skygrad)

    # Explicit clear, never "leave it from the last arm": an inherited env var is
    # exactly how a three-arm control quietly becomes a one-arm control.
    if ($Atmos)   { $env:VOXELFORGE_LOOK_ATMOS = $Atmos }     else { Remove-Item Env:\VOXELFORGE_LOOK_ATMOS -ErrorAction SilentlyContinue }
    if ($Skygrad) { $env:VOXELFORGE_LOOK_SKYGRAD = $Skygrad } else { Remove-Item Env:\VOXELFORGE_LOOK_SKYGRAD -ErrorAction SilentlyContinue }

    $shot = Join-Path $Out ("outdoor-noon_sky-" + $Name + ".png")
    if (Test-Path $shot) { Remove-Item $shot -Force }
    $env:VOXELFORGE_SHOT = $shot

    $runLog = Join-Path $Out ("_run_sky-" + $Name + ".log")
    $errLog = Join-Path $Out ("_run_sky-" + $Name + ".err.log")
    Say ("[shoot] {0}  ATMOS={1} SKYGRAD={2}" -f $Name, $(if ($Atmos) { $Atmos } else { "<unset>" }), $(if ($Skygrad) { $Skygrad } else { "<unset>" }))
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process -FilePath $Exe -ArgumentList "--play" -PassThru -NoNewWindow -RedirectStandardOutput $runLog -RedirectStandardError $errLog
    if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
        Say "         !! TIMEOUT after $TimeoutSec s -- killing"
        try { $proc.Kill() } catch {}
        Start-Sleep -Milliseconds 500
    }
    $sw.Stop()

    if (Test-Path $shot) {
        $f = Get-Item $shot
        Say ("         wrote {0} bytes  md5 {1}  in {2:N1}s" -f $f.Length, (Get-FileHash $shot -Algorithm MD5).Hash, $sw.Elapsed.TotalSeconds)
    } else {
        Say "         !! NO FILE WRITTEN"
    }

    # what the engine itself says about the sky this arm, quoted not paraphrased
    foreach ($line in (Select-String -Path $runLog -Pattern "LOOK atmosphere|SKY dome|BLOCK_ART|LOOK_IBL" -ErrorAction SilentlyContinue)) {
        Say ("         | " + $line.Line.Trim())
    }
    $missing = @(Select-String -Path $errLog -Pattern "Path not found" -ErrorAction SilentlyContinue).Count
    Say ("         asset 404s: {0}" -f $missing)
}

Invoke-Arm -Name "atmos" -Atmos $null  -Skygrad $null
Invoke-Arm -Name "dome"  -Atmos "off"  -Skygrad $null
Invoke-Arm -Name "flat"  -Atmos "off"  -Skygrad "off"

Say ""
Say "DONE"
Get-ChildItem $Out -Filter "*.png" | ForEach-Object { Say ("  {0,-34} {1,9} bytes  md5 {2}" -f $_.Name, $_.Length, (Get-FileHash $_.FullName -Algorithm MD5).Hash) }
