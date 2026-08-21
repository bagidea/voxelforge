# Hero-rig film: shoot the BEFORE (capsule) and AFTER (rig) cuts from ONE binary,
# through the same camera and the same scripted stick. The only thing that differs
# between the two runs is VOXELFORGE_ANIM_RIG_OFF.
#
#   powershell -File scripts/_poppy_herofilm_shoot.ps1 -Exe target-poppy/perf/voxelforge.exe -Out _poppy_hero
#
# The stick is main.rs's existing feel-demo (walk 0.5-2.5 s, run 2.5-4.0 s, jump
# 4.0-4.2 s, walk resumed to 6.0 s); VOXELFORGE_FEEL_DEMO_FILM turns its nine
# sampled beats into every frame. The framing comes from the existing
# VOXELFORGE_LOOK_CAM boom override, not from new camera code.
#
# ASCII only, and no em dashes: PS 5.1 mojibakes UTF-8 punctuation into a curly
# quote it then treats as a string delimiter, which bricks the whole file.
param(
    [string]$Exe = "target-poppy/perf/voxelforge.exe",
    [string]$Out = "_poppy_hero",
    [int]$TimeoutSec = 900,
    [string]$Cam = "0,-12,5.5",
    [switch]$AfterOnly
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not (Test-Path $Exe)) {
    Write-Output "SHOOT FAIL: no exe at $Exe"
    exit 2
}
$exeFull = (Resolve-Path $Exe).Path
Write-Output ("SHOOT exe={0} bytes={1} mtime={2}" -f $exeFull, (Get-Item $exeFull).Length, (Get-Item $exeFull).LastWriteTime)

# Gate the binary on the levers it is about to be driven with. An exe that never
# saw VOXELFORGE_FEEL_DEMO_FILM will run as a plain --play session, film nothing,
# and never exit -- and the only symptom would be an empty directory. Scan for the
# strings, not the mtime: a link stamp is not a rebuild.
$bytes = [System.IO.File]::ReadAllBytes($exeFull)
$text = [System.Text.Encoding]::ASCII.GetString($bytes)
foreach ($needle in @("VOXELFORGE_FEEL_DEMO_FILM", "VOXELFORGE_ANIM_RIG_OFF", "FEEL_FILM done")) {
    if ($text.IndexOf($needle) -lt 0) {
        Write-Output "SHOOT FAIL: exe has no '$needle' - it predates the film lane"
        exit 3
    }
    Write-Output "  lever ok: $needle"
}
$bytes = $null; $text = $null

# (6.0 - 0.30) / (1/30), i.e. FEEL_DEMO_EXIT_AT - FEEL_FILM_FROM over FEEL_FILM_DT.
$expected = 171

function Invoke-Film {
    param([string]$Label, [string]$Dir, [bool]$RigOff)

    if (Test-Path $Dir) { Remove-Item -Recurse -Force $Dir }
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null

    $env:VOXELFORGE_FEEL_DEMO_FILM = $Dir
    $env:VOXELFORGE_LOOK_CAM = $Cam
    if ($RigOff) { $env:VOXELFORGE_ANIM_RIG_OFF = "1" } else { Remove-Item Env:\VOXELFORGE_ANIM_RIG_OFF -ErrorAction SilentlyContinue }

    $log = Join-Path $Dir "_run.log"
    $err = Join-Path $Dir "_run.err"
    $started = Get-Date
    $p = Start-Process -FilePath $exeFull -NoNewWindow -PassThru -RedirectStandardOutput $log -RedirectStandardError $err
    $p.WaitForExit($TimeoutSec * 1000) | Out-Null
    if (-not $p.HasExited) {
        $p.Kill()
        Write-Output "  $Label TIMEOUT after $TimeoutSec s"
    }
    $secs = [int]((Get-Date) - $started).TotalSeconds
    $n = (Get-ChildItem -Path $Dir -Filter "f*.png" | Measure-Object).Count
    $done = Select-String -Path $log -Pattern "^FEEL_FILM done" -ErrorAction SilentlyContinue
    Write-Output ("  {0}: frames={1} wall={2}s exit={3} done='{4}'" -f $Label, $n, $secs, $p.ExitCode, ($done -join ""))
    return $n
}

$nAfter = Invoke-Film -Label "AFTER (rig)" -Dir (Join-Path $Out "film_after") -RigOff $false
$nBefore = 0
if (-not $AfterOnly) {
    $nBefore = Invoke-Film -Label "BEFORE (capsule)" -Dir (Join-Path $Out "film_before") -RigOff $true
}

Remove-Item Env:\VOXELFORGE_FEEL_DEMO_FILM -ErrorAction SilentlyContinue
Remove-Item Env:\VOXELFORGE_LOOK_CAM -ErrorAction SilentlyContinue
Remove-Item Env:\VOXELFORGE_ANIM_RIG_OFF -ErrorAction SilentlyContinue

if ($nAfter -lt ($expected - 4)) { Write-Output "SHOOT FAIL: after film is only $nAfter frames (expected $expected)"; exit 4 }
if ((-not $AfterOnly) -and $nBefore -ne $nAfter) {
    Write-Output "SHOOT FAIL: before=$nBefore after=$nAfter - the pair is not frame-aligned"
    exit 5
}
Write-Output "SHOOT OK before=$nBefore after=$nAfter expected=$expected"
