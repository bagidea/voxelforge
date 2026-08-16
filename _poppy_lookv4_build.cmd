@echo off
REM Look-v4 build (poppy). Same bin/profile/-j 2 as v2 and v3 -- the box is
REM running four other cargo lanes right now and more jobs is how this repo gets
REM 0xc0000142 (that crash is commit headroom, not a code bug).
REM
REM WHAT THIS PROVES: v3's log carried a STALE E0004 on look.rs:2379 -- rustc read
REM the file before the `LookFill::Rim` arm was saved (look.rs 21:24:23, log
REM 21:27:18). This run reads the saved file. quest.rs's five E0425 are Sun's lane
REM and may still be in flight; grade look.rs by errors that name look.rs, not by
REM the total.
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
del /q "%ROOT%\_poppy_lookv4_build.done" 2>nul
cd /d "%ROOT%\client"
set CARGO_TARGET_DIR=%ROOT%\target-poppy
cargo build --bin voxelforge --profile perf -j 2 > "%ROOT%\_poppy_lookv4_build.log" 2>&1
echo EXIT=%ERRORLEVEL% > "%ROOT%\_poppy_lookv4_build.done"
