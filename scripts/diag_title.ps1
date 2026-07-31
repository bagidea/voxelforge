# diag_title.ps1 — Launch voxelforge --play, enumerate its window, check title bytes + FindWindow
$ErrorActionPreference = "Continue"

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class Wdiag {
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

$exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-combat\debug\voxelforge.exe"

# Kill stale
Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force

# Launch game
Write-Output "Launching: $exe --play"
$proc = Start-Process -FilePath $exe -ArgumentList "--play" -PassThru -WindowStyle Normal
$gamePid = $proc.Id
Write-Output "Game PID: $gamePid"

# Poll for window every 0.5s
$foundHwnd = [IntPtr]::Zero
$foundTitle = $null
$foundRect = $null

for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 500

    $cb = [Wdiag+EnumWin]{
        param($hWnd, $lp)
        $wpid = [UInt32]0
        [Wdiag]::GetWindowThreadProcessId($hWnd, [ref]$wpid) | Out-Null
        if ($wpid -eq $gamePid -and [Wdiag]::IsWindowVisible($hWnd)) {
            $sb = New-Object System.Text.StringBuilder(512)
            [Wdiag]::GetWindowTextW($hWnd, $sb, 512) | Out-Null
            $t = $sb.ToString()
            if ($t.Length -gt 0) {
                $rect = New-Object Wdiag+RECT
                [Wdiag]::GetWindowRect($hWnd, [ref]$rect) | Out-Null
                $script:foundHwnd = $hWnd
                $script:foundTitle = $t
                $script:foundRect = $rect
                return $false
            }
        }
        return $true
    }
    [Wdiag]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null

    if ($foundTitle) {
        $elapsed = [math]::Round(($i + 1) * 0.5, 1)
        Write-Output "`nWindow found after ${elapsed}s"
        Write-Output "Title: '$foundTitle'"
        Write-Output "Title length: $($foundTitle.Length)"
        # Hex dump of title
        $hex = ($foundTitle.ToCharArray() | ForEach-Object { "0x{0:X4}" -f [int]$_ }) -join " "
        Write-Output "Title hex: $hex"

        $w = $foundRect.Right - $foundRect.Left
        $h = $foundRect.Bottom - $foundRect.Top
        Write-Output "RECT: left=$($foundRect.Left) top=$($foundRect.Top) right=$($foundRect.Right) bottom=$($foundRect.Bottom) => ${w}x${h}"

        # Test FindWindowW with this exact title
        Write-Output "`n--- FindWindowW test ---"
        $hwnd1 = [Wdiag]::FindWindowW([NullString]::Value, $foundTitle)
        Write-Output "FindWindowW(NULL, '$foundTitle') => $hwnd1 (expected: $foundHwnd)"
        if ($hwnd1 -eq [IntPtr]::Zero) { Write-Output "  => FindWindowW FAILED!" }
        elseif ($hwnd1 -eq $foundHwnd) { Write-Output "  => FindWindowW MATCH!" }
        else { Write-Output "  => Different window!" }

        # Try with MainWindowTitle fallback
        $mwt = $proc.MainWindowTitle
        Write-Output "`nMainWindowTitle: '$mwt' (len=$($mwt.Length))"
        if ($mwt -and $mwt -ne $foundTitle) {
            $hwnd2 = [Wdiag]::FindWindowW([NullString]::Value, $mwt)
            Write-Output "FindWindowW(NULL, '$mwt') => $hwnd2"
            if ($hwnd2 -eq [IntPtr]::Zero) { Write-Output "  => FindWindowW FAILED!" }
            elseif ($hwnd2 -eq $foundHwnd) { Write-Output "  => FindWindowW MATCH!" }
        }

        # Try ffmpeg with the EnumWindows title
        Write-Output "`n--- ffmpeg test ---"
        $testmkv = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_combat_recordings\diag_test.mkv"
        $arg = "title=$foundTitle"
        Write-Output "ffmpeg -f gdigrab -i `"$arg`" -t 2 ..."
        $ffOut = & ffmpeg -y -hide_banner -loglevel error -t 2 -f gdigrab -framerate 10 -draw_mouse 0 -i "title=$foundTitle" -c:v libx264 -preset ultrafast -crf 23 -pix_fmt yuv420p "$testmkv" 2>&1
        Write-Output "ffmpeg stderr: $ffOut"
        Write-Output "ffmpeg exit: $LASTEXITCODE"
        if ((Test-Path $testmkv) -and ((Get-Item $testmkv).Length -gt 0)) {
            $sz = (Get-Item $testmkv).Length
            $dur = & ffprobe -v error -show_entries format=duration -of csv=p=0 "$testmkv" 2>&1
            $res = & ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=s=x:p=0 "$testmkv" 2>&1
            Write-Output "SUCCESS: ${sz} bytes, ${dur}s, ${res}"
        } else {
            Write-Output "ffmpeg gdigrab FAILED for this title"
            if (Test-Path $testmkv) { Remove-Item $testmkv }
        }

        break
    }

    if ($proc.HasExited) {
        Write-Output "Game exited before window found"
        break
    }
}

# Cleanup
if (-not $proc.HasExited) {
    Stop-Process -Id $gamePid -Force -ErrorAction SilentlyContinue
}
Write-Output "`nDone."