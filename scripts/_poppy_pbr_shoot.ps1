# =============================================================================
# Look v4 (PBR + exposure) before/after capture -- poppy.
#
# ONE BINARY, ONE CAMERA, ONE VARIABLE. Every pair below is the same exe, the
# same `VOXELFORGE_CINE` pose and the same hour; the only thing that moves is
# VOXELFORGE_LOOK_GEN (v3 = the surface rig that shipped 2026-08-17, v4 = that
# plus the per-block occlusion map and the exposure/midtone pass). Both halves
# of the pair read that ONE variable -- `look.rs::look_gen` for the post stack and
# `voxel.rs::occlusion_maps_enabled` for the material -- so there is no way to
# shoot a frame that is half of one generation and half of the other.
#
# Camera, hour, key/fill colour and base exposure are copied VERBATIM from
# `_poppy_lookv3_shoot.cmd`'s outdoor-noon scene, so these plates are
# like-for-like with `docs/assets/look/outdoor-noon_{before,after}.png` and the
# numbers in `docs/art-gap-vs-reference-2026-08-17.md`.
#
# Usage:  _poppy_pbr_shoot.ps1 [-Exe <path>] [-Out <dir>] [-Sweep "<name>=<env;env>"]
#
# -Sweep shoots EXTRA v4 frames with additional environment overrides, so the
# tuning knobs (VOXELFORGE_LOOK_EVTRIM / _GAIN / _BLOOM / VOXELFORGE_MAT_AO) can
# be found from the binary that is already built instead of one link per guess.
# =============================================================================
param(
    [string]$Exe = "",
    [string]$Out = "",
    [string[]]$Sweep = @()
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
if (-not $Exe) { $Exe = Join-Path $Root "target-poppy\release\voxelforge.exe" }
if (-not $Out) { $Out = Join-Path $Root "_poppy_pbr" }

if (-not (Test-Path $Exe)) { throw "no exe at $Exe -- build first" }
if (-not (Test-Path $Out)) { New-Item -ItemType Directory -Path $Out | Out-Null }

$log = Join-Path $Out "_shoot.log"
if (Test-Path $log) { Remove-Item $log }

# The exe is what the plates are evidence OF, so its identity goes in the log
# before a single frame is written -- a before/after whose binary nobody recorded
# is a pair of pictures, not a measurement.
$exeInfo = Get-Item $Exe
$md5 = (Get-FileHash $Exe -Algorithm MD5).Hash
"EXE  $Exe"                              | Tee-Object -FilePath $log -Append
"     $($exeInfo.Length) bytes  mtime $($exeInfo.LastWriteTime.ToString('s'))  md5 $md5" | Tee-Object -FilePath $log -Append
"REV  $(git -C $Root rev-parse --short HEAD)  branch $(git -C $Root branch --show-current)" | Tee-Object -FilePath $log -Append

# ---- shared: playable scene, no HUD, top tier, one still ---------------------
$env:VOXELFORGE_PLAY = "1"
$env:VOXELFORGE_NOHUD = "1"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_CINE_START = "1.0"

# ---- scene: outdoor noon (verbatim from _poppy_lookv3_shoot.cmd) ------------
# Sun at 66deg with a neutral-white key and a cool fill. Deliberately the least
# flattering hour: with the warm key gone, none of the difference can be colour.
$env:VOXELFORGE_CINE = "44,14,44, 44,14,44, 32.5,2.0,29.5, 1"
$env:VOXELFORGE_LOOK_SUN = "66,205,20000"
$env:VOXELFORGE_LOOK_LIGHT = "1.00,0.98,0.93,0.84,0.88,1.00"
$env:VOXELFORGE_LOOK_EXPOSURE = "10.6"

function Invoke-Shot {
    param([string]$Gen, [string]$Name, [hashtable]$Extra = @{})

    $env:VOXELFORGE_LOOK_GEN = $Gen
    $shot = Join-Path $Out "outdoor-noon_$Name.png"
    $env:VOXELFORGE_SHOT = $shot
    foreach ($k in $Extra.Keys) { Set-Item -Path "env:$k" -Value $Extra[$k] }

    $extraTxt = ($Extra.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join " "
    "[shoot] gen=$Gen -> $Name  $extraTxt" | Tee-Object -FilePath $log -Append
    & $Exe --play *>> $log

    foreach ($k in $Extra.Keys) { Remove-Item -Path "env:$k" -ErrorAction SilentlyContinue }

    if (Test-Path $shot) {
        "         wrote $((Get-Item $shot).Length) bytes  md5 $((Get-FileHash $shot -Algorithm MD5).Hash)" |
            Tee-Object -FilePath $log -Append
    } else {
        "         !! NO FILE WRITTEN" | Tee-Object -FilePath $log -Append
    }
}

Invoke-Shot -Gen "v3" -Name "before"
Invoke-Shot -Gen "v4" -Name "after"

# ---- optional sweep ---------------------------------------------------------
# Each -Sweep entry is "<name>=<VAR=val>;<VAR=val>" and shoots one more v4 frame.
foreach ($s in $Sweep) {
    $name, $rest = $s -split "=", 2
    $extra = @{}
    foreach ($pair in ($rest -split ";")) {
        if (-not $pair.Trim()) { continue }
        $k, $v = $pair -split "=", 2
        $extra[$k.Trim()] = $v.Trim()
    }
    Invoke-Shot -Gen "v4" -Name $name -Extra $extra
}

"DONE" | Tee-Object -FilePath $log -Append
Get-ChildItem $Out -Filter "*.png" | ForEach-Object { "  $($_.Name)  $($_.Length)" }
