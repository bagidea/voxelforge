# ===========================================================================
# Poppy - build the current sky source into target-poppy (2026-08-19 round).
#
# Same shape as _poppy_skybuild_20260818.ps1, with -j 2 (Director's call for
# this round) and ONE bin. The five-bin fat-LTO OOM of 2026-08-18 16:20 came
# from a bare `cargo build --release`; naming the bin is what avoids it, not
# the job count. Free RAM at launch: 3.9 GB of 15.8.
#
# Verdict is read from `grep -c '^error' <log>`, never from `tail` and never
# from an exit code that crossed a pipe.
#
# USAGE  powershell -File scripts/_poppy_skybuild_20260819.ps1
# ===========================================================================
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

$env:CARGO_TARGET_DIR = Join-Path $root "target-poppy"

$log  = Join-Path $root "_poppy_skybuild_20260819.log"
$done = Join-Path $root "_poppy_skybuild_20260819.done"
$fail = Join-Path $root "_poppy_skybuild_20260819.fail"
Remove-Item $done, $fail -ErrorAction SilentlyContinue

"=== START $(Get-Date -Format o) ===" | Out-File -FilePath $log -Encoding utf8
& cargo build --release --bin voxelforge -j 2 *>> $log
$code = $LASTEXITCODE
"=== EXIT $code $(Get-Date -Format o) ===" | Out-File -FilePath $log -Append -Encoding utf8

if ($code -eq 0) { "ok" | Out-File $done -Encoding utf8 }
else { "exit=$code" | Out-File $fail -Encoding utf8 }
