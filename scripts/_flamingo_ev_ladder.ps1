# EV ladder — hero + s1-vista, one binary, one knob.
#
# The ONLY variable across rungs is VOXELFORGE_LOOK_EXPOSURE, injected via
# -ExtraEnv. The exe is the frozen 05:38 target-flamingo build for every rung, so
# a difference between rungs cannot be a relink. Nothing here touches client/src
# or invokes cargo — that is the point: prove the value before baking it.
param(
    [string[]]$Evs   = @('10.9','10.6','10.3'),
    [string]$Plates  = 'hero,s1-vista',
    [string]$Exe     = 'target-flamingo\release\voxelforge.exe',
    [string]$Root    = '_flamingo_ev'
)
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $PSCommandPath
$repo = (Resolve-Path (Join-Path $here '..')).Path
Push-Location $repo
try {
    foreach ($ev in $Evs) {
        $out = "$Root\ev$ev"
        "`n############ EV $ev -> $out ############"
        powershell -NoProfile -File scripts/_poppy_shotset.ps1 `
            -Only $Plates `
            -ExtraEnv "VOXELFORGE_LOOK_EXPOSURE=$ev" `
            -OutDir $out `
            -Exe $Exe `
            -NoRegrade 2>&1 | Write-Output
        "############ EV $ev exit=$LASTEXITCODE ############"
    }
} finally { Pop-Location }
