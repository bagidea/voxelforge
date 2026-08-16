# Rose end-to-end aiproof pipeline (2026-08-15) — DETACHED driver.
# Waits for the detached cargo build (PID in _rose_aiproof2.pid) to exit,
# then gates on BOTH zero '^error' lines AND a literal 'Finished' in the log
# (errors==0 alone is a false green - the compiler may have died early).
# Then runs the proof bin + picks the before/after evidence pairs via
# scripts/_rose_ai_pairs.py. Writes progress to _rose_aiproof_pipeline.log
# so any future session can see exactly where this got to. Prints nothing else.
$ErrorActionPreference = 'Continue'
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root
$log = "_rose_aiproof_pipeline.log"
function Say($m) { Add-Content $log ("{0} {1}" -f (Get-Date -Format 'HH:mm:ss'), $m) }

$buildPid = (Get-Content _rose_aiproof2.pid -ErrorAction SilentlyContinue | Select-Object -First 1)
Say "pipeline start; waiting for cargo pid=$buildPid"
while (Get-Process -Id $buildPid -ErrorAction SilentlyContinue) { Start-Sleep -Seconds 30 }
Say "cargo exited"

# Gate on `Finished` IN THE LOG, not errors==0 alone — a small log means the
# compiler died before reaching our crate and errors can still be 0 (false green)
$errs = (Select-String -Path _rose_aiproof2.log -Pattern '^error' -ErrorAction SilentlyContinue).Count
$finished = (Select-String -Path _rose_aiproof2.log -Pattern 'Finished').Count
Say "verdict: errors=$errs finished=$finished"
if ($errs -gt 0 -or $finished -eq 0 -or -not (Test-Path "target-rose\debug\voxelforge_enemyai_proof.exe")) {
    Say "ABORT: build red, unfinished, or exe missing — a session must fix source and relight"
    exit 1
}

# ---- run the proof bin (frames + trace) ----
$frames = "docs/assets/ai/_frames"
New-Item -ItemType Directory -Force $frames | Out-Null
$env:VOXELFORGE_AIFRAMES = $frames
$env:VOXELFORGE_AILOG = "docs/assets/ai/trace.csv"
& "target-rose\debug\voxelforge_enemyai_proof.exe" 2>&1 |
    Tee-Object -FilePath docs/assets/ai/runlog.txt |
    ForEach-Object { if ($_ -match 'AIPROOF DONE|TELEGRAPH->COMMIT') { Say "BIN: $_" } }
Remove-Item Env:VOXELFORGE_AIFRAMES, Env:VOXELFORGE_AILOG -ErrorAction SilentlyContinue
Say "bin run complete"

# ---- before/after pairs, picked from the trace the sim wrote ----
if (-not (Test-Path docs/assets/ai/trace.csv)) { Say "ABORT: trace.csv missing"; exit 1 }
Say "picking before/after pairs"
python scripts/_rose_ai_pairs.py docs/assets/ai/trace.csv $frames docs/assets/ai 2>&1 |
    ForEach-Object { Say "PAIRS: $_" }
if ($LASTEXITCODE -ne 0) {
    Say "ABORT: fewer than 3 real before/after pairs — a session must inspect the trace"
    exit 1
}
Say "pipeline done — before/after pairs in docs/assets/ai/"
