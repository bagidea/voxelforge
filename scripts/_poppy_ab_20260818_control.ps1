# =============================================================================
# Poppy -- ONE-BINARY control for the 2026-08-18 exe-to-exe A/B.
#
# WHY THIS EXISTS. The exe-to-exe pair (before_*.png / after_*.png) straddles ten
# days of EVERY lane's work, so it cannot attribute anything it shows to the look
# lane. The after plate lost its sky (black) and went warm; that has to be pinned
# on a variable before it is reported as a look result.
#
# This shoots the SAME after-binary twice at the SAME camera, moving only
# VOXELFORGE_LOOK_GEN (v3 = the surface rig that shipped 2026-08-17, v4 = that
# plus the occlusion map + exposure pass). Both halves read that one variable, so
# whatever differs between these two frames IS the look generation -- and
# whatever is IDENTICAL in both (a black sky, say) is NOT.
#
# Camera/sun/light/exposure are copied verbatim from the outdoor-noon scene in
# `_poppy_ab_20260818_shoot.ps1`, so this control is directly comparable to the
# `after_outdoor-noon.png` plate.
#
# Runs the exe via Start-Process with real file redirects: `& exe *>> log` under
# ErrorActionPreference=Stop aborts on the engine's first stderr line in
# PowerShell 5.1 (NativeCommandError), which is what killed the earlier attempt.
# =============================================================================
$ErrorActionPreference = "Stop"
$Root  = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Out   = Join-Path $Root "docs\look-ab-poppy-2026-08-18\control-onebinary"
$Exe   = Join-Path $Root "_poppy_ab_stage\after_voxelforge.exe"
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

# ---- shared: identical to the outdoor-noon scene in the main shoot ----------
$env:VOXELFORGE_PLAY          = "1"
$env:VOXELFORGE_NOHUD         = "1"
$env:VOXELFORGE_LOOK_QUALITY  = "ultra"
$env:VOXELFORGE_CINE_START    = "1.0"
$env:VOXELFORGE_CINE          = "44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
$env:VOXELFORGE_LOOK_SUN      = "66,205,20000"
$env:VOXELFORGE_LOOK_LIGHT    = "1.00,0.98,0.93,0.84,0.88,1.00"
$env:VOXELFORGE_LOOK_EXPOSURE = "10.6"

function Invoke-Gen {
    param([string]$Gen)

    $env:VOXELFORGE_LOOK_GEN = $Gen
    $shot = Join-Path $Out ("outdoor-noon_gen-" + $Gen + ".png")
    if (Test-Path $shot) { Remove-Item $shot -Force }
    $env:VOXELFORGE_SHOT = $shot

    $runLog = Join-Path $Out ("_run_gen-" + $Gen + ".log")
    $errLog = Join-Path $Out ("_run_gen-" + $Gen + ".err.log")
    Say "[shoot] LOOK_GEN=$Gen"
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
}

Invoke-Gen -Gen "v3"
Invoke-Gen -Gen "v4"

Say ""
Say "DONE"
Get-ChildItem $Out -Filter "*.png" | ForEach-Object { Say ("  {0,-30} {1,9} bytes" -f $_.Name, $_.Length) }
