# _yama_feeldemo_before_capture.ps1
#
# Records the "before" clip for the player_tuning.rs feel refactor (accel/decel
# curves, coyote time, jump buffer, spring-damped camera boom, speed FOV) using
# the OLD exe at target-yamamoto/release/voxelforge.exe (built 2026-08-20 21:09,
# predates the refactor -- confirmed via mtime vs client/src/main.rs and
# client/src/player_tuning.rs, both edited 2026-08-21 ~02:xx).
#
# That old exe has NO feel_demo/FEEL_DEMO_FILM auto-play+capture harness (that
# code was added in the same uncommitted diff as the refactor it's meant to
# prove -- it doesn't exist in this binary yet). So this script fakes the same
# walk->run->jump->land timeline via real OS-level key events (SendInput --
# posts actual hardware-equivalent input, unlike SendKeys, so winit sees it the
# same as a real keypress and can hold keys for a duration).
#
# CAPTURE METHOD: ffmpeg's `ddagrab` lavfi source (DXGI Desktop Duplication),
# NOT gdigrab. Proven by direct pixel inspection this session: gdigrab (both
# title= and desktop-region BitBlt) grabbed a frozen/blank white frame for this
# window's whole client area across its entire capture -- Bevy's Vulkan
# swapchain presents outside the path BitBlt reads, a known gdigrab blind spot
# for hardware-accelerated windows. ddagrab reads the DWM-composited surface
# instead and captured real, moving content on the first try. It also doesn't
# need title= matching, so the window title's em-dash (U+2014) never enters the
# picture (see ps51-utf8-emdash-breaks-scripts memory -- that whole class of
# bug is moot here).
#
# The window is moved to (0,0) before capture (SetWindowPos) so the fixed
# capture rectangle doesn't overlap the office UI panel docked on the right
# edge of the screen -- confirmed by direct frame inspection this session that
# capturing in-place bled the panel into the right ~70px of the frame.
#
# Beats mirror feel_demo_input's offsets exactly (elapsed 0.5-2.5s walk,
# 2.5-4.0s run, 4.0-4.2s jump tap, 4.2-6.5s walk resumed) so the after clip
# (in-engine VOXELFORGE_FEEL_DEMO_FILM capture, same timeline) lines up.
#
# Output: _yama_feeldemo/before_<ts>.mp4

$ErrorActionPreference = "Stop"
$ProjectDir = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $ProjectDir

$Exe = Join-Path $ProjectDir "target-yamamoto\release\voxelforge.exe"
$Out = Join-Path $ProjectDir "_yama_feeldemo"
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Ts = Get-Date -Format "yyyyMMdd-HHmmss"
$Log = Join-Path $Out "before_$Ts.log"
$Mp4 = Join-Path $Out "before_$Ts.mp4"
$FPS = 30

if (-not (Test-Path $Exe)) { throw "exe not found: $Exe" }

function Die($msg) {
    Write-Output "FATAL: $msg"
    Get-Process -Name ffmpeg -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    throw $msg
}

# --- SendInput (real hardware-equivalent key events) + window placement/rect probe ---
Add-Type @'
using System;
using System.Runtime.InteropServices;
public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
public static class Kb {
    [StructLayout(LayoutKind.Sequential)]
    struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)]
    struct INPUT { public uint type; public KEYBDINPUT ki; public long padding; }
    [DllImport("user32.dll", SetLastError = true)]
    static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int X, int Y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr hWnd, ref System.Drawing.Point pt);
    const uint INPUT_KEYBOARD = 1;
    const uint KEYEVENTF_KEYUP = 0x0002;
    const uint SWP_NOSIZE = 0x0001;
    const uint SWP_NOZORDER = 0x0004;
    const uint SWP_SHOWWINDOW = 0x0040;
    static readonly IntPtr HWND_TOPMOST = new IntPtr(-1);
    static readonly IntPtr HWND_NOTOPMOST = new IntPtr(-2);
    const ushort VK_MENU = 0x12; // Alt
    public static void KeyDown(ushort vk) {
        var i = new INPUT[1]; i[0].type = INPUT_KEYBOARD; i[0].ki.wVk = vk;
        SendInput(1, i, Marshal.SizeOf(typeof(INPUT)));
    }
    public static void KeyUp(ushort vk) {
        var i = new INPUT[1]; i[0].type = INPUT_KEYBOARD; i[0].ki.wVk = vk; i[0].ki.dwFlags = KEYEVENTF_KEYUP;
        SendInput(1, i, Marshal.SizeOf(typeof(INPUT)));
    }
    // A shared desktop can have another window (e.g. a maximized browser) as
    // the real foreground app; SetForegroundWindow from an unrelated process
    // is silently denied by Windows' anti-focus-stealing lock in that case.
    // Tapping Alt via SendInput first counts as "this process generated the
    // most recent real input", which satisfies the condition Windows checks
    // before honoring SetForegroundWindow from a background process.
    public static bool Focus(IntPtr hWnd) {
        KeyDown(VK_MENU); KeyUp(VK_MENU);
        return SetForegroundWindow(hWnd);
    }
    public static void MoveToOrigin(IntPtr hWnd) {
        SetWindowPos(hWnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOSIZE | SWP_SHOWWINDOW);
    }
    public static void ClearTopmost(IntPtr hWnd) {
        SetWindowPos(hWnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOSIZE | SWP_SHOWWINDOW);
    }
    public static bool IsForeground(IntPtr hWnd) { return GetForegroundWindow() == hWnd; }
    public static RECT ClientRectOnScreen(IntPtr hWnd) {
        RECT cr; GetClientRect(hWnd, out cr);
        var topLeft = new System.Drawing.Point(0, 0);
        ClientToScreen(hWnd, ref topLeft);
        return new RECT { Left = topLeft.X, Top = topLeft.Y, Right = topLeft.X + cr.Right, Bottom = topLeft.Y + cr.Bottom };
    }
}
'@ -ReferencedAssemblies System.Drawing

$VK_W = 0x57
$VK_CTRL = 0xA2   # left control
$VK_SPACE = 0x20

# --- kill any stale instance ---
Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

# --- launch game (plain --play, manual input) ---
Write-Output "=== LAUNCH (before exe, --play) ==="
$gameProc = Start-Process -FilePath $Exe -ArgumentList "--play" -PassThru -RedirectStandardOutput $Log -RedirectStandardError "$Log.err"
Write-Output "game PID: $($gameProc.Id)"

# --- wait for window ---
Write-Output "waiting for window..."
# Sun's cargo build has 4 rustc processes going right now (one at 2.3GB RSS) --
# window creation itself is getting starved by that CPU/RAM contention, so
# poll generously rather than the usual ~5-10s.
$proc = $null
for ($i = 0; $i -lt 60; $i++) {
    Start-Sleep -Seconds 1
    $proc = Get-Process -Id $gameProc.Id -ErrorAction SilentlyContinue
    if ($proc -and $proc.MainWindowTitle -and $proc.MainWindowHandle -ne [IntPtr]::Zero) {
        Write-Output "window found after ${i}s: `"$($proc.MainWindowTitle)`""
        break
    }
    if (-not $proc) { throw "game exited before window appeared -- check $Log" }
}
if (-not $proc -or -not $proc.MainWindowTitle) { throw "could not resolve game window" }

$EM = [char]0x2014
$expectedPrefix = "Voxelforge ${EM} Phase 0 spike"
if ($proc.MainWindowTitle -notmatch "^$([regex]::Escape($expectedPrefix))") {
    Die "WRONG WINDOW -- expected title starting with '$expectedPrefix', got '$($proc.MainWindowTitle)'"
}
$hwnd = $proc.MainWindowHandle
Write-Output "title check: PASS ('$($proc.MainWindowTitle)')"

Start-Sleep -Milliseconds 500
[Kb]::MoveToOrigin($hwnd)
Start-Sleep -Milliseconds 300
[Kb]::Focus($hwnd)
Start-Sleep -Milliseconds 200

# This machine's desktop is shared -- whatever the CEO or another agent has
# open (a maximized browser, in one earlier run) can sit in front of / steal
# focus from the game window, silently wrecking both the capture (wrong
# pixels) and the input (keys land on the wrong window). Topmost + Alt-tap
# focus (above) should prevent it; verify it actually landed before trusting
# any of the recording that follows.
if (-not [Kb]::IsForeground($hwnd)) {
    Die "game window did NOT become foreground (something else is stealing focus on this desktop) -- capture would record the wrong window"
}
Write-Output "foreground check: PASS (game window is in front)"

# Under Sun's build contention, chunk streaming/meshing is starved too: a
# spot-check capture came back with a real HUD ("chunks 12", avatar falling
# through unrendered terrain) but a blank white/sky world -- not a capture
# bug, the world just hadn't finished loading in the first ~7s after spawn.
# Warm up here, BEFORE recording starts, so the clip itself isn't padded
# with empty-world seconds.
Write-Output "=== WORLD WARM-UP (build-lane contention headroom) ==="
Start-Sleep -Seconds 15
[Kb]::Focus($hwnd) | Out-Null
Start-Sleep -Milliseconds 200
if (-not [Kb]::IsForeground($hwnd)) { Die "lost foreground during world warm-up -- another window took over" }

$cliRect = [Kb]::ClientRectOnScreen($hwnd)
$cliW = $cliRect.Right - $cliRect.Left
$cliH = $cliRect.Bottom - $cliRect.Top
if ($cliW % 2 -ne 0) { $cliW -= 1 }
if ($cliH % 2 -ne 0) { $cliH -= 1 }
Write-Output "client area (after move-to-origin): offset=($($cliRect.Left),$($cliRect.Top)) size=${cliW}x${cliH}"

# --- start ffmpeg ddagrab (DXGI Desktop Duplication -- reads the real
#     compositor surface, unlike gdigrab's BitBlt which came back blank for
#     this window in testing) ---
Write-Output "=== RECORD (ffmpeg ddagrab @ ${FPS}fps) ==="
$ddaSrc = "ddagrab=framerate=${FPS}:offset_x=$($cliRect.Left):offset_y=$($cliRect.Top):video_size=${cliW}x${cliH}"
$ffArgs = @(
    "-y", "-hide_banner", "-loglevel", "warning",
    "-f", "lavfi", "-i", $ddaSrc,
    "-vf", "hwdownload,format=bgra",
    "-c:v", "libx264", "-preset", "ultrafast", "-crf", "18", "-pix_fmt", "yuv420p",
    "$Mp4"
)
$FfLog = Join-Path $Out "before_${Ts}_ffmpeg.log"
# Raw Process (not Start-Process) so StandardInput is a live writable pipe --
# needed to send ffmpeg a graceful 'q' at the end (a hard kill left a previous
# gdigrab capture with a broken duration/index; 'q' lets the mp4 muxer finalize).
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = "ffmpeg"
# ArgumentList (Collection<string>) isn't available on this PS5.1 / .NET
# Framework combo -- build a classic quoted command-line string instead. None
# of these args contain non-ASCII (ddagrab has no title= to mangle), so simple
# double-quoting is safe here.
$psi.Arguments = ($ffArgs | ForEach-Object { '"' + ($_ -replace '"', '\"') + '"' }) -join ' '
$psi.RedirectStandardInput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true
$ffProc = [System.Diagnostics.Process]::Start($psi)
$ffErrTask = $ffProc.StandardError.ReadToEndAsync()
Write-Output "ffmpeg PID: $($ffProc.Id)"
Start-Sleep -Milliseconds 500
if ($ffProc.HasExited) {
    $ffErrTask.Result | Out-File $FfLog -Encoding utf8
    Die "ffmpeg exited immediately (code $($ffProc.ExitCode)) -- see $FfLog"
}

[Kb]::Focus($hwnd)

function Assert-Foreground($beat) {
    if (-not [Kb]::IsForeground($hwnd)) {
        [Kb]::Focus($hwnd)
        Start-Sleep -Milliseconds 100
        if (-not [Kb]::IsForeground($hwnd)) {
            Die "lost foreground focus at beat '$beat' -- another window stole it mid-recording, clip is contaminated"
        }
    }
}

# --- scripted timeline, mirrors feel_demo_input's offsets ---
Write-Output "=== INPUT TIMELINE ==="
Write-Output "t=0.0 idle (settle)"
Start-Sleep -Milliseconds 500

Assert-Foreground "walk"
Write-Output "t=0.5 walk: W down"
[Kb]::KeyDown($VK_W)
Start-Sleep -Milliseconds 2000

Assert-Foreground "run"
Write-Output "t=2.5 run: Ctrl down (W still held)"
[Kb]::KeyDown($VK_CTRL)
Start-Sleep -Milliseconds 1500

Assert-Foreground "jump"
Write-Output "t=4.0 jump: Space tap (W+Ctrl still held)"
[Kb]::KeyDown($VK_SPACE)
Start-Sleep -Milliseconds 150
[Kb]::KeyUp($VK_SPACE)
Start-Sleep -Milliseconds 50

Write-Output "t=4.2 walk resumed: Ctrl up (W still held)"
[Kb]::KeyUp($VK_CTRL)
Start-Sleep -Milliseconds 2300

Assert-Foreground "walk_resumed"
Write-Output "t=6.5 stop: W up"
[Kb]::KeyUp($VK_W)
Start-Sleep -Milliseconds 500

# --- stop capture gracefully (q on stdin lets the mp4 muxer finalize
#     properly) then stop the game ---
Write-Output "=== STOP ==="
try {
    $ffProc.StandardInput.Write("q")
    $ffProc.StandardInput.Flush()
    $ffProc.WaitForExit(5000) | Out-Null
} catch {}
if (-not $ffProc.HasExited) { Stop-Process -Id $ffProc.Id -Force -ErrorAction SilentlyContinue }
if (-not $proc.HasExited) { $proc.Kill(); $proc.WaitForExit(2000) | Out-Null }

if (-not (Test-Path $Mp4) -or (Get-Item $Mp4).Length -lt 3000) { Die "no video captured: $Mp4" }
$sizeKb = [math]::Round((Get-Item $Mp4).Length / 1KB, 1)
Write-Output "capture: $Mp4 (${sizeKb} KB)"

# --- scene-change check: prove it's not a frozen/blank clip ---
# ffmpeg writes its normal progress/showinfo chatter to stderr; PS 5.1 wraps
# every such line in a NativeCommandError under $ErrorActionPreference=Stop,
# so run this one call with EAP relaxed instead of merging 2>&1.
$prevEap = $ErrorActionPreference
$ErrorActionPreference = "Continue"
$sceneOut = (& ffmpeg -i $Mp4 -vf "select='gt(scene,0.003)',showinfo" -vsync vfr -f null NUL) 2>&1 | Out-String
$ErrorActionPreference = $prevEap
$changed = ([regex]::Matches($sceneOut, "showinfo")).Count
Write-Output "scene-change frames: $changed"
if ($changed -eq 0) { Die "0 scene-change frames -- clip is frozen/blank, capture is not usable" }

Write-Output "log: $Log"
Write-Output "DONE ts=$Ts"
