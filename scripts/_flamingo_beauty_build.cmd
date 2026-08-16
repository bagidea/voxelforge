@echo off
REM ===========================================================================
REM Beauty-axes lane build (flamingo, 2026-08-16).
REM
REM Builds the ONE binary the whole ladder is shot on.  Every knob under test
REM (VOXELFORGE_LOOK_FOG / _AMBIENT / _EXPOSURE) is read from env at spawn, so
REM the ladder never needs a second link -- that is the whole reason the levers
REM exist (look.rs:1752-1801).
REM
REM Isolated on purpose:
REM   * own worktree   _flamingo_beauty_wt  (detached at the branch tip) so the
REM     six lanes with uncommitted work in the main tree cannot land in my exe
REM   * own target dir target-pixel/        so the shared target/ is untouched
REM
REM Profile `perf` = release opt-level WITHOUT fat LTO (Cargo.toml:19-26).  This
REM is a look A/B judged on pixels, not a CPU-time probe, and fat LTO costs ~20
REM min of link for nothing that shows up in a frame.
REM ===========================================================================
setlocal
set WT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_flamingo_beauty_wt
set LOG=%WT%\_build.log
set CARGO_TARGET_DIR=%WT%\target-pixel

del /q "%WT%\_build.done" 2>nul
echo [build] start %DATE% %TIME% > "%LOG%"
echo [build] target=%CARGO_TARGET_DIR% >> "%LOG%"
cd /d "%WT%"
cargo build --profile perf --bin voxelforge >> "%LOG%" 2>&1
echo EXIT=%ERRORLEVEL% >> "%LOG%"
echo [build] end %DATE% %TIME% >> "%LOG%"
echo DONE > "%WT%\_build.done"
endlocal
