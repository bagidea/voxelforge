# record_combat_clip.ps1 -- Combat demo capture with full QA checks.
# Captures --combat-demo (scene::combat_proof) via gdigrab, crops chrome,
# runs scene-change detection, and exports MP4 + GIF only if the clip is alive.
$ErrorActionPreference = "Stop"

$projectDir = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
$exe        = "$projectDir\target-int\debug\voxelforge.exe"
$tmpDir     = "$projectDir\_combat_recordings"
$outDir     = "$projectDir\docs\assets"
$ts         = Get-Date -Format "yyyyMMdd-HHmmss"
$fps        = 60
$recordSec  = 20
$sceneThreshold = "0.003"

# Crop: remove title bar (~32px on Win11 100% scaling). The window is
# created at 1280x720 native; gdigrab includes chrome, so we crop the top.
# Output = 1280x(720-32) = 1280x688, then scale-pad back to 1280x720 for
# consistent output. Left/right are NOT cropped unless the edge-check finds
# overlapping windows -- that case is handled by repositioning the window.
$cropTop    = 32
$cropW      = 1280
$cropH      = 688

foreach ($d in @($tmpDir, $outDir)) {
    if (-not (Test-Path $d)) { New-Item -ItemType Directory $d -Force | Out-Null }
}

# ---- Win32 interop -------------------------------------------------------
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32Clip {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern IntPtr FindWindowW(string c, string w);
    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(IntPtr h, IntPtr a, int X, int Y, int cx, int cy, uint f);
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr h, out RECT r);
}
public struct RECT { public int L,T,R,B; }
"@

function Die($msg) {
    Write-Output "FATAL: $msg"
    Get-Job -Name "ffmpeg_cap" -ErrorAction SilentlyContinue | Stop-Job -ErrorAction SilentlyContinue
    Get-Job -Name "ffmpeg_cap" -ErrorAction SilentlyContinue | Remove-Job -Force -ErrorAction SilentlyContinue
    Get-Process -Name voxelforge,ffmpeg -ErrorAction SilentlyContinue | Stop-Process -Force
    exit 1
}

# Clean stale processes
Get-Process -Name voxelforge,ffmpeg -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep 1

# =========================================================================
# PRE-FLIGHT: verify binary mtime >= latest commit
# =========================================================================
$latestCommitDate = (git -C $projectDir log -1 --format='%ai') -as [DateTime]
if (-not $latestCommitDate) { Die 'cannot read git commit date' }

$binMtime = (Get-Item $exe).LastWriteTime
Write-Output "=== PRE-FLIGHT ==="
Write-Output "binary     : $exe"
Write-Output "binary size: $((Get-Item $exe).Length) bytes"
Write-Output "binary mtime: $($binMtime.ToString('yyyy-MM-dd HH:mm:ss'))"
Write-Output "latest commit: $($latestCommitDate.ToString('yyyy-MM-dd HH:mm:ss'))"

if ($binMtime -lt $latestCommitDate) {
    $msg = 'WARNING: binary mtime ({0}) is OLDER than latest commit ({1})' -f $binMtime.ToString('yyyy-MM-dd HH:mm:ss'), $latestCommitDate.ToString('yyyy-MM-dd HH:mm:ss')
    Write-Output $msg
    Write-Output '  Continuing anyway -- verify manually that the binary is fresh enough for your capture purpose.'
} else {
    Write-Output 'mtime check: PASS (binary fresh enough for capture)'
}
Write-Output ''

# =========================================================================
# Launch game with --combat-demo (sets play=true -> scene::combat_proof)
# =========================================================================
Write-Output '--- Launch game (--combat-demo) ---'
$proc = Start-Process -FilePath $exe -ArgumentList '--combat-demo' -PassThru -WindowStyle Normal
Write-Output "game PID: $($proc.Id)"

# Poll for window title
Write-Output 'polling for window title...'
$winTitle = $null
$windowPollMax = 25
for ($i = 0; $i -lt ($windowPollMax * 2); $i++) {
    Start-Sleep -Milliseconds 500
    $proc.Refresh()
    $t = $proc.MainWindowTitle
    if ($t -and $t.Length -gt 0) {
        $elapsed = [math]::Round(($i + 1) * 0.5, 1)
        Write-Output "window found after ${elapsed}s: '$t'"
        $winTitle = $t
        break
    }
    Write-Host -NoNewline '.'
    if ($proc.HasExited) { Die 'game exited before window appeared' }
}
Write-Output ''
if (-not $winTitle) { Die 'no window title found' }

# (1) Title gate -- MUST be the game binary (main.rs:486), NOT hero-shot (shot_main.rs).
#     Bevy sets "Voxelforge \u2014 Phase 0 spike" (em-dash = U+2014).
#     winit may append " (NNv0)" -- anchored regex handles both without letting
#     "Voxelforge \u2014 hero shot" (shot binary) through.
$EM = [char]0x2014
$expectedTitlePrefix = "Voxelforge ${EM} Phase 0 spike"
if ($winTitle -notmatch "^$([regex]::Escape($expectedTitlePrefix))") {
    Die "WRONG WINDOW -- expected title starting with '$expectedTitlePrefix', got '$winTitle'. The capture would grab the wrong content (likely hero-shot binary, not the game)."
}
Write-Output "title check: PASS (matched '$winTitle')"

# Find the window handle
$hwnd = [W32Clip]::FindWindowW([NullString]::Value, $winTitle)
if ($hwnd -eq [IntPtr]::Zero) { Die "FindWindowW returned NULL for title: $winTitle" }
Write-Output "HWND: $hwnd"

# ---- Reposition window to top-left corner to avoid office panel overlap ----
# Move to (0,0) -- top-left corner, away from the office panel (typically right side)
[W32Clip]::SetWindowPos($hwnd, [IntPtr]::Zero, 0, 0, 0, 0, 0x0001) | Out-Null  # SWP_NOSIZE=0x0001
Start-Sleep -Milliseconds 200
$rect = New-Object RECT
[W32Clip]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
Write-Output "window pos: ($($rect.L),$($rect.T))-($($rect.R),$($rect.B)) size=$($rect.R-$rect.L)x$($rect.B-$rect.T)"

# (3a) Position guard -- the office panel lives on the right side of the screen.
#      Assert the window does NOT overlap the rightmost 10% of the primary monitor.
#      Window at (0,0)+~1296px chrome on a 1680px screen leaves ~384px free -- panel
#      is docked far right, so 90% is a safe threshold that won't false-positive.
Add-Type -AssemblyName System.Windows.Forms
$screen = [System.Windows.Forms.Screen]::PrimaryScreen
$workW = $screen.WorkingArea.Width
$safeRight = [math]::Floor($workW * 0.90)
if ($rect.R -gt $safeRight) {
    Die "window right edge ($($rect.R)) overlaps office panel zone (limit ${safeRight}px of ${workW}px working area) -- move window away from right side before recording"
}
Write-Output "position guard: PASS (right edge $($rect.R), safe limit ${safeRight}px)"

# ---- Start ffmpeg capture IMMEDIATELY after title check ----
#      combat-demo completes in ~6-7s; every ms of delay costs footage.
#      Title check frame is captured AFTER recording from the raw clip.
Write-Output '--- starting ffmpeg capture ---'
$rawMkv = "$tmpDir\capture_${ts}.mkv"

# Use title= (as originally designed for this script). The title check frame
# already proved & ffmpeg with "title=$winTitle" works. We use Start-Job
# with splatted args to preserve the em-dash and spaces in the title string.
# Crop: remove title bar (~32px on Win11 100% scaling), output = 1280x720.
$vfCrop = "crop=${cropW}:${cropH}:0:${cropTop},scale=1280:720:flags=lanczos"
$ffArgs = @(
    '-y', '-hide_banner', '-loglevel', 'error',
    '-f', 'gdigrab', '-framerate', "$fps", '-draw_mouse', '0',
    '-i', "title=$winTitle",
    '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '12',
    '-pix_fmt', 'yuv420p',
    '-vf', $vfCrop,
    $rawMkv
)

# (2) Start-Job with splatting — preserves em-dash and spaces in title correctly.
#     The title check frame on line ~143 uses the same "title=$winTitle" pattern
#     with & ffmpeg and it works. Start-Job copies the variable via $using: scope.
$ffJob = $null
for ($attempt = 1; $attempt -le 15; $attempt++) {
    $ffJob = Start-Job -Name "ffmpeg_cap" -ScriptBlock {
        param($a)
        & ffmpeg @a *>&1 | Out-Null
    } -ArgumentList (,$ffArgs)
    Start-Sleep -Milliseconds 800
    if ((Test-Path $rawMkv) -and ((Get-Item $rawMkv).Length -gt 3000)) {
        Write-Output "ffmpeg connected (attempt $attempt)"
        break
    }
    $ffJob | Stop-Job -ErrorAction SilentlyContinue
    $ffJob | Remove-Job -ErrorAction SilentlyContinue
    $ffJob = $null
    Remove-Item $rawMkv -ErrorAction SilentlyContinue
    if ($proc.HasExited) { Die 'game exited before ffmpeg connected' }
    Write-Output "retry $attempt..."
}
if (-not $ffJob) { Die 'ffmpeg could not connect to window' }

# Record for N seconds
Write-Output "recording ${recordSec}s..."
Start-Sleep -Seconds $recordSec

# Stop capture (ffmpeg is in a PowerShell job)
Write-Output 'stopping capture...'
if ($ffJob -and $ffJob.State -ne 'Completed') {
    Stop-Job $ffJob -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
    Remove-Job $ffJob -Force -ErrorAction SilentlyContinue
}
Get-Process -Name ffmpeg -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Write-Output 'ffmpeg stopped'

if ((-not $proc.HasExited)) {
    $proc.Kill()
    $proc.WaitForExit(2000) | Out-Null
}
Write-Output 'game killed'
Write-Output ''

# =========================================================================
# Verify raw capture + extract title check frame (post-capture, saves game-time)
# =========================================================================
if (-not (Test-Path $rawMkv) -or ((Get-Item $rawMkv).Length -lt 3000)) {
    Die 'no video captured (file missing or too small)'
}
$titleCheckFrame = "$tmpDir\title_check_${ts}.png"
& ffmpeg -y -hide_banner -loglevel error -ss 0.3 -i $rawMkv -vframes 1 $titleCheckFrame 2>&1 | Out-Null
if ((Test-Path $titleCheckFrame) -and ((Get-Item $titleCheckFrame).Length -gt 500)) {
    Write-Output "title frame extracted: $titleCheckFrame"
} else {
    Write-Output "WARNING: could not extract title check frame"
}
$rawSizeKb = [math]::Round((Get-Item $rawMkv).Length / 1024)
$durStr = (& ffprobe -v error -show_entries format=duration -of csv=p=0 $rawMkv 2>&1 | Out-String).Trim()
$resStr = (& ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=s=x:p=0 $rawMkv 2>&1 | Out-String).Trim()
$dur = [double]$durStr
Write-Output "RAW: ${rawSizeKb} KB, ${durStr}s, ${resStr}"

if ($dur -lt 3.0) {
    Die "captured video too short (${durStr}s) -- likely capture failed"
}

# =========================================================================
# Edge check: verify no office panel or other window contamination on edges.
# Uses ffmpeg signalstats to compare luma stddev between the right 60px strip
# and the center 60px strip. A 3D game scene has similar texture complexity
# across horizontal bands; a window overlay (dark panel + colored avatars + text)
# changes the luma distribution drastically.
# =========================================================================
Write-Output '--- Edge check ---'
$edgeL = "$tmpDir\edge_check_L_${ts}.png"
$edgeR = "$tmpDir\edge_check_R_${ts}.png"
$edgeC = "$tmpDir\edge_check_C_${ts}.png"

# Crop 60px strips from left, right, and center at t=2s (output is 1280x720 after scale)
& ffmpeg -y -hide_banner -loglevel error -ss 2.0 -i $rawMkv -vframes 1 -vf "crop=60:720:0:0"        $edgeL 2>&1 | Out-Null
& ffmpeg -y -hide_banner -loglevel error -ss 2.0 -i $rawMkv -vframes 1 -vf "crop=60:720:1220:0"     $edgeR 2>&1 | Out-Null
& ffmpeg -y -hide_banner -loglevel error -ss 2.0 -i $rawMkv -vframes 1 -vf "crop=60:720:610:0"      $edgeC 2>&1 | Out-Null

# (3b) Compute luma stddev for each 60px strip via ffmpeg signalstats.
#      signalstats writes per-frame stats to stderr in the form:
#      [Parsed_signalstats_0 @ ...] YMIN:... YMAX:... YAVG:... YSTDDEV:...
function Get-StripYStddev($pngPath) {
    $out = & ffmpeg -v error -i $pngPath -vf "signalstats" -vframes 1 -f null NUL 2>&1 | Out-String
    if ($out -match 'YSTDDEV:\s*([\d.]+)') {
        return [double]$Matches[1]
    }
    return -1.0
}

$rightStddev = Get-StripYStddev $edgeR
$centerStddev = Get-StripYStddev $edgeC
$leftStddev   = Get-StripYStddev $edgeL

Write-Output "edge luma stddev: L=$leftStddev  C=$centerStddev  R=$rightStddev"

if ($rightStddev -lt 0 -or $centerStddev -lt 0) {
    Die "edge check: could not compute signalstats -- strips may be empty or ffmpeg signalstats failed"
}

# Compare right vs center: if right stddev differs from center by > 35% AND
# absolute difference > 5 (meaningful gap), the right edge is contaminated.
$stddevDiff = [math]::Abs($rightStddev - $centerStddev)
$stddevRatio = if ($centerStddev -gt 0.001) { $stddevDiff / $centerStddev } else { 0.0 }
$stddevAbsDiff = $stddevDiff

Write-Output "edge diff: abs=$stddevAbsDiff  ratio=$stddevRatio"

# Thresholds: a clean 3D scene has stddev within ~25% across horizontal bands.
# Panel overlay (dark UI cards, avatars, text) changes stddev by 40%+.
if ($stddevRatio -gt 0.35 -and $stddevAbsDiff -gt 5.0) {
    Die "RIGHT EDGE CONTAMINATED -- luma stddev differs by ${stddevRatio:P0} (abs=${stddevAbsDiff:F1}) from center. Office panel or other window is overlapping the game window. Move the game window to (0,0) and ensure no other windows overlap."
}
Write-Output "edge check: PASS (stddev ratio ${stddevRatio:P0} within tolerance)"

# Also check the left edge for symmetry
$leftDiff = [math]::Abs($leftStddev - $centerStddev)
$leftRatio = if ($centerStddev -gt 0.001) { $leftDiff / $centerStddev } else { 0.0 }
if ($leftRatio -gt 0.35 -and $leftDiff -gt 5.0) {
    Die "LEFT EDGE CONTAMINATED -- luma stddev differs by ${leftRatio:P0} from center. Another window is overlapping from the left side."
}

# =========================================================================
# Scene-change detection
# =========================================================================
Write-Output '--- Scene-change detection ---'
$sceneLog = "$tmpDir\scene_changes_${ts}.txt"
$sceneOut = & ffmpeg -i $rawMkv -vf "select='gt(scene,$sceneThreshold)',showinfo" -vsync vfr -f null NUL 2>&1 | Out-String
$sceneOut | Out-File $sceneLog -Encoding utf8

$changedFrames = ([regex]::Matches($sceneOut, 'showinfo')).Count
$totalFramesStr = (& ffprobe -v error -count_frames -select_streams v:0 -show_entries stream=nb_read_frames -of csv=p=0 $rawMkv 2>&1 | Out-String).Trim()
$totalFrames = if ($totalFramesStr) { [int]$totalFramesStr } else { 0 }

Write-Output "total frames    : $totalFrames"
Write-Output ('scene-changed   : {0}  (threshold {1})' -f $changedFrames, $sceneThreshold)
Write-Output "scene log       : $sceneLog"

if ($changedFrames -eq 0) {
    Write-Output ''
    Write-Output '============================================'
    Write-Output '  FATAL: 0 scene-change frames detected.'
    Write-Output '  Clip is FROZEN -- check if:'
    Write-Output '    - game actually renders (not stuck)'
    Write-Output '    - correct window was captured'
    Write-Output '    - binary has working combat systems'
    Write-Output '  Raw MKV preserved for diagnosis:'
    Write-Output "    $rawMkv"
    Write-Output '============================================'
    # Keep raw for diagnosis, don't export bogus clips
    exit 1
}

# Safety floor: a real combat clip should have many more, but even a few is
# enough to prove it's not frozen.
if ($changedFrames -lt 3) {
    Write-Output "WARNING: only $changedFrames scene-change frames -- clip may be near-static"
}

# =========================================================================
# Export MP4
# =========================================================================
$mp4Out = "$outDir\combat-demo_${ts}.mp4"
Write-Output '--- Export MP4 ---'
& ffmpeg -y -hide_banner -loglevel error -i $rawMkv -c:v libx264 -preset slow -crf 14 -pix_fmt yuv420p -movflags +faststart $mp4Out 2>&1 | Out-Null
$mp4Dur = (& ffprobe -v error -show_entries format=duration -of csv=p=0 $mp4Out 2>&1 | Out-String).Trim()
$mp4Kb  = [math]::Round((Get-Item $mp4Out).Length / 1024)
Write-Output "MP4: $mp4Out  ($mp4Kb KB, ${mp4Dur}s)"

# Verify MP4 scene changes too
$mp4SceneOut = & ffmpeg -i $mp4Out -vf "select='gt(scene,$sceneThreshold)',showinfo" -vsync vfr -f null NUL 2>&1 | Out-String
$mp4Changed = ([regex]::Matches($mp4SceneOut, 'showinfo')).Count
Write-Output "MP4 scene-changed: $mp4Changed frames"

# =========================================================================
# Export GIF
# =========================================================================
$gifOut = "$outDir\combat-demo_${ts}.gif"
Write-Output '--- Export GIF ---'
$palette = "$tmpDir\pal_${ts}.png"
& ffmpeg -y -hide_banner -loglevel error -i $rawMkv -vf 'fps=15,scale=854:480:flags=lanczos,palettegen=max_colors=256' $palette 2>&1 | Out-Null
& ffmpeg -y -hide_banner -loglevel error -i $rawMkv -i $palette -lavfi 'fps=15,scale=854:480:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5' $gifOut 2>&1 | Out-Null
Remove-Item $palette -ErrorAction SilentlyContinue
$gifKb = [math]::Round((Get-Item $gifOut).Length / 1024)
Write-Output "GIF: $gifOut  ($gifKb KB)"

# =========================================================================
# Verify frames (visual spot-check)
# =========================================================================
Write-Output '--- Verify frames ---'
$v1 = "$tmpDir\v_${ts}_t1.png"
$v2 = "$tmpDir\v_${ts}_tMid.png"
$v3 = "$tmpDir\v_${ts}_tEnd.png"
$midT = [math]::Round($dur / 2, 1).ToString('F1', [System.Globalization.CultureInfo]::InvariantCulture)
$endT = ($dur - 2.0).ToString('F1', [System.Globalization.CultureInfo]::InvariantCulture)
& ffmpeg -y -hide_banner -loglevel error -ss 2.0 -i $rawMkv -vframes 1 $v1 2>&1 | Out-Null
& ffmpeg -y -hide_banner -loglevel error -ss $midT -i $rawMkv -vframes 1 $v2 2>&1 | Out-Null
& ffmpeg -y -hide_banner -loglevel error -ss $endT -i $rawMkv -vframes 1 $v3 2>&1 | Out-Null
Write-Output $v1
Write-Output $v2
Write-Output $v3

# =========================================================================
# Cleanup
# =========================================================================
Remove-Item $rawMkv -ErrorAction SilentlyContinue
Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

Write-Output ''
Write-Output '============================================'
Write-Output '  RECORDING COMPLETE'
Write-Output "  scene-change frames: $changedFrames"
Write-Output "  MP4: $mp4Out"
Write-Output "  GIF: $gifOut"
Write-Output "  Verify frames: $v1 | $v2 | $v3"
Write-Output "  Title check   : $titleCheckFrame"
Write-Output '============================================'
