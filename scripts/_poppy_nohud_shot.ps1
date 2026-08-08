# _poppy_nohud_shot.ps1 -- one clean 1600x900 beauty PNG from the 19:53 binary.
#
# ASCII ONLY (PS 5.1 reads this file as ANSI; a UTF-8 dash in a string is a parse error).
#
# WHY THIS AND NOT THE gdigrab SCRIPTS: those exist because the OLD binary had no
# VOXELFORGE_NOHUD and no VOXELFORGE_CINE, so the only way to move the camera was
# to drive the editor free-cam with synthetic input, and the only way to get pixels
# was to crop the desktop. The 19:53 exe has both knobs, so the capture collapses to:
#   VOXELFORGE_CINE  parks the camera        (PostUpdate, wins over the boom)
#   VOXELFORGE_NOHUD blanks HUD + gizmos + egui
#   VOXELFORGE_SHOT  saves the swapchain straight to PNG at t=3.2s, then exits at 4.4s
# No ffmpeg, no desktop composite, no crop, no inpaint.
#
# The ONE thing still done through Win32: the window is hardcoded to 1280x720, so we
# resize the client to 1600x900 as soon as it appears. Bevy follows the window with
# its swapchain, and screenshot_once grabs whatever the swapchain is at 3.2s -- so
# the resize just has to land before then (it lands ~2s early in practice).
#
# BUILD LANE: this script only RUNS an existing exe. It never invokes cargo and never
# writes inside a target dir.
param(
    [Parameter(Mandatory=$true)][string]$Name,
    [string]$Mode = "--play",
    [string]$Cine = "",          # "ex,ey,ez, ex2,ey2,ez2, ax,ay,az[, ax2,ay2,az2], secs"
    [string]$Env = "",           # "K=V;K=V" extra VOXELFORGE_* knobs
    [string]$OutDir = "_poppy_beauty/final",
    [int]$ClientW = 1600,
    [int]$ClientH = 900
)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$exe = Join-Path $root "target-flamingo/release/voxelforge.exe"
if (-not (Test-Path $exe)) { throw "missing $exe" }
$exeName = "voxelforge"
New-Item -ItemType Directory -Force -Path (Join-Path $root $OutDir) | Out-Null
$rel = "$OutDir/$Name.png"
$png = Join-Path $root $rel
$log = Join-Path $root "$OutDir/$Name.log"

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32N {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowThreadProcessId(IntPtr h, out int pid);
  // The process also owns a 16x16 helper window and MainWindowHandle picks whichever
  // existed first (a 0x0 client rect forever). Pick by geometry instead.
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
"@

# Start-Process hands the child our environment block, so setting them here is how
# the VOXELFORGE_* knobs reach the game.
$env:VOXELFORGE_NOHUD = "1"
$env:VOXELFORGE_SHOT  = $rel
Remove-Item env:VOXELFORGE_CINE -ErrorAction SilentlyContinue
Remove-Item env:VOXELFORGE_CINE_START -ErrorAction SilentlyContinue
Remove-Item env:VOXELFORGE_CINE_EXIT -ErrorAction SilentlyContinue
Remove-Item env:VOXELFORGE_ANIM_POSE -ErrorAction SilentlyContinue
Remove-Item env:VOXELFORGE_LOOK_SUN -ErrorAction SilentlyContinue
if ($Cine -ne "") {
    $env:VOXELFORGE_CINE = $Cine
    # Park at the opening pose from frame 0: these are locked-off compositions, not
    # dollies, so there is nothing to ease into and no reason to burn a settle window.
    $env:VOXELFORGE_CINE_START = "0"
}
foreach ($kv in ($Env -split ";")) {
    if ($kv -match "^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$") {
        Set-Item -Path ("env:" + $matches[1]) -Value $matches[2]
        "ENV $($matches[1])=$($matches[2])"
    }
}

Get-Process -Name $exeName -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item $png -ErrorAction SilentlyContinue
$proc = Start-Process -FilePath $exe -ArgumentList $Mode -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err"

# Poll hard: the resize has to land before the 3.2s shot timer.
$hwnd = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(25)
while ((Get-Date) -lt $deadline) {
    if ($proc.HasExited) { break }
    $hwnd = [W32N]::FindRenderWindow($proc.Id, 320, 200)
    if ($hwnd -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 20
}
if ($hwnd -ne [IntPtr]::Zero) {
    $wr = New-Object W32N+RECT; $c0 = New-Object W32N+RECT
    [W32N]::GetWindowRect($hwnd, [ref]$wr) | Out-Null
    [W32N]::GetClientRect($hwnd, [ref]$c0) | Out-Null
    $chromeW = ($wr.R - $wr.L) - ($c0.R - $c0.L)
    $chromeH = ($wr.B - $wr.T) - ($c0.B - $c0.T)
    $scr = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $wantW = [Math]::Min($ClientW + $chromeW, $scr.Width)
    $wantH = [Math]::Min($ClientH + $chromeH, $scr.Height)
    [W32N]::SetWindowPos($hwnd, [IntPtr](-1), 0, 0, $wantW, $wantH, 0) | Out-Null
    $t = [math]::Round(((Get-Date) - $proc.StartTime).TotalSeconds, 2)
    "WINDOW resized at t=${t}s chrome=${chromeW}x${chromeH} want=${wantW}x${wantH}"
} else {
    "WINDOW not found -- falling back to the built-in 1280x720"
}

# screenshot_once exits at t=4.4s on its own; 40s is only a hang guard.
if (-not $proc.WaitForExit(40000)) { $proc | Stop-Process -Force; "TIMEOUT -- killed" }
Start-Sleep -Milliseconds 300
Get-Process -Name $exeName -ErrorAction SilentlyContinue | Stop-Process -Force

if (Test-Path $png) {
    Add-Type -AssemblyName System.Drawing
    $img = [System.Drawing.Image]::FromFile($png)
    $dims = "$($img.Width)x$($img.Height)"
    $img.Dispose()
    "OK $rel $dims $((Get-Item $png).Length)b"
} else {
    "MISS $rel -- see $log"
    Get-Content $log -Tail 6
}
