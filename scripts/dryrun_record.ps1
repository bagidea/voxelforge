# dryrun_record.ps1 -- PROVE the harness end-to-end in pure PowerShell.
# The voxelforge window title contains an em-dash (U+2014) that does NOT survive
# the Bash->PowerShell->Bash pipeline on Windows. By keeping title, FindWindow,
# and ffmpeg all inside ONE PowerShell process, we avoid encoding loss.
#
# NO cargo. Use existing exe. Throwaway clip. Keep only PNGs.

$ErrorActionPreference = "Stop"

# config
$projectDir = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
$exe = "$projectDir\target-combat\debug\voxelforge.exe"
$outputDir = "$projectDir\_combat_recordings"
$ts = "dryrun-" + (Get-Date -Format "yyyyMMdd-HHmmss")
$rawMkv = "$outputDir\raw_${ts}.mkv"
$logFile = "$outputDir\dryrun_${ts}.log"
$fps = 15
$gameTimeout = 35
$ffmpegTimeout = 40
$windowPollMax = 20

if (-not (Test-Path $outputDir)) { New-Item -ItemType Directory $outputDir -Force | Out-Null }

# P/Invoke helpers
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class W32Dry {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern IntPtr FindWindowW(string lpClassName, string lpWindowName);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern int GetWindowTextW(IntPtr h, StringBuilder t, int m);
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pId);
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWin cb, IntPtr lp);
    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr h, out RECT rect);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public delegate bool EnumWin(IntPtr hWnd, IntPtr lp);
}
"@

# helpers
function Resolve-VoxelforgeTitle {
    $proc.Refresh()
    $t = $proc.MainWindowTitle
    if ($t -and $t.Length -gt 0) { return $t }
    return $null
}

function Die($msg) {
    Write-Output "FATAL: $msg"
    Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force
    exit 1
}

# pre-flight
Write-Output "exe: $exe ($((Get-Item $exe).Length) bytes)"
Write-Output "output: $outputDir"
Write-Output ""

# Kill stale
$stale = Get-Process -Name voxelforge -ErrorAction SilentlyContinue
if ($stale) {
    Write-Output "killing stale voxelforge.exe..."
    $stale | Stop-Process -Force
    Start-Sleep 1
}
Write-Output "no stale voxelforge [check]"
Write-Output ""

# ---- PHASE 1: Launch game ----
Write-Output "=== LAUNCH GAME ==="
Write-Output "cmd: $exe --play"
$proc = Start-Process -FilePath $exe -ArgumentList "--play" -PassThru -WindowStyle Normal
$gamePid = $proc.Id
Write-Output "game PID: $gamePid"
Write-Output "polling window every 0.5s (max ${windowPollMax}s)..."

$winTitle = $null
for ($i = 0; $i -lt ($windowPollMax * 2); $i++) {
    Start-Sleep -Milliseconds 500
    $winTitle = Resolve-VoxelforgeTitle
    if ($winTitle) {
        Write-Output ""
        $elapsed = [math]::Round(($i + 1) * 0.5, 1)
        Write-Output ">>> WINDOW FOUND after ${elapsed}s"
        Write-Output ">>> Title: '${winTitle}'"
        Write-Output ">>> settle delay 3s..."
        Start-Sleep -Seconds 3
        break
    }
    Write-Host -NoNewline "."
    if ($proc.HasExited) {
        Write-Output ""
        Write-Output "WARNING: game exited before window appeared"
        break
    }
}
Write-Output ""

if (-not $winTitle) {
    Die "could not find voxelforge window in ${windowPollMax}s"
}

# ---- PHASE 2: Sanity check FindWindowW ----
$hwnd = [W32Dry]::FindWindowW([NullString]::Value, $winTitle)
if ($hwnd -eq [IntPtr]::Zero) {
    Die "FindWindowW returned NULL - window gone?"
}
Write-Output "FindWindowW verified: HWND=$hwnd"

# ---- PHASE 3: ffmpeg capture via cmd /c (preserves em-dash in title) ----
Write-Output "=== RECORD ==="
Write-Output "capturing: '${winTitle}' @ ${fps}fps"
Write-Output ""

# Build command with native Windows quoting: double-quotes around title value
$cmdLine = "ffmpeg -y -hide_banner -loglevel error -f gdigrab -framerate $fps -draw_mouse 0 -i `"title=$winTitle`" -c:v libx264 -preset ultrafast -crf 23 -pix_fmt yuv420p `"$rawMkv`""

Write-Output "cmd: $cmdLine"

$ffProc = $null
for ($attempt = 1; $attempt -le 10; $attempt++) {
    $ffProc = Start-Process -FilePath "cmd" -ArgumentList "/c", $cmdLine -NoNewWindow -PassThru

    Start-Sleep -Milliseconds 1000

    if (-not $ffProc.HasExited) {
        if ((Test-Path $rawMkv) -and ((Get-Item $rawMkv).Length -gt 1000)) {
            Write-Output "ffmpeg connected on attempt ${attempt}, PID=$($ffProc.Id)"
            break
        }
    }

    # ffmpeg exited — try reading its output
    $ffProc.WaitForExit(500) | Out-Null
    $ffProc = $null
    Remove-Item $rawMkv -ErrorAction SilentlyContinue

    if ($proc.HasExited) {
        Write-Output "game exited during retry"
        break
    }
    Write-Output ("attempt ${attempt}: retrying...")
    $ffProc = $null
}

if (-not $ffProc) {
    Die "ffmpeg could not connect to window"
}

Write-Output "ffmpeg started, PID=$($ffProc.Id)"

# ---- PHASE 4: Record for N seconds, then kill game + ffmpeg ----
$recordDuration = 8
Write-Output "=== RECORDING (${recordDuration}s) ==="
Start-Sleep -Seconds $recordDuration
Write-Output "stopping capture..."

if ($ffProc -and (-not $ffProc.HasExited)) {
    $ffProc.Kill()
    $ffProc.WaitForExit(2000) | Out-Null
}
Write-Output "ffmpeg stopped"

if (-not $proc.HasExited) {
    $proc.Kill()
    $proc.WaitForExit(2000) | Out-Null
}
Write-Output "game killed"
Write-Output ""

# ---- PHASE 5: Verify capture ----
if (-not (Test-Path $rawMkv) -or ((Get-Item $rawMkv).Length -eq 0)) {
    Die "no video captured ($rawMkv missing or empty)"
}

$rawSize = (Get-Item $rawMkv).Length
$durStr = & ffprobe -v error -show_entries format=duration -of csv=p=0 $rawMkv 2>&1
$resStr = & ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=s=x:p=0 $rawMkv 2>&1
$dur = [double]::Parse($durStr.Trim())
$res = $resStr.Trim()
$width, $height = $res -split 'x'

Write-Output "============================================"
Write-Output "  CAPTURE OK: ${rawSize} bytes, ${dur}s"
Write-Output "  RESOLUTION: ${res}"
Write-Output "============================================"
Write-Output ""

# ---- PHASE 6: Extract verification frames ----
Write-Output "=== VERIFY FRAMES ==="
$verifyFirst = "$outputDir\dryrun_${ts}_first.png"
$verifyMid   = "$outputDir\dryrun_${ts}_mid.png"
$verifyLast  = "$outputDir\dryrun_${ts}_last.png"

$firstSS = if ($dur -gt 1.0) { "0.5" } else { "0.1" }
& ffmpeg -y -hide_banner -loglevel error -ss $firstSS -i $rawMkv -vframes 1 $verifyFirst 2>&1 | Out-Null
Write-Output "  [check] first (t=${firstSS}s): $verifyFirst"

$midSS = [math]::Round($dur / 2.0, 1).ToString("F1", [System.Globalization.CultureInfo]::InvariantCulture)
& ffmpeg -y -hide_banner -loglevel error -ss $midSS -i $rawMkv -vframes 1 $verifyMid 2>&1 | Out-Null
Write-Output "  [check] mid   (t=${midSS}s): $verifyMid"

$lastT = $dur - 0.5
if ($lastT -lt 0.3) { $lastT = 0.3 }
$lastSS = [math]::Round($lastT, 1).ToString("F1", [System.Globalization.CultureInfo]::InvariantCulture)
& ffmpeg -y -hide_banner -loglevel error -ss $lastSS -i $rawMkv -vframes 1 $verifyLast 2>&1 | Out-Null
Write-Output "  [check] last  (t=${lastSS}s): $verifyLast"
Write-Output ""

# ---- PHASE 7: Delete clip, keep PNGs ----
Write-Output "=== CLEANUP ==="
Remove-Item $rawMkv -Force
Write-Output "  removed MKV: $rawMkv"
Write-Output ""

# ---- PHASE 8: Kill lingering process ----
Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 1
$hanging = Get-Process -Name voxelforge -ErrorAction SilentlyContinue
if ($hanging) {
    $hanging | Stop-Process -Force
    Write-Output "WARNING: lingering voxelforge.exe killed"
}
else {
    Write-Output "clean: no voxelforge.exe [check]"
}

Write-Output ""
Write-Output "============================================"
Write-Output "  DRY-RUN COMPLETE"
Write-Output "  Window title : ${winTitle}"
Write-Output "  Resolution   : ${res}"
Write-Output "  Duration     : ${dur}s"
Write-Output ""
Write-Output "  Verify PNGs:"
Write-Output "    ${verifyFirst}"
Write-Output "    ${verifyMid}"
Write-Output "    ${verifyLast}"
Write-Output "============================================"
Write-Output ""
Write-Output "Clip deleted. PNGs kept for CEO review."