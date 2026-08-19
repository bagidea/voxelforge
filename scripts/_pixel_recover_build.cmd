@echo off
REM ===========================================================================
REM Re-shoot lane build (pixel/flamingo, 2026-08-17) -- recovery of the 64
REM beauty plates `git clean -fd` destroyed on 2026-08-16.
REM
REM Builds the ONE binary the whole re-shoot is fired on, from HEAD (2262f71),
REM in its OWN detached worktree so the six lanes with uncommitted work in the
REM main tree cannot land in the exe.  Same shape as
REM scripts/_flamingo_beauty_build.cmd, which built the LOST plates' binary at
REM 97cb283 -- only the worktree and the commit differ, so a plate delta can be
REM attributed to the 223-line look.rs change between those two commits and not
REM to a different build recipe.
REM
REM Profile `perf` = release opt-level WITHOUT fat LTO (Cargo.toml:19-26), the
REM same profile the lost plates were shot on.
REM ===========================================================================
setlocal
set WT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_pixel_recover_wt
set LOG=%WT%\_build.log
set CARGO_TARGET_DIR=%WT%\target-pixel

del /q "%WT%\_build.done" 2>nul
echo [build] start %DATE% %TIME% > "%LOG%"
echo [build] commit=2262f71 target=%CARGO_TARGET_DIR% >> "%LOG%"
cd /d "%WT%"
cargo build --profile perf --bin voxelforge >> "%LOG%" 2>&1
echo EXIT=%ERRORLEVEL% >> "%LOG%"
echo [build] end %DATE% %TIME% >> "%LOG%"
echo DONE > "%WT%\_build.done"
endlocal
