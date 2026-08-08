# _poppy_beauty_flycam.ps1 -- fly the REAL gameplay camera to a beauty pose and
# grab one lossless PNG, without a rebuild.
#
# ASCII ONLY (PS 5.1 reads this file as ANSI).
#
# WHY INPUT AND NOT AN ENV KNOB: the 16:15 flamingo binary predates
# VOXELFORGE_CINE, and its VOXELFORGE_LOOK_CAM is dead on arrival -- scene.rs
# `place_player` at that vintage stomps yaw/pitch/dist back to the wake pose on
# the boot frame. The only camera in that exe that can leave the spawn pose is
# the editor free-cam (Tab -> AppState::Editor, F -> fly), which is live because
# a plain `--play` boot is `not_scripted`. So: drive it like a player would.
#
# Grab path is gdigrab DESKTOP cropped to the client rect -- `-i title=` BitBlts
# the window DC and comes back black on a wgpu swapchain.
param(
    [string]$Name = "vista",
    [double]$SettleSec = 6.0,   # let the map stream + the look stack settle
    [int]$PitchDown = 0,        # mouse dy while middle-dragging (+ = look down)
    [int]$YawTurn = 0,          # mouse dx while middle-dragging (+ = turn right)
    [double]$RiseSec = 0.0,     # hold Space this long (fly up)
    [double]$BackSec = 0.0,     # hold S this long (dolly back)
    [double]$FwdSec = 0.0,      # hold W this long (dolly in)
    [int]$ZoomOut = 0,          # wheel notches before F (orbit mode zoom out)
    [switch]$NoFly,             # stay in orbit mode instead of pressing F
    [double]$HoldSec = 1.2,     # settle again before the grab
    [string]$Env = "",          # "K=V;K=V" passed through to the game process
    [int]$ClientW = 1600,       # render at more than the hardcoded 1280x720 --
    [int]$ClientH = 900         # the window is resizable, so the swapchain follows
)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$exe = Join-Path $root "_poppy_beauty.exe"
if (-not (Test-Path $exe)) { throw "missing _poppy_beauty.exe" }
$outDir = Join-Path $root "_poppy_beauty"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$png = Join-Path $outDir "$Name.png"
$log = Join-Path $outDir "$Name.log"

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32P {
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
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint data, IntPtr extra);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint f, IntPtr extra);

  public const uint MOVE=0x0001, MDOWN=0x0020, MUP=0x0040, WHEEL=0x0800;
  public const uint LDOWN=0x0002, LUP=0x0004;
  public const uint KEYUP=0x0002;

  // Process.MainWindowHandle picks a 16x16 helper window here; find by geometry.
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
  // winit resolves a key from its SCANCODE, so keybd_event with scan=0 can land
  // as the wrong key (or nothing). Always fill it from MapVirtualKey.
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint type);
  static byte Scan(byte vk) { return (byte)MapVirtualKey(vk, 0); }
  public static void Tap(byte vk) { keybd_event(vk,Scan(vk),0,IntPtr.Zero); System.Threading.Thread.Sleep(60); keybd_event(vk,Scan(vk),KEYUP,IntPtr.Zero); }
  public static void Hold(byte vk, int ms) { keybd_event(vk,Scan(vk),0,IntPtr.Zero); System.Threading.Thread.Sleep(ms); keybd_event(vk,Scan(vk),KEYUP,IntPtr.Zero); }
  // Middle-drag in small steps: the camera integrates MouseMotion deltas, and one
  // huge jump can be swallowed as a single frame's event.
  public static void MidDrag(int dx, int dy, int steps) {
    mouse_event(MDOWN,0,0,0,IntPtr.Zero);
    System.Threading.Thread.Sleep(60);
    for (int i=0;i<steps;i++) { mouse_event(MOVE,dx/steps,dy/steps,0,IntPtr.Zero); System.Threading.Thread.Sleep(16); }
    System.Threading.Thread.Sleep(60);
    mouse_event(MUP,0,0,0,IntPtr.Zero);
  }
  // SetForegroundWindow alone is refused for a background console (foreground
  // lock), so the mouse reached the game but the keyboard never did. Attach to
  // the target's input queue for the duration of the call, which is the
  // documented way around it.
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(int idAttach, int idAttachTo, bool attach);
  [DllImport("user32.dll")] public static extern IntPtr SetFocus(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr SetActiveWindow(IntPtr h);
  [DllImport("kernel32.dll")] public static extern int GetCurrentThreadId();
  public static bool ForceForeground(IntPtr h) {
    int pid; int target = GetWindowThreadProcessId(h, out pid);
    int me = GetCurrentThreadId();
    AttachThreadInput(me, target, true);
    SetForegroundWindow(h); SetActiveWindow(h); SetFocus(h);
    AttachThreadInput(me, target, false);
    System.Threading.Thread.Sleep(120);
    return GetForegroundWindow() == h;
  }
  // A real click activates the window, which is the only thing that reliably
  // hands keyboard focus to a game launched from a background console --
  // SetForegroundWindow (even with AttachThreadInput) is refused there.
  public static void ClickAt(int x, int y) {
    SetCursorPos(x, y);
    System.Threading.Thread.Sleep(80);
    mouse_event(LDOWN,0,0,0,IntPtr.Zero);
    System.Threading.Thread.Sleep(60);
    mouse_event(LUP,0,0,0,IntPtr.Zero);
    System.Threading.Thread.Sleep(200);
  }
  public static void Wheel(int notches) {
    for (int i=0;i<Math.Abs(notches);i++) { mouse_event(WHEEL,0,0,(uint)(notches>0?120:-120),IntPtr.Zero); System.Threading.Thread.Sleep(30); }
  }
}
"@ -ReferencedAssemblies System.Drawing

Get-Process -Name "_poppy_beauty" -ErrorAction SilentlyContinue | Stop-Process -Force
# Start-Process gives the child our environment block, so setting them here is
# how VOXELFORGE_* knobs reach the game.
foreach ($kv in ($Env -split ";")) {
    if ($kv -match "^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=(.*)$") {
        [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2])
        "ENV $($matches[1])=$($matches[2])"
    }
}
$proc = Start-Process -FilePath $exe -ArgumentList "--play" -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err"

$hwnd = [IntPtr]::Zero
$deadline = (Get-Date).AddSeconds(25)
while ((Get-Date) -lt $deadline) {
    if ($proc.HasExited) { throw "game exited early, see $log" }
    $hwnd = [W32P]::FindRenderWindow($proc.Id, 640, 360)
    if ($hwnd -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 50
}
if ($hwnd -eq [IntPtr]::Zero) { $proc | Stop-Process -Force; throw "no sized game window" }

# Grow the window so the beauty frames are not stuck at the hardcoded 1280x720.
# Bevy resizes its swapchain with the window, and gdigrab cannot capture past the
# desktop, so the client is clamped to the screen.
$wr = New-Object W32P+RECT
$c0 = New-Object W32P+RECT
[W32P]::GetWindowRect($hwnd, [ref]$wr) | Out-Null
[W32P]::GetClientRect($hwnd, [ref]$c0) | Out-Null
$chromeW = ($wr.R - $wr.L) - ($c0.R - $c0.L)
$chromeH = ($wr.B - $wr.T) - ($c0.B - $c0.T)
$scr = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$wantW = [Math]::Min($ClientW + $chromeW, $scr.Width)
$wantH = [Math]::Min($ClientH + $chromeH, $scr.Height)
# HWND_TOPMOST(-1)
[W32P]::SetWindowPos($hwnd, [IntPtr](-1), 0, 0, $wantW, $wantH, 0) | Out-Null
[W32P]::ForceForeground($hwnd) | Out-Null
Start-Sleep -Milliseconds 800

$cr = New-Object W32P+RECT
[W32P]::GetClientRect($hwnd, [ref]$cr) | Out-Null
$origin = New-Object System.Drawing.Point 0, 0
[W32P]::ClientToScreen($hwnd, [ref]$origin) | Out-Null
$w = $cr.R - $cr.L; $h = $cr.B - $cr.T
"WINDOW client=${w}x${h} at ($($origin.X),$($origin.Y))"

Start-Sleep -Seconds $SettleSec
# Park the pointer inside the client so the drag lands on the game window.
[W32P]::SetCursorPos($origin.X + [int]($w/2), $origin.Y + [int]($h/2)) | Out-Null
[W32P]::ForceForeground($hwnd) | Out-Null
# Click high in the frame (sky / far wall) so PLAY-mode L=break cannot reach a
# voxel, purely to steal focus.
[W32P]::ClickAt($origin.X + [int]($w/2), $origin.Y + 30)
$fg = [W32P]::ForceForeground($hwnd)
"FOCUS foreground=$fg"
if (-not $fg) { $proc | Stop-Process -Force; throw "could not give the game window keyboard focus" }
Start-Sleep -Milliseconds 200

[W32P]::Tap(0x09)                       # Tab -> AppState::Editor (free cursor)
Start-Sleep -Milliseconds 500
if ($ZoomOut -ne 0) {
    for ($i = 0; $i -lt $ZoomOut; $i++) {
        [W32P]::mouse_event(0x0800, 0, 0, [uint32]4294967176, [IntPtr]::Zero)  # -120 = one notch back
        Start-Sleep -Milliseconds 40
    }
    Start-Sleep -Milliseconds 400
}
if (-not $NoFly) { [W32P]::Tap(0x46); Start-Sleep -Milliseconds 300 }   # F -> fly
if ($PitchDown -ne 0 -or $YawTurn -ne 0) { [W32P]::MidDrag($YawTurn, $PitchDown, 24); Start-Sleep -Milliseconds 300 }
if ($RiseSec -gt 0) { [W32P]::Hold(0x20, [int]($RiseSec*1000)) }        # Space
if ($BackSec -gt 0) { [W32P]::Hold(0x53, [int]($BackSec*1000)) }        # S
if ($FwdSec  -gt 0) { [W32P]::Hold(0x57, [int]($FwdSec*1000)) }         # W
Start-Sleep -Seconds $HoldSec

& ffmpeg -hide_banner -loglevel error -f gdigrab -draw_mouse 0 `
    -offset_x $origin.X -offset_y $origin.Y -video_size "${w}x${h}" `
    -i desktop -frames:v 1 -y $png
$rc = $LASTEXITCODE

Start-Sleep -Milliseconds 200
Get-Process -Name "_poppy_beauty" -ErrorAction SilentlyContinue | Stop-Process -Force
if (Test-Path $png) { "SHOT $png ($((Get-Item $png).Length)b) ffmpeg=$rc" } else { "MISS $png ffmpeg=$rc" }
