# =============================================================================
# Poppy -- the look-v4 A/B, done the way the last one was not.
#
# The 2026-08-18 pair in docs\look-ab-poppy-2026-08-18 was void: it shot an
# Aug-8 Flamingo binary against an Aug-18 binary and never set
# VOXELFORGE_LOOK_GEN at all, so the "after" ran gen=V3 and the two plates
# straddled ten days of every lane's work instead of a look generation. See
# docs\look-ab-poppy-2026-08-18\CONTROL-FINDINGS.md.
#
# This script fixes exactly that, and refuses to run if it cannot:
#
#   ONE BINARY.  Both plates come from the same staged exe, md5 in the log, so
#                nothing but the lever can differ.
#   LEVER SET EXPLICITLY.  v3 vs v4 as WORDS. Never unset, never 0/1 --
#                `look_gen()` maps anything it does not recognise to the DEFAULT
#                generation (v3), so an unset or typo'd lever silently prints
#                two identical v3 plates. Both arms name their generation and
#                the engine's own `gen=` line is echoed back into this log, so a
#                lever that did not take shows up as v3/v3 instead of passing.
#   STRADDLE PROVEN IN THE BINARY.  The exe is scanned for the v4 marker
#                strings before a single frame is shot. No marker => hard stop,
#                because an exe without v4 cannot shoot a v4 plate no matter
#                what the env says. mtime and commit order prove nothing.
#
# Camera / sun / light / exposure are VERBATIM from _poppy_ab_20260818_shoot.ps1
# (which took them from _poppy_pbr_shoot.ps1 <- _poppy_lookv3_shoot.cmd), so
# these plates stay like-for-like with docs\assets\look\outdoor-noon_*.png.
#
# Usage: _poppy_v4_ab_shoot.ps1 [-Exe <path>] [-DryRun]
# =============================================================================
param(
    [string]$Exe = "",
    [switch]$DryRun
)
$ErrorActionPreference = "Stop"

$Root  = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Out   = Join-Path $Root "docs\look-v4-ab-2026-08-18"
$Stage = Join-Path $Root "_poppy_v4_stage"
$TimeoutSec = 240

# The world-renderer bin, NOT voxelforge_shot: shot_main.rs is the isolated
# hero-shot bin and never declares `mod look` / `mod voxel`, so no build of it
# can ever carry a look generation. This default was wrong in the first cut and
# the gate below is what caught it.
if (-not $Exe) { $Exe = Join-Path $Root "target\release\voxelforge.exe" }

foreach ($d in @($Out, $Stage)) {
    if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
}
$log = Join-Path $Out "_shoot.log"
if (Test-Path $log) { Remove-Item $log -Force }
function Say($m) { $m | Tee-Object -FilePath $log -Append }

Say "REV  $(git -C $Root rev-parse --short HEAD)  branch $(git -C $Root branch --show-current)"
Say "EXE  $Exe"

if (-not (Test-Path $Exe)) { throw "no exe at $Exe -- build first, do not shoot a stale one" }

# ---- stage ONE binary; both plates come from this copy ----------------------
$staged = Join-Path $Stage "v4ab_voxelforge_shot.exe"
Copy-Item $Exe $staged -Force
$i = Get-Item $staged
$h = (Get-FileHash $staged -Algorithm MD5).Hash
Say "STAGED  $($i.Length) bytes  srcmtime $((Get-Item $Exe).LastWriteTime.ToString('s'))  md5 $h"

# ---- GATE: the v4 path must be IN this binary -------------------------------
# Scanning for marker strings that ONLY the v4 commit introduced. The first cut
# of this gate scanned VOXELFORGE_LOOK_GEN / LOOK_IBL / LOOK_FILL and was
# useless: target\release\voxelforge.exe (Aug 18 00:18, two hours BEFORE a07348e
# landed v4) carries all three -- 1/2/2 hits -- so the gate would have waved a
# zero-v4 binary straight through. These three env names are read only inside
# the v4 grade block, so an exe that predates a07348e cannot contain them.
Say ""
Say "--- v4 straddle gate (scanned in the staged exe, not inferred from mtime) ---"
$bytes = [System.IO.File]::ReadAllBytes($staged)
$text  = [System.Text.Encoding]::ASCII.GetString($bytes)
$markers = @("VOXELFORGE_LOOK_EVTRIM", "VOXELFORGE_LOOK_GAIN", "VOXELFORGE_LOOK_BLOOM")
$missing = @()
foreach ($m in $markers) {
    $n = ([regex]::Matches($text, [regex]::Escape($m))).Count
    Say ("  {0,-24} {1}" -f $m, $n)
    if ($n -eq 0) { $missing += $m }
}
if ($missing.Count -gt 0) {
    Say ""
    Say "GATE FAILED -- missing from the exe: $($missing -join ', ')"
    throw "this exe cannot shoot a v4 plate (missing: $($missing -join ', ')). Build, then re-run."
}
Say "  gate PASS -- the v4 grade block is compiled into this exe"

# ---- shared render settings (identical for both arms) -----------------------
$env:VOXELFORGE_PLAY          = "1"
$env:VOXELFORGE_NOHUD         = "1"
$env:VOXELFORGE_LOOK_QUALITY  = "ultra"
$env:VOXELFORGE_CINE_START    = "1.0"

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
    param([string]$Tag, [string]$Gen, [hashtable]$Scene)

    $env:VOXELFORGE_LOOK_GEN      = $Gen      # <-- the ONLY thing that differs
    $env:VOXELFORGE_CINE          = $Scene.cine
    $env:VOXELFORGE_LOOK_SUN      = $Scene.sun
    $env:VOXELFORGE_LOOK_LIGHT    = $Scene.light
    $env:VOXELFORGE_LOOK_EXPOSURE = $Scene.ev

    $shot = Join-Path $Out ($Tag + "_" + $Scene.name + ".png")
    if (Test-Path $shot) { Remove-Item $shot -Force }
    $env:VOXELFORGE_SHOT = $shot

    $o = Join-Path $Out ("_run_" + $Tag + "_" + $Scene.name + ".log")
    $e = Join-Path $Out ("_run_" + $Tag + "_" + $Scene.name + ".err.log")

    Say "[shoot] $Tag / $($Scene.name)   LOOK_GEN=$Gen"
    if ($DryRun) { Say "         (dry run -- not launching)"; return }

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $staged -ArgumentList "--play" -PassThru -NoNewWindow `
                       -RedirectStandardOutput $o -RedirectStandardError $e
    if (-not $p.WaitForExit($TimeoutSec * 1000)) {
        try { $p.Kill() } catch {}
        Say "         TIMEOUT after ${TimeoutSec}s"
        return
    }
    $sw.Stop()

    if (Test-Path $shot) {
        $s = Get-Item $shot
        $m = (Get-FileHash $shot -Algorithm MD5).Hash
        Say ("         wrote {0} bytes  md5 {1}  in {2:N1}s" -f $s.Length, $m, $sw.Elapsed.TotalSeconds)
    } else {
        Say "         NO FILE WRITTEN (exit $($p.ExitCode)) -- see $e"
    }
    # what the engine itself says it ran; the plate is only as good as this line
    $gl = Select-String -Path $o -Pattern "LOOK_IBL gen=|LOOK_FILL gen=" -ErrorAction SilentlyContinue
    foreach ($l in $gl) { Say "         $($l.Line.Trim())" }
}

foreach ($sc in $Scenes) {
    Invoke-Shot -Tag "v3" -Gen "v3" -Scene $sc
    Invoke-Shot -Tag "v4" -Gen "v4" -Scene $sc
}

# ---- the pair is only real if the two plates actually differ ----------------
Say ""
Say "--- pair check (identical md5 = the lever did nothing) ---"
foreach ($sc in $Scenes) {
    $a = Join-Path $Out ("v3_" + $sc.name + ".png")
    $b = Join-Path $Out ("v4_" + $sc.name + ".png")
    if ((Test-Path $a) -and (Test-Path $b)) {
        $ha = (Get-FileHash $a -Algorithm MD5).Hash
        $hb = (Get-FileHash $b -Algorithm MD5).Hash
        Say ("  {0,-16} {1}" -f $sc.name, $(if ($ha -eq $hb) { "IDENTICAL -- LEVER DID NOTHING, pair is void" } else { "differ OK" }))
    } else {
        Say ("  {0,-16} MISSING a plate" -f $sc.name)
    }
}

Say ""
Say "DONE"
Get-ChildItem $Out -Filter *.png | ForEach-Object { Say ("    {0,-40} {1} bytes" -f $_.Name, $_.Length) }
