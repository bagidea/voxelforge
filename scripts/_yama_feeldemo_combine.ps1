# _yama_feeldemo_combine.ps1
#
# Side-by-side "BEFORE | AFTER" comparison video for the player_tuning.rs feel
# refactor. Both source clips share the same scripted walk->run->jump->land
# timeline (before: SendInput against the old exe, elapsed 0.5-2.5s walk /
# 2.5-4.0s run / 4.0-4.2s jump / 4.2-6.5s resumed; after: the new exe's
# in-engine feel_demo_input on the identical offsets, exits at 6.0s) so they're
# already time-aligned -- `-shortest` just trims to whichever is a touch
# shorter instead of needing an explicit re-time.
#
# Usage: powershell -File scripts\_yama_feeldemo_combine.ps1 <before.mp4> <after.mp4>
#   (or run with no args to auto-pick the newest before_*.mp4 / after_*.mp4
#   under _yama_feeldemo\)
#
# Output: docs\assets\feel-demo-before-after_<ts>.mp4

param(
    [string]$BeforePath,
    [string]$AfterPath
)

$ErrorActionPreference = "Stop"
$ProjectDir = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $ProjectDir
$WorkDir = Join-Path $ProjectDir "_yama_feeldemo"
$OutDir = Join-Path $ProjectDir "docs\assets"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

if (-not $BeforePath) {
    $BeforePath = (Get-ChildItem $WorkDir -Filter "before_*.mp4" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
}
if (-not $AfterPath) {
    $AfterPath = (Get-ChildItem $WorkDir -Filter "after_*.mp4" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
}
if (-not $BeforePath -or -not (Test-Path $BeforePath)) { throw "before clip not found (pass -BeforePath or put one in $WorkDir)" }
if (-not $AfterPath -or -not (Test-Path $AfterPath)) { throw "after clip not found (pass -AfterPath or put one in $WorkDir)" }
Write-Output "before: $BeforePath"
Write-Output "after : $AfterPath"

$Ts = Get-Date -Format "yyyyMMdd-HHmmss"
$Combined = Join-Path $OutDir "feel-demo-before-after_$Ts.mp4"

# Scale each to 960x540 (half-width 1080p-ish, keeps the combined frame at a
# sane 1920x540) with a burned-in label, then stack side by side, capped to
# the shorter clip's length so a small duration mismatch doesn't freeze-frame
# the longer one at the end.
$filter = @"
[0:v]scale=960:540,drawtext=text='BEFORE':x=20:y=20:fontsize=36:fontcolor=white:box=1:boxcolor=black@0.5:boxborderw=8[left];
[1:v]scale=960:540,drawtext=text='AFTER':x=20:y=20:fontsize=36:fontcolor=white:box=1:boxcolor=black@0.5:boxborderw=8[right];
[left][right]hstack=inputs=2[out]
"@ -replace "`r?`n", ""

& ffmpeg -y -hide_banner -loglevel warning -i $BeforePath -i $AfterPath -filter_complex $filter -map "[out]" -c:v libx264 -preset medium -crf 18 -pix_fmt yuv420p -shortest $Combined

if (-not (Test-Path $Combined) -or (Get-Item $Combined).Length -lt 3000) { throw "combine failed: $Combined missing/empty" }
$sizeKb = [math]::Round((Get-Item $Combined).Length / 1KB, 1)
$dur = (& ffprobe -v error -show_entries format=duration -of csv=p=0 $Combined | Out-String).Trim()
Write-Output "combined: $Combined (${sizeKb} KB, ${dur}s)"
Write-Output "DONE"
