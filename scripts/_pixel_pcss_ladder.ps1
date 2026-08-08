# _pixel_pcss_ladder.ps1 -- sweep PCSS_WIDTH on a BUILT exe to find out whether the
# penumbra is real or eaten by Bevy's blur floor, before spending a rebuild on it.
#
# WHY A SWEEP AND NOT A GUESS. bevy_pbr 0.19 shadow_sampling.wgsl:303 is
#     blur_size = max((z_blocker - depth) * light_size / depth, 0.5)   // TEXELS
# so the shipped PCSS_WIDTH only does anything at all when
#     (z_blocker - depth) * light_size / depth > 0.5.
# Both z's are cascade NDC (reverse-Z, ndc = 1 + z_lightspace/dz), so the left side
# scales as gap_blocks/cascade_depth_span -- a number in the thousandths here. The
# ladder is what turns "the clamp probably bites" into a measured curve: if the
# 20-80% edge width does not move between rungs, every rung is sitting on the 0.5
# floor and PCSS is off in all but name.
#
# Tier is forced to Ultra because that is the ONLY tier that turns PCSS on
# (apply_look_to_sun: `let pcss = matches!(*quality, LookQuality::Ultra)`).
# One binary, one camera, one variable. ASCII only (PS 5.1 reads .ps1 as ANSI).
param(
    [string]$Exe    = "target-flamingo\release\voxelforge.exe",
    [string]$OutDir = "_pixel_pcss",
    [string]$Cam    = "s4",          # s4 | vista
    [string[]]$Only = @()
)
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Exe  = Join-Path $Root $Exe
$Out  = Join-Path $Root $OutDir
New-Item -ItemType Directory -Force $Out | Out-Null

# Same CINE lines the light ladder uses, so plates are comparable across sweeps.
$CAMS = @{
    s4    = "54,12,44, 54,12,44, 26,3,20, 4"
    vista = "32,20,54, 32,20,54, 33,5,20, 4"
}
$Cine = $CAMS[$Cam]
if (-not $Cine) { throw "unknown cam '$Cam'" }

# Geometric, not linear: if the clamp is biting, a linear sweep just prints the
# same number six times. Spanning 4 -> 1024 covers the whole plausible range of
# "where does this constant stop being decorative".
$RUNGS = @(
    @{ name = "P000-off";  pcss = "off"  }
    @{ name = "P004-ship"; pcss = "4"    }
    @{ name = "P016";      pcss = "16"   }
    @{ name = "P064";      pcss = "64"   }
    @{ name = "P256";      pcss = "256"  }
    @{ name = "P1024";     pcss = "1024" }
)

foreach ($r in $RUNGS) {
    if ($Only.Count -gt 0 -and ($Only -notcontains $r.name)) { continue }
    # -nohud2 suffix: NOHUD=1 makes the frame HUD-free in-engine, and the graders'
    # nohud2_guard refuses to speak about a file without it.
    $name = "$Cam-$($r.name)-nohud2"
    $png  = Join-Path $Out "$name.png"

    $env:VOXELFORGE_CINE         = $Cine
    $env:VOXELFORGE_CINE_START   = "0"
    $env:VOXELFORGE_NOHUD        = "1"
    $env:VOXELFORGE_SHOT         = $png
    $env:VOXELFORGE_LOOK_QUALITY = "ultra"
    $env:VOXELFORGE_LOOK_PCSS    = $r.pcss

    $p = Start-Process -FilePath $Exe -ArgumentList "--play" -WorkingDirectory $Root -PassThru `
         -RedirectStandardOutput (Join-Path $Out "$name.out") `
         -RedirectStandardError  (Join-Path $Out "$name.err") -WindowStyle Minimized
    if (-not $p.WaitForExit(90000)) { $p.Kill(); Write-Output "TIMEOUT $name"; continue }
    Write-Output ("{0,-24} pcss={1,-6} shot={2}" -f $name, $r.pcss, (Test-Path $png))
}
Remove-Item env:VOXELFORGE_LOOK_PCSS    -ErrorAction SilentlyContinue
Remove-Item env:VOXELFORGE_LOOK_QUALITY -ErrorAction SilentlyContinue
