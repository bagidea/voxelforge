# _poppy_cine_shot.ps1 -- grab ONE frame from a live game window.
#
# Same capture path as _poppy_cine_capture.ps1 (gdigrab desktop cropped to the
# client rect; "-i title=" is black on a wgpu swapchain) but it writes a single
# PNG instead of a clip. This is the cheap verifier: does VOXELFORGE_NOHUD
# actually clear the screen on THIS binary, before spending 26s + an encode on
# a take that turns out to still have the "[E] ..." prompts in frame.
#
# ASCII ONLY -- PowerShell 5.1 reads this file as ANSI and a UTF-8 dash inside a
# string literal is a parse error.
#
# Usage:
#   powershell -File scripts/_poppy_cine_shot.ps1 -Exe target/release/voxelforge.exe `
#              -At 14 -Out _poppy_cine/check.png -Env "VOXELFORGE_NOHUD=1"
param(
    [string]$Exe = "target/release/voxelforge.exe",
    [string]$Mode = "--quest-demo",
    [int]$At = 14,
    [string]$Out = "_poppy_cine/shot.png",
    [string[]]$Env = @()
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$exePath = Join-Path $root $Exe
if (-not (Test-Path $exePath)) { throw "missing $exePath" }
$exeName = [System.IO.Path]::GetFileNameWithoutExtension($exePath)
$outPath = Join-Path $root $Out
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outPath) | Out-Null
$log = "$outPath.log"

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32Shot {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref System.Drawing.Point p);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowThreadProcessId(IntPtr h, out int pid);
  // Pick by geometry: the process also owns a 16x16 helper window and
  // MainWindowHandle picks whichever existed first (a 0x0 client rect forever).
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

foreach ($kv in $Env) {
    $i = $kv.IndexOf("=")
    if ($i -lt 1) { throw "bad -Env entry '$kv' (want NAME=value)" }
    Set-Item -Path ("env:" + $kv.Substring(0, $i)) -Value $kv.Substring($i + 1)
    "ENV $kv"
}

Get-Process -Name $exeName -ErrorAction SilentlyContinue | Stop-Process -Force
$proc = Start-Process -FilePath $exePath -ArgumentList $Mode -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err"

$hwnd = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(25)
while ((Get-Date) -lt $deadline) {
    if ($proc.HasExited) { throw "game exited before its window appeared, see $log" }
    $hwnd = [W32Shot]::FindRenderWindow($proc.Id, 640, 360)
    if ($hwnd -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 50
}
if ($hwnd -eq [IntPtr]::Zero) { $proc | Stop-Process -Force; throw "no sized game window within 25s" }

# HWND_TOPMOST(-1), SWP_NOSIZE(0x0001) -- park at (0,0) so no always-on-top dock
# composites over the crop region.
[W32Shot]::SetWindowPos($hwnd, [IntPtr](-1), 0, 0, 0, 0, 0x0001) | Out-Null
[W32Shot]::SetForegroundWindow($hwnd) | Out-Null

$wait = $At - ((Get-Date) - $proc.StartTime).TotalSeconds
if ($wait -gt 0) { Start-Sleep -Milliseconds ([int]($wait * 1000)) }

$cr = New-Object W32Shot+RECT
[W32Shot]::GetClientRect($hwnd, [ref]$cr) | Out-Null
$origin = New-Object System.Drawing.Point 0, 0
[W32Shot]::ClientToScreen($hwnd, [ref]$origin) | Out-Null
$w = $cr.R - $cr.L; $h = $cr.B - $cr.T
$w = $w - ($w % 2); $h = $h - ($h % 2)
"WINDOW client=${w}x${h} at ($($origin.X),$($origin.Y)) t=$([math]::Round(((Get-Date) - $proc.StartTime).TotalSeconds,2))s"

& ffmpeg -hide_banner -loglevel warning -f gdigrab -framerate 10 -draw_mouse 0 `
    -offset_x $origin.X -offset_y $origin.Y -video_size "${w}x${h}" -i desktop `
    -frames:v 1 -y $outPath
"FFMPEG exit=$LASTEXITCODE"

Start-Sleep -Milliseconds 200
Get-Process -Name $exeName -ErrorAction SilentlyContinue | Stop-Process -Force
"GAME stopped."
"OUT=$outPath"
