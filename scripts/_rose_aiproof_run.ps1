# Enemy-AI proof harness (Rose, 2026-08-14).
# Runs AFTER target-rose\debug\voxelforge_enemyai_proof.exe exists.
# Every verdict line it prints is a quote of a line the BIN printed itself
# ("AIPROOF DONE ...", "AI id=... TELEGRAPH->COMMIT ...") — this script adds
# no PASS/FAIL of its own.
#
#   powershell -File scripts\_rose_aiproof_run.ps1
#
# Steps: run the bin with frames+trace env -> pick 4 frames from trace.csv
# (first real occurrence of patrol / alert / pursuit(closing) / strike) ->
# copy those PNGs to docs/assets/ai/rose-ai-01..04-*.png.

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$exe = "target-rose\debug\voxelforge_enemyai_proof.exe"
if (-not (Test-Path $exe)) { Write-Output "HARNESS: exe missing: $exe"; exit 1 }

$frames = "docs/assets/ai/_frames"
New-Item -ItemType Directory -Force $frames | Out-Null
$env:VOXELFORGE_AIFRAMES = $frames
$env:VOXELFORGE_AILOG = "docs/assets/ai/trace.csv"

& $exe 2>&1 | Tee-Object -FilePath docs/assets/ai/runlog.txt | ForEach-Object {
    if ($_ -match 'AIPROOF DONE|TELEGRAPH->COMMIT') { Write-Output "BIN: $_" }
}
Remove-Item Env:VOXELFORGE_AIFRAMES, Env:VOXELFORGE_AILOG

# ---- pick 4 frames from the trace the sim itself wrote ----
$rows = Import-Csv docs/assets/ai/trace.csv
function First-Row($pred) {
    foreach ($r in $rows) { if (& $pred $r) { return $r } }
    return $null
}
$patrol  = First-Row { param($r) $r.state -eq 'patrol' }
$alert   = First-Row { param($r) $r.state -eq 'alert' }
$pursuit = First-Row { param($r) ([double]$r.dist -lt 10.0) -and ($r.state -in 'chase','stalk','advance') }
$strike  = First-Row { param($r) $r.state -in 'strike','pounce' }

$picks = @(
    @{n='rose-ai-01-patrol.png';  r=$patrol;  why='first patrol row'},
    @{n='rose-ai-02-alert.png';   r=$alert;   why='first alert row'},
    @{n='rose-ai-03-pursuit.png'; r=$pursuit; why='first closing chase/stalk under dist 10'},
    @{n='rose-ai-04-strike.png';  r=$strike;  why='first strike/pounce row'}
)
foreach ($p in $picks) {
    if ($null -eq $p.r) { Write-Output "HARNESS: MISSING state for $($p.n)"; continue }
    $f = [int]$p.r.frame
    $src = "{0}/f{1:d4}.png" -f $frames, $f
    if (-not (Test-Path $src)) { $src = "{0}/f{1:d4}.png" -f $frames, ($f + 1) }  # capture is every 2nd frame
    if (-not (Test-Path $src)) { $src = "{0}/f{1:d4}.png" -f $frames, ($f - 1) }
    Copy-Item $src ("docs/assets/ai/" + $p.n)
    Write-Output ("HARNESS: {0} <- frame {1} ({2}) enemy={3} {4} dist={5}" -f `
        $p.n, $p.r.frame, $p.why, $p.r.enemy, $p.r.state, $p.r.dist)
}
Write-Output "HARNESS: done — see docs/assets/ai/"
