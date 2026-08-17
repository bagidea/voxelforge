# =============================================================================
# Poppy -- exe-to-exe look A/B, 2026-08-18.
#
# WHY THIS IS NOT THE EXE PAIR THE BRIEF NAMED. The brief asked for
# before=`target-poppy\release\voxelforge.exe` (Aug 5) vs
# after=`target-poppy\release\voxelforge_shot.exe` (01:53 today). Both halves of
# that pair are unusable as a like-for-like look A/B, and the exes say so:
#
#   * voxelforge_shot.exe is the ISOLATED HERO-SHOT bin (`shot_main.rs`). It
#     declares hero/characters/equipment/enemies/vfx/block_atlas and NOT
#     `voxel.rs` or `look.rs`, so it cannot render the world scene at all and
#     contains none of the look levers (scan: VOXELFORGE_CINE 0, NOHUD 0,
#     LOOK_SUN 0). It renders a hero rig, not the game.
#   * the Aug 5 voxelforge.exe predates the in-engine still rig entirely
#     (scan: VOXELFORGE_CINE 0, VOXELFORGE_NOHUD 0). The camera cannot be posed
#     and the HUD cannot be removed, so "same angle" is not achievable with it.
#
# So the pair below is the honest one: the two WORLD binaries that both honour
# the same camera contract, straddling the look-v4 render path.
#
#   BEFORE  target-flamingo\release\voxelforge.exe  (Aug 8)  VOXELFORGE_LOOK_GEN absent
#   AFTER   target\release\voxelforge.exe           (Aug 18 00:18) LOOK_GEN present
#
# The straddle is proven by a string scan for the lever the change introduced,
# not by mtime or commit order.
#
# Both exes are COPIED into a staging dir before a single frame is shot: the
# `target\` dir is the shared default and a teammate is mid-build into it, so
# shooting from it directly risks the binary being replaced halfway through the
# pair. md5 of each staged copy goes in the log.
#
# Usage: _poppy_ab_20260818_shoot.ps1
# =============================================================================
$ErrorActionPreference = "Stop"
$Root  = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Out   = Join-Path $Root "docs\look-ab-poppy-2026-08-18"
$Stage = Join-Path $Root "_poppy_ab_stage"
$TimeoutSec = 180

foreach ($d in @($Out, $Stage)) { if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null } }

$log = Join-Path $Out "_shoot.log"
if (Test-Path $log) { Remove-Item $log }
function Say($m) { $m | Tee-Object -FilePath $log -Append }

Say "REV  $(git -C $Root rev-parse --short HEAD)  branch $(git -C $Root branch --show-current)"

# ---- stage the two binaries, pinned by md5 ----------------------------------
$srcBefore = Join-Path $Root "target-flamingo\release\voxelforge.exe"
$srcAfter  = Join-Path $Root "target\release\voxelforge.exe"
$exeBefore = Join-Path $Stage "before_voxelforge.exe"
$exeAfter  = Join-Path $Stage "after_voxelforge.exe"

foreach ($pair in @(@($srcBefore,$exeBefore,"BEFORE"), @($srcAfter,$exeAfter,"AFTER"))) {
    $src, $dst, $tag = $pair
    if (-not (Test-Path $src)) { throw "no exe at $src" }
    Copy-Item $src $dst -Force
    $i = Get-Item $dst
    $h = (Get-FileHash $dst -Algorithm MD5).Hash
    Say "$tag  src=$($src.Replace($Root,''))"
    Say "      staged $($i.Length) bytes  srcmtime $((Get-Item $src).LastWriteTime.ToString('s'))  md5 $h"
}

# ---- prove the straddle on the staged copies (not on mtime) -----------------
Say ""
Say "--- straddle scan (marker that the look-v4 path introduced) ---"
foreach ($pair in @(@($exeBefore,"BEFORE"), @($exeAfter,"AFTER"))) {
    $exe, $tag = $pair
    $t = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
    $line = "$tag "
    foreach ($m in @("VOXELFORGE_LOOK_GEN","VOXELFORGE_CINE","VOXELFORGE_NOHUD","VOXELFORGE_LOOK_SUN")) {
        $line += "  $m=$((([regex]::Matches($t,[regex]::Escape($m)))).Count)"
    }
    Say $line
}
Say ""

# ---- shared render settings -------------------------------------------------
$env:VOXELFORGE_PLAY          = "1"
$env:VOXELFORGE_NOHUD         = "1"
$env:VOXELFORGE_LOOK_QUALITY  = "ultra"
$env:VOXELFORGE_CINE_START    = "1.0"

# Scenes. Camera/sun/light for outdoor-noon are VERBATIM from
# `_poppy_pbr_shoot.ps1`, which took them from `_poppy_lookv3_shoot.cmd`, so
# these plates stay like-for-like with docs/assets/look/outdoor-noon_*.png.
$Scenes = @(
    @{ name = "outdoor-noon"
       cine = "44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
       sun  = "66,205,20000"
       light= "1.00,0.98,0.93,0.84,0.88,1.00"
       ev   = "10.6" },
    @{ name = "village-raking"
       cine = "44,14,44, 44,14,44, 26.0,6.0,34.0, 1"
       sun  = "22,118,16000"
       light= "1.00,0.94,0.84,0.80,0.86,1.00"
       ev   = "10.2" }
)

function Invoke-Shot {
    param([string]$Exe, [string]$Tag, [hashtable]$Scene)

    $env:VOXELFORGE_CINE          = $Scene.cine
    $env:VOXELFORGE_LOOK_SUN      = $Scene.sun
    $env:VOXELFORGE_LOOK_LIGHT    = $Scene.light
    $env:VOXELFORGE_LOOK_EXPOSURE = $Scene.ev

    $shot = Join-Path $Out ($Tag + "_" + $Scene.name + ".png")
    if (Test-Path $shot) { Remove-Item $shot -Force }
    $env:VOXELFORGE_SHOT = $shot

    $runLog = Join-Path $Out ("_run_" + $Tag + "_" + $Scene.name + ".log")
    $errLog = Join-Path $Out ("_run_" + $Tag + "_" + $Scene.name + ".err.log")
    Say "[shoot] $Tag / $($Scene.name)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process -FilePath $Exe -ArgumentList "--play" -PassThru -NoNewWindow -RedirectStandardOutput $runLog -RedirectStandardError $errLog
    if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
        Say "         !! TIMEOUT after $TimeoutSec s -- killing"
        try { $proc.Kill() } catch {}
        Start-Sleep -Milliseconds 500
    }
    $sw.Stop()

    if (Test-Path $shot) {
        $i = Get-Item $shot
        Say ("         wrote {0} bytes  md5 {1}  in {2:N1}s" -f $i.Length,
             (Get-FileHash $shot -Algorithm MD5).Hash, $sw.Elapsed.TotalSeconds)
    } else {
        Say "         !! NO FILE WRITTEN (after $([int]$sw.Elapsed.TotalSeconds)s)"
    }
}

foreach ($s in $Scenes) {
    Invoke-Shot -Exe $exeBefore -Tag "before" -Scene $s
    Invoke-Shot -Exe $exeAfter  -Tag "after"  -Scene $s
}

Say ""
Say "DONE"
Get-ChildItem $Out -Filter "*.png" | ForEach-Object { Say ("  {0,-34} {1,9} bytes" -f $_.Name, $_.Length) }
