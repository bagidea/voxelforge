# _poppy_beauty_capture.ps1 -- record REAL gameplay from the green flamingo build
# so beauty frames can be pulled out of it losslessly.
#
# ASCII ONLY. Windows PowerShell 5.1 reads this file as ANSI; a UTF-8 dash inside
# a string literal is a parse error.
#
# Same capture policy as _poppy_cine_capture.ps1:
#   * game starts first; ffmpeg only starts once the window rect is known
#   * gdigrab DESKTOP cropped to the client rect (title= BitBlt is black on wgpu)
#   * window parked at (0,0) + topmost so no dock composites into the crop
#   * NO cargo -- the binary is a frozen copy; Rose holds the build lane
#
# Lossless (ffv1/bgr0) because these frames get eyedropped by the look rubric.
param(
    [int]$Seconds = 30,
    [int]$Fps = 30,
    [string]$Mode = "--quest-demo",
    [string]$Tag = "quest",
    [int]$ClientW = 1600,   # the window is resizable, so the swapchain follows --
    [int]$ClientH = 900     # no need to stay at the hardcoded 1280x720
)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$exe = Join-Path $root "_poppy_beauty.exe"
if (-not (Test-Path $exe)) { throw "missing _poppy_beauty.exe (cp target-flamingo/release/voxelforge.exe _poppy_beauty.exe)" }

$outDir = Join-Path $root "_poppy_beauty"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$raw = Join-Path $outDir "raw_$Tag.mkv"
$log = Join-Path $outDir "run_$Tag.log"

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32B {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref System.Drawing.Point p);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowThreadProcessId(IntPtr h, out int pid);

  // Not Process.MainWindowHandle: the process also owns a 16x16 visible helper
  // whose client rect reads 0x0 forever. Pick the window by geometry.
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

Get-Process -Name "_poppy_beauty" -ErrorAction SilentlyContinue | Stop-Process -Force
$proc = Start-Process -FilePath $exe -ArgumentList $Mode -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err"

$hwnd = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(25)
while ((Get-Date) -lt $deadline) {
    if ($proc.HasExited) { throw "game exited before its window appeared, see $log" }
    $hwnd = [W32B]::FindRenderWindow($proc.Id, 640, 360)
    if ($hwnd -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 50
}
if ($hwnd -eq [IntPtr]::Zero) { $proc | Stop-Process -Force; throw "no sized game window within 25s" }

$wr = New-Object W32B+RECT
$c0 = New-Object W32B+RECT
[W32B]::GetWindowRect($hwnd, [ref]$wr) | Out-Null
[W32B]::GetClientRect($hwnd, [ref]$c0) | Out-Null
$chromeW = ($wr.R - $wr.L) - ($c0.R - $c0.L)
$chromeH = ($wr.B - $wr.T) - ($c0.B - $c0.T)
$scr = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$wantW = [Math]::Min($ClientW + $chromeW, $scr.Width)
$wantH = [Math]::Min($ClientH + $chromeH, $scr.Height)
# HWND_TOPMOST(-1), park at 0,0 and resize in one call
[W32B]::SetWindowPos($hwnd, [IntPtr](-1), 0, 0, $wantW, $wantH, 0) | Out-Null
[W32B]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 900

$cr = New-Object W32B+RECT
[W32B]::GetClientRect($hwnd, [ref]$cr) | Out-Null
$origin = New-Object System.Drawing.Point 0, 0
[W32B]::ClientToScreen($hwnd, [ref]$origin) | Out-Null
$w = $cr.R - $cr.L; $h = $cr.B - $cr.T
$w = $w - ($w % 2); $h = $h - ($h % 2)
$elapsed = ((Get-Date) - $proc.StartTime).TotalSeconds
"WINDOW hwnd=$hwnd client=${w}x${h} at ($($origin.X),$($origin.Y)) t_since_launch=$([math]::Round($elapsed,2))s"

$ff = @(
  "-hide_banner", "-loglevel", "warning",
  "-f", "gdigrab", "-framerate", "$Fps", "-draw_mouse", "0",
  "-offset_x", "$($origin.X)", "-offset_y", "$($origin.Y)",
  "-video_size", "${w}x${h}", "-i", "desktop",
  "-t", "$Seconds", "-c:v", "ffv1", "-pix_fmt", "bgr0", "-y", $raw
)
$t0 = Get-Date
& ffmpeg @ff
$rc = $LASTEXITCODE
"FFMPEG exit=$rc wall=$([math]::Round(((Get-Date) - $t0).TotalSeconds,2))s"

Start-Sleep -Milliseconds 300
Get-Process -Name "_poppy_beauty" -ErrorAction SilentlyContinue | Stop-Process -Force
"GAME stopped."
"RAW=$raw"
"LOG=$log"
