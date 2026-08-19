# ===========================================================================
# Poppy - build the A4/A5 painted sky into target-poppy.
#
# STRAIGHT TO --release, NOT `cargo check` FIRST. target-poppy already holds a
# full release build of every dependency, so this run recompiles the client
# crate and relinks; a cold `cargo check` would rebuild bevy's metadata from
# scratch in a profile nothing else here uses and take LONGER than the thing it
# was meant to shortcut.
#
# -j 1, ONE BIN, and a per-lane CARGO_TARGET_DIR — three separate collisions.
#
#   CARGO_TARGET_DIR : two cargo processes over one target dir is the
#                      STATUS_DLL_INIT_FAILED (0xc0000142) this lane has been
#                      bitten by before.
#   --bin voxelforge : the workspace ship profile is `lto = "fat"` +
#                      `codegen-units = 1`, the most memory-hungry setting rustc
#                      has. A bare `cargo build --release` compiles FIVE bins of
#                      this crate under it.
#   -j 1             : 2026-08-18 16:20, a bare `cargo build --release` on this
#                      machine died with `rustc-LLVM ERROR: out of memory` and
#                      exit 0xc0000409 (STATUS_STACK_BUFFER_OVERRUN) on all five
#                      bins at once, with 0.4 GB free of 15.8 GB. Parallel fat-LTO
#                      links are what ate it. One job is slower and finishes.
#
# The caller checks `tasklist` for cargo/rustc BEFORE invoking this; the marker
# files below are what lets a poll say "done" without reading the whole log.
#
# USAGE  powershell -File scripts/_poppy_skybuild_20260818.ps1
# ===========================================================================
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

$env:CARGO_TARGET_DIR = Join-Path $root "target-poppy"

$log = Join-Path $root "_poppy_sky_build.log"
Remove-Item (Join-Path $root "_poppy_sky_build.done"), `
            (Join-Path $root "_poppy_sky_build.fail") -ErrorAction SilentlyContinue

"=== START $(Get-Date -Format o) ===" | Out-File -FilePath $log -Encoding utf8
& cargo build --release --bin voxelforge -j 1 *>> $log
$code = $LASTEXITCODE
"=== EXIT $code $(Get-Date -Format o) ===" | Out-File -FilePath $log -Append -Encoding utf8

if ($code -eq 0) { "ok" | Out-File (Join-Path $root "_poppy_sky_build.done") -Encoding utf8 }
else { "exit=$code" | Out-File (Join-Path $root "_poppy_sky_build.fail") -Encoding utf8 }
