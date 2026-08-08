# _pixel_light_ladder.ps1 -- sweep the LIGHT levers that already exist as env on a
# built exe, so the rebuild that bakes the winner is ONE build and an informed one.
#
# Shoots the same camera for every rung and changes exactly one thing per row.
# ASCII only (Windows PowerShell 5.1 reads .ps1 as ANSI).
param(
    [string]$Exe    = "target-flamingo\release\voxelforge.exe",
    [string]$OutDir = "_pixel_shadowdiag",
    [string]$Cam    = "s4",          # s4 | vista
    [string[]]$Only = @()
)
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Exe  = Join-Path $Root $Exe
$Out  = Join-Path $Root $OutDir
New-Item -ItemType Directory -Force $Out | Out-Null

# Cameras copied from the CINE lines the engine printed into _poppy_shotset/raw/*.log.
$CAMS = @{
    s4    = "54,12,44, 54,12,44, 26,3,20, 4"
    vista = "32,20,54, 32,20,54, 33,5,20, 4"
}
$Cine = $CAMS[$Cam]
if (-not $Cine) { throw "unknown cam '$Cam'" }

# rung = @{ name; sun = "elev,azim,illum" or $null (shipped default); amb = lux or $null }
$RUNGS = @(
    @{ name = "L00-shipped";      sun = $null;            amb = $null }
    @{ name = "L01-amb200";       sun = $null;            amb = "200"  }
    @{ name = "L02-e28";          sun = "28,205,11000";   amb = $null  }
    @{ name = "L03-e28-amb1200";  sun = "28,205,11000";   amb = "1200" }
    @{ name = "L04-amb1200";      sun = $null;            amb = "1200" }
    @{ name = "L05-e28-i18k-a1600"; sun = "28,205,18000"; amb = "1600" }
    @{ name = "L06-e35-i14k-a1400"; sun = "35,205,14000"; amb = "1400" }
    @{ name = "L07-e22-i16k-a1600"; sun = "22,205,16000"; amb = "1600" }

    # Round 2. Round 1 said the fill is the lever and elevation is not -- but the
    # fill is also what holds G3's p05 floor off the deck, so cutting it trades one
    # gate for another. These rungs move the OTHER side of the same ratio: raise the
    # key, hold the fill AND the exposure. Shadowed ground is lit by ambient alone,
    # so its absolute display level (= G3's p05) does not move; sunlit ground climbs
    # away from it and the valley opens between them.
    @{ name = "M1-i20k";      sun = "17,205,20000"; amb = $null }
    @{ name = "M2-i30k";      sun = "17,205,30000"; amb = $null }
    @{ name = "M3-i45k";      sun = "17,205,45000"; amb = $null }
    @{ name = "M4-e26-i20k";  sun = "26,205,20000"; amb = $null }
    @{ name = "M5-e26-i30k";  sun = "26,205,30000"; amb = $null }
    @{ name = "M6-e26-i45k";  sun = "26,205,45000"; amb = $null }
)

foreach ($r in $RUNGS) {
    if ($Only.Count -gt 0 -and ($Only -notcontains $r.name)) { continue }
    $name = "$Cam-$($r.name)"
    $png  = Join-Path $Out "$name.png"

    $env:VOXELFORGE_CINE        = $Cine
    $env:VOXELFORGE_CINE_START  = "0"
    $env:VOXELFORGE_NOHUD       = "1"
    $env:VOXELFORGE_SHOT        = $png
    if ($r.sun) { $env:VOXELFORGE_LOOK_SUN = $r.sun } else { Remove-Item env:VOXELFORGE_LOOK_SUN -ErrorAction SilentlyContinue }
    if ($r.amb) { $env:VOXELFORGE_LOOK_AMBIENT = $r.amb } else { Remove-Item env:VOXELFORGE_LOOK_AMBIENT -ErrorAction SilentlyContinue }

    $p = Start-Process -FilePath $Exe -ArgumentList "--play" -WorkingDirectory $Root -PassThru `
         -RedirectStandardOutput (Join-Path $Out "$name.out") `
         -RedirectStandardError  (Join-Path $Out "$name.err") -WindowStyle Minimized
    if (-not $p.WaitForExit(60000)) { $p.Kill(); Write-Output "TIMEOUT $name"; continue }
    # NOTE: no `??` -- Windows PowerShell 5.1 has no null-coalescing operator.
    $sunLbl = if ($r.sun) { $r.sun } else { "default" }
    $ambLbl = if ($r.amb) { $r.amb } else { "default" }
    Write-Output ("{0,-26} sun={1,-16} amb={2,-8} shot={3}" -f $name, $sunLbl, $ambLbl, (Test-Path $png))
}
