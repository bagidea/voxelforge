# =============================================================================
# Poppy -- A3 cloud deck: the one-binary before/after, 2026-08-18.
#
# ONE EXE, ONE SCENE, ONE CAMERA, ONE ENV VAR APART. The pair is
#   before  VOXELFORGE_CLOUDS=off   (the sky this game shipped: atmosphere only)
#   after   VOXELFORGE_CLOUDS unset (deck + enlarged solar disk)
# both fired from the SAME staged copy of target-poppy\release\voxelforge.exe,
# with identical VOXELFORGE_CINE / _LOOK_SUN / _LOOK_EXPOSURE. That is the only
# shape of evidence this lane accepts, because an exe-to-exe pair cannot separate
# the change from everything else that landed between two builds.
#
# A NULL FLOOR IS SHOT TOO. The AFTER config is fired a SECOND time into
# null_<scene>.png. Any mean|dRGB| the A/B reports has to be read against the
# difference between two shots of the SAME config -- streaming order, TAA history
# and frame timing are not bit-stable, so "the images differ" proves nothing on
# its own. The floor is what makes the A/B number mean something.
#
# The deck's own bake is dumped on the AFTER arm (VOXELFORGE_CLOUDS_DUMP) so the
# texture the GPU sampled can be looked at directly -- see
# scripts\_poppy_clouds_preview.py.
#
# ASCII ONLY IN THIS FILE. A UTF-8 em dash mojibakes into a curly quote that
# PowerShell 5.1 reads as a string delimiter, which silently bricks the script.
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts\_poppy_clouds_shoot.ps1
# =============================================================================
$ErrorActionPreference = "Stop"
$Root  = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Out   = Join-Path $Root "docs\clouds-ab-poppy-2026-08-18"
$Stage = Join-Path $Root "_poppy_clouds_stage"
$TimeoutSec = 240

foreach ($d in @($Out, $Stage)) { if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null } }

$log = Join-Path $Out "_shoot.log"
if (Test-Path $log) { Remove-Item $log }
function Say($m) { $m | Tee-Object -FilePath $log -Append }

Say "REV  $(git -C $Root rev-parse --short HEAD)  branch $(git -C $Root branch --show-current)"

# ---- stage the ONE binary, pinned by md5 ------------------------------------
$src = Join-Path $Root "target-poppy\release\voxelforge.exe"
$exe = Join-Path $Stage "ab_voxelforge.exe"
if (-not (Test-Path $src)) { throw "no exe at $src -- build first" }
Copy-Item $src $exe -Force
$i = Get-Item $exe
Say ("EXE   {0}" -f $src.Replace($Root, ''))
Say ("      staged {0} bytes  srcmtime {1}  md5 {2}" -f $i.Length,
     (Get-Item $src).LastWriteTime.ToString('s'), (Get-FileHash $exe -Algorithm MD5).Hash)

# ---- prove the lever is IN this binary, by string scan, not by mtime --------
# A build that did not relink renders the old frame under the new env var and
# reports "no visible effect" -- which is how a live change gets called dead.
$t = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
$line = "SCAN "
foreach ($m in @("VOXELFORGE_CLOUDS","VOXELFORGE_CLOUDS_SUNDISK","VOXELFORGE_CLOUDS_DUMP",
                 "LOOK cloud-deck spawned","VOXELFORGE_CINE","VOXELFORGE_NOHUD","VOXELFORGE_LOOK_SUN")) {
    $line += "  $m=$((([regex]::Matches($t,[regex]::Escape($m)))).Count)"
}
Say $line
if ((([regex]::Matches($t,[regex]::Escape("LOOK cloud-deck spawned")))).Count -lt 1) {
    throw "staged exe carries no cloud deck -- it did not relink against look.rs"
}
Say ""

# ---- shared render settings -------------------------------------------------
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START   = "2.5"   # the bake runs on frame 1; let it settle

# Scenes. Both are sky-dominant framings, which the existing look plates are not
# -- a cloud deck cannot be judged on a camera pointed at the ground.
#   sunset-vista  looks ALONG the sun heading (azim 205) at a 3 deg sun: the disk,
#                 the hot forward-scatter lobe and the lit undersides.
#   dusk-away     looks 180 deg away at a 9 deg sun: the violet/magenta half of
#                 the deck, where the ambient term rather than the beam is doing
#                 the work. Same deck, opposite side -- if only one of the two
#                 moves, the lighting model is one-sided.
#
# The heading matters and is not eyeballed. `Hour::sun_dir` returns the direction
# light TRAVELS: at azim 205 that is xz = (sin 205, cos 205) = (-0.42, -0.91), so
# the sun itself sits at xz (+0.42, +0.91). Both cameras are aimed along that
# axis (toward it, then away from it) and pitched up ~3.4 deg, which puts the
# horizon at about 60% down the frame over the map's 2x2 chunks (64x64 blocks).
$Scenes = @(
    @{ name = "sunset-vista"
       cine = "20,22,6, 20,22,6, 62,28,97, 1"
       sun  = "3,205,9000"
       ev   = "10.1" },
    @{ name = "dusk-away"
       cine = "44,22,58, 44,22,58, 2,28,-33, 1"
       sun  = "9,205,14000"
       ev   = "10.3" }
)

function Invoke-Shot {
    param([string]$Tag, [hashtable]$Scene, [string]$Clouds, [string]$Dump)

    $env:VOXELFORGE_CINE          = $Scene.cine
    $env:VOXELFORGE_LOOK_SUN      = $Scene.sun
    $env:VOXELFORGE_LOOK_EXPOSURE = $Scene.ev
    if ($Clouds -eq "off") { $env:VOXELFORGE_CLOUDS = "off" }
    else { Remove-Item Env:\VOXELFORGE_CLOUDS -ErrorAction SilentlyContinue }
    if ($Dump) { $env:VOXELFORGE_CLOUDS_DUMP = $Dump }
    else { Remove-Item Env:\VOXELFORGE_CLOUDS_DUMP -ErrorAction SilentlyContinue }

    $shot = Join-Path $Out ($Tag + "_" + $Scene.name + ".png")
    if (Test-Path $shot) { Remove-Item $shot -Force }
    $env:VOXELFORGE_SHOT = $shot

    $runLog = Join-Path $Out ("_run_" + $Tag + "_" + $Scene.name + ".log")
    $errLog = Join-Path $Out ("_run_" + $Tag + "_" + $Scene.name + ".err.log")
    Say "[shoot] $Tag / $($Scene.name)  CLOUDS=$(if ($Clouds -eq 'off') {'off'} else {'on'})"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process -FilePath $exe -ArgumentList "--play" -PassThru -NoNewWindow `
                          -RedirectStandardOutput $runLog -RedirectStandardError $errLog
    if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
        Say "         !! TIMEOUT after $TimeoutSec s -- killing"
        try { $proc.Kill() } catch {}
        Start-Sleep -Milliseconds 800
    }
    $sw.Stop()

    if (Test-Path $shot) {
        $f = Get-Item $shot
        Say ("         wrote {0} bytes  md5 {1}  in {2:N1}s" -f $f.Length,
             (Get-FileHash $shot -Algorithm MD5).Hash, $sw.Elapsed.TotalSeconds)
    } else {
        Say "         !! NO FILE WRITTEN (after $([int]$sw.Elapsed.TotalSeconds)s)"
    }
    # Echo the deck's own spawn line so the log states what was baked, per arm.
    if (Test-Path $runLog) {
        Get-Content $runLog | Where-Object { $_ -match "cloud-deck|clouds dump|atmosphere spawned|sky-dome" } |
            ForEach-Object { Say ("         | " + $_) }
    }
}

foreach ($s in $Scenes) {
    Invoke-Shot -Tag "before" -Scene $s -Clouds "off"
    Invoke-Shot -Tag "after"  -Scene $s -Clouds "on" `
                -Dump (Join-Path $Out ("_bake_" + $s.name + ".rgba"))
    # Null floor: the AFTER config, again, nothing changed.
    Invoke-Shot -Tag "null"   -Scene $s -Clouds "on"
}

Say ""
Say "DONE"
Get-ChildItem $Out -Filter "*.png" | ForEach-Object { Say ("  {0,-34} {1,9} bytes" -f $_.Name, $_.Length) }
