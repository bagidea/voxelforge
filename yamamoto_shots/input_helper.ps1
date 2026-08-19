Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Input {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int nWidth, int nHeight, bool bRepaint);
    [DllImport("user32.dll")] public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool fAttach);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll", CharSet = CharSet.Auto, CallingConvention = CallingConvention.StdCall)]
    public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, int dwExtraInfo);
    [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, int dwExtraInfo);
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@

$MOUSEEVENTF_MOVE = 0x0001
$MOUSEEVENTF_LEFTDOWN = 0x0002
$MOUSEEVENTF_LEFTUP = 0x0004
$MOUSEEVENTF_RIGHTDOWN = 0x0008
$MOUSEEVENTF_RIGHTUP = 0x0010
$KEYEVENTF_KEYUP = 0x0002

function Find-VFWindow {
    $p = Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if ($p) { return $p.MainWindowHandle }
    return [IntPtr]::Zero
}

function Focus-VFWindow {
    $h = Find-VFWindow
    if ($h -ne [IntPtr]::Zero) {
        [Win32Input]::ShowWindow($h, 9) | Out-Null   # SW_RESTORE
        [Win32Input]::MoveWindow($h, 0, 0, 1280, 800, $true) | Out-Null

        $fgWnd = [Win32Input]::GetForegroundWindow()
        $fgPid = 0
        $fgThread = [Win32Input]::GetWindowThreadProcessId($fgWnd, [ref]$fgPid)
        $curThread = [Win32Input]::GetCurrentThreadId()
        [Win32Input]::AttachThreadInput($curThread, $fgThread, $true) | Out-Null

        [Win32Input]::SetWindowPos($h, [IntPtr](-1), 0, 0, 1280, 800, 0x0040) | Out-Null  # HWND_TOPMOST, SWP_SHOWWINDOW
        [Win32Input]::BringWindowToTop($h) | Out-Null
        [Win32Input]::SetForegroundWindow($h) | Out-Null

        [Win32Input]::AttachThreadInput($curThread, $fgThread, $false) | Out-Null
        Start-Sleep -Milliseconds 300
    }
    return $h
}

function Unpin-VFWindow {
    $h = Find-VFWindow
    if ($h -ne [IntPtr]::Zero) {
        [Win32Input]::SetWindowPos($h, [IntPtr](-2), 0, 0, 1280, 800, 0x0040) | Out-Null  # HWND_NOTOPMOST
    }
}

function Send-Key([byte]$vk, [int]$holdMs = 80) {
    [Win32Input]::keybd_event($vk, 0, 0, 0)
    Start-Sleep -Milliseconds $holdMs
    [Win32Input]::keybd_event($vk, 0, $KEYEVENTF_KEYUP, 0)
}

function Hold-Key([byte]$vk, [int]$ms) {
    [Win32Input]::keybd_event($vk, 0, 0, 0)
    Start-Sleep -Milliseconds $ms
    [Win32Input]::keybd_event($vk, 0, $KEYEVENTF_KEYUP, 0)
}

function Move-MouseRel([int]$dx, [int]$dy) {
    [Win32Input]::mouse_event($MOUSEEVENTF_MOVE, $dx, $dy, 0, 0)
}

function Click-Left([int]$holdMs = 50) {
    [Win32Input]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0)
    Start-Sleep -Milliseconds $holdMs
    [Win32Input]::mouse_event($MOUSEEVENTF_LEFTUP, 0, 0, 0, 0)
}

function Click-Right([int]$holdMs = 50) {
    [Win32Input]::mouse_event($MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0)
    Start-Sleep -Milliseconds $holdMs
    [Win32Input]::mouse_event($MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0)
}

function Grab-Screen([string]$path) {
    Add-Type -AssemblyName System.Windows.Forms,System.Drawing
    $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
}

# VK codes
$VK_W = 0x57
$VK_A = 0x41
$VK_S = 0x53
$VK_D = 0x44
$VK_SPACE = 0x20
$VK_ESCAPE = 0x1B
$VK_TAB = 0x09
$VK_1 = 0x31
$VK_2 = 0x32
