# _yama_feeldemo_after_capture.ps1
#
# Records the "after" clip for the player_tuning.rs feel refactor, using the
# NEW exe (post-rebuild at target-yamamoto/release/voxelforge.exe) which has
# the in-engine feel-demo harness (client/src/main.rs ~L3470-3557):
#   VOXELFORGE_FEEL_DEMO=1        -- auto-plays walk(0.5-2.5s)->run(2.5-4.0s)
#                                     ->jump(4.0-4.2s)->walk resumed(4.2-6.5s),
#                                     forces play mode, no window focus/input
#                                     injection needed (unlike the before clip)
#   VOXELFORGE_FEEL_DEMO_FILM=dir -- dumps frame_00000.png.. at 20fps for the
#                                     whole run into dir, exits itself at t=6.0s
#                                     (FEEL_DEMO_EXIT_AT) -- fully headless-safe,
#                                     no gdigrab/ddagrab/SendInput needed at all.
#
# Usage: run AFTER the real rebuild finishes (Director is handling that via
# the office job / -j 2 --target-dir target-yamamoto build).
#   powershell -File scripts\_yama_feeldemo_after_capture.ps1
#
# Output: _yama_feeldemo/after_<ts>.mp4 (encoded from the PNG sequence)

$ErrorActionPreference = "Stop"
$ProjectDir = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $ProjectDir

$Exe = Join-Path $ProjectDir "target-yamamoto\release\voxelforge.exe"
$Out = Join-Path $ProjectDir "_yama_feeldemo"
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Ts = Get-Date -Format "yyyyMMdd-HHmmss"
$FilmDir = Join-Path $Out "after_${Ts}_frames"
New-Item -ItemType Directory -Force -Path $FilmDir | Out-Null
$Log = Join-Path $Out "after_$Ts.log"
$Mp4 = Join-Path $Out "after_$Ts.mp4"
$FEEL_DEMO_FILM_FPS = 20   # must match client/src/main.rs FEEL_DEMO_FILM_FPS

if (-not (Test-Path $Exe)) { throw "exe not found: $Exe" }

# --- freshness gate: refuse to run against a stale (pre-refactor) binary ---
$srcMtime = (Get-Item (Join-Path $ProjectDir "client\src\main.rs")).LastWriteTime
$exeMtime = (Get-Item $Exe).LastWriteTime
Write-Output "exe mtime   : $($exeMtime.ToString('yyyy-MM-dd HH:mm:ss'))"
Write-Output "main.rs mtime: $($srcMtime.ToString('yyyy-MM-dd HH:mm:ss'))"
if ($exeMtime -lt $srcMtime) {
    throw "exe is OLDER than client/src/main.rs -- this is still the 'before' binary, rebuild first"
}
Write-Output "freshness check: PASS (exe is newer than main.rs)"

Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

Write-Output "=== LAUNCH (after exe, VOXELFORGE_FEEL_DEMO + FEEL_DEMO_FILM) ==="
$envBackup = @{
    FEEL_DEMO = $env:VOXELFORGE_FEEL_DEMO
    FEEL_FILM = $env:VOXELFORGE_FEEL_DEMO_FILM
}
$env:VOXELFORGE_FEEL_DEMO = "1"
$env:VOXELFORGE_FEEL_DEMO_FILM = $FilmDir
try {
    $proc = Start-Process -FilePath $Exe -PassThru -RedirectStandardOutput $Log -RedirectStandardError "$Log.err"
    Write-Output "game PID: $($proc.Id)"

    # Self-exits at FEEL_DEMO_EXIT_AT=6.0s in-engine; give it a generous ceiling
    # in case Sun's build is still contending for CPU when this runs.
    $exited = $proc.WaitForExit(60000)
    if (-not $exited) {
        $proc.Kill()
        throw "feel-demo did not self-exit within 60s -- something is wrong (check $Log)"
    }
    Write-Output "game exited cleanly (exit code $($proc.ExitCode))"
} finally {
    $env:VOXELFORGE_FEEL_DEMO = $envBackup.FEEL_DEMO
    $env:VOXELFORGE_FEEL_DEMO_FILM = $envBackup.FEEL_FILM
}

$frames = Get-ChildItem $FilmDir -Filter "frame_*.png" | Sort-Object Name
Write-Output "frames captured: $($frames.Count)"
if ($frames.Count -eq 0) { throw "no frames captured in $FilmDir -- check $Log for FEEL_SHOT/errors" }

# --- encode PNG sequence -> mp4 ---
Write-Output "=== ENCODE (ffmpeg, ${FEEL_DEMO_FILM_FPS}fps) ==="
& ffmpeg -y -hide_banner -loglevel warning -framerate $FEEL_DEMO_FILM_FPS -i (Join-Path $FilmDir "frame_%05d.png") -c:v libx264 -preset medium -crf 18 -pix_fmt yuv420p $Mp4
if (-not (Test-Path $Mp4) -or (Get-Item $Mp4).Length -lt 3000) { throw "encode failed: $Mp4 missing/empty" }
$sizeKb = [math]::Round((Get-Item $Mp4).Length / 1KB, 1)
Write-Output "capture: $Mp4 (${sizeKb} KB)"

# --- scene-change check: prove it's not a frozen/blank clip ---
$prevEap = $ErrorActionPreference
$ErrorActionPreference = "Continue"
$sceneOut = (& ffmpeg -i $Mp4 -vf "select='gt(scene,0.003)',showinfo" -vsync vfr -f null NUL) 2>&1 | Out-String
$ErrorActionPreference = $prevEap
$changed = ([regex]::Matches($sceneOut, "showinfo")).Count
Write-Output "scene-change frames: $changed"
if ($changed -eq 0) { throw "0 scene-change frames -- clip is frozen/blank, capture is not usable" }

Write-Output "log: $Log"
Write-Output "DONE ts=$Ts"
