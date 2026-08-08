# _poppy_cine_capture.ps1 -- record REAL gameplay from _poppy_cine.exe.
#
# ASCII ONLY. Windows PowerShell 5.1 reads this file as ANSI, so a UTF-8 em-dash
# inside a string literal comes back as mojibake and breaks the parser. It did.
#
# CAPTURE POLICY (inherits record_combat.sh's rules):
#   * The game starts FIRST; ffmpeg only starts once the window exists and its
#     rect is known, so no frame of desktop-only content is ever recorded.
#   * Capture is cropped to the game window's EXACT client rect, taken via
#     gdigrab desktop -- the DWM-composited path. "-i title=" BitBlts the window
#     DC, which comes back black for a wgpu/DX12 swapchain.
#   * The window is forced foreground + topmost for the duration, so nothing can
#     composite over the crop region.
#   * NO cargo. The binary is a frozen copy (_poppy_cine.exe) -- Flamingo holds
#     the build lock and is overwriting target/release/voxelforge.exe.
#
# Output: _poppy_cine/raw_<ts>.mkv  +  _poppy_cine/run_<ts>.log
param(
    [int]$Seconds = 26,
    [int]$Fps = 60,
    [string]$Mode = "--quest-demo"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$exe = Join-Path $root "_poppy_cine.exe"
if (-not (Test-Path $exe)) { throw "missing _poppy_cine.exe (cp target/release/voxelforge.exe _poppy_cine.exe)" }

$outDir = Join-Path $root "_poppy_cine"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$ts = Get-Date -Format "yyyyMMdd-HHmmss"
$raw = Join-Path $outDir "raw_$ts.mkv"
$log = Join-Path $outDir "run_$ts.log"

# ---- win32: find the render window, read its rect, pin it on top ----
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32 {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref System.Drawing.Point p);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowThreadProcessId(IntPtr h, out int pid);

  // Process.MainWindowHandle is NOT usable here: the process also owns a 16x16
  // visible helper window, and whichever exists first wins the race -- the first
  // capture attempt sat on that one and read a 0x0 client rect forever, which
  // fed ffmpeg "video_size 0x0". Pick by geometry instead.
  public static IntPtr FindRenderWindow(int pid, int minW, int minH) {
    IntPtr hit = IntPtr.Zero;
    EnumWindows(delegate(IntPtr h, IntPtr l) {
      int wpid; GetWindowThreadProcessId(h, out wpid);
      if (wpid != pid || !IsWindowVisible(h)) return true;
      RECT c; GetClientRect(h, out c);
      if (c.R - c.L >= minW && c.B - c.T >= minH) { hit = h; return false; }
      return true;
    }, IntPtr.Zero);
    return hit;
  }
}
"@ -ReferencedAssemblies System.Drawing

# ---- kill strays, launch ----
Get-Process -Name "_poppy_cine" -ErrorAction SilentlyContinue | Stop-Process -Force
$proc = Start-Process -FilePath $exe -ArgumentList $Mode -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err"

$hwnd = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(25)
while ((Get-Date) -lt $deadline) {
    if ($proc.HasExited) { throw "game exited before its window appeared, see $log" }
    $hwnd = [W32]::FindRenderWindow($proc.Id, 640, 360)
    if ($hwnd -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 50
}
if ($hwnd -eq [IntPtr]::Zero) { $proc | Stop-Process -Force; throw "no sized game window within 25s" }

# Park the window at the screen's top-left, then pin it topmost. The default
# centred position put the client's right edge under an always-on-top dock at
# screen x~1344, and those icons were baked into the smoke capture.
# HWND_TOPMOST(-1), SWP_NOSIZE(0x0001)
[W32]::SetWindowPos($hwnd, [IntPtr](-1), 0, 0, 0, 0, 0x0001) | Out-Null
[W32]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 200

$cr = New-Object W32+RECT
[W32]::GetClientRect($hwnd, [ref]$cr) | Out-Null
$origin = New-Object System.Drawing.Point 0, 0
[W32]::ClientToScreen($hwnd, [ref]$origin) | Out-Null
$w = $cr.R - $cr.L; $h = $cr.B - $cr.T
$w = $w - ($w % 2); $h = $h - ($h % 2)
$elapsed = ((Get-Date) - $proc.StartTime).TotalSeconds
"WINDOW hwnd=$hwnd client=${w}x${h} at ($($origin.X),$($origin.Y)) t_since_launch=$([math]::Round($elapsed,2))s"

# ---- record ----
$ff = @(
  "-hide_banner", "-loglevel", "warning",
  "-f", "gdigrab", "-framerate", "$Fps", "-draw_mouse", "0",
  "-offset_x", "$($origin.X)", "-offset_y", "$($origin.Y)",
  "-video_size", "${w}x${h}", "-i", "desktop",
  "-t", "$Seconds", "-c:v", "libx264", "-preset", "ultrafast", "-crf", "12",
  "-pix_fmt", "yuv444p", "-y", $raw
)
$t0 = Get-Date
& ffmpeg @ff
$rc = $LASTEXITCODE
$capSec = ((Get-Date) - $t0).TotalSeconds
"FFMPEG exit=$rc wall=$([math]::Round($capSec,2))s"

# ---- stop the game (quest_demo wedges on phase 4 by design; it never exits) ----
Start-Sleep -Milliseconds 300
Get-Process -Name "_poppy_cine" -ErrorAction SilentlyContinue | Stop-Process -Force
"GAME stopped."
"RAW=$raw"
"LOG=$log"
