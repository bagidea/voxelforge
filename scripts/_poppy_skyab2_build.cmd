@echo off
REM ===========================================================================
REM Sky A/B, TWO binaries out of ONE clean worktree, ONE target dir.
REM
REM   worktree  _poppy_skyab_wt  detached at 676e2c5 (poppy/native-only tip, clean)
REM   target    target-poppysky
REM
REM AFTER  = 676e2c5 verbatim (the sky patch e4ceb80 is ALREADY an ancestor of the
REM          tip -- confirmed by git merge-base --is-ancestor).
REM BEFORE = the same tree with client/src/look.rs restored to 0e54d85 (e4ceb80's
REM          parent). look.rs has NO other drift between 0e54d85 and the tip
REM          (git diff --stat 0e54d85 676e2c5 -- client/src/look.rs == the sky
REM          patch exactly), so this reverts the sky patch and nothing else.
REM
REM Why not reuse the 05:11 BEFORE exe built at 0e54d85: 14 commits sit between
REM 0e54d85 and the tip, including 27cb2c5 (14 regenerated block textures). That
REM pair would NOT straddle only the sky patch -- the 2026-08-14 scar exactly.
REM
REM Fire-and-forget: waits for a free cargo slot, builds AFTER, then BEFORE.
REM Never starts a 3rd cargo (STATUS_DLL_INIT_FAILED guard).
REM .done codes: 0 = both arms compiled, 90 = guard aborted nothing built,
REM              else the first failing cargo exit code.
REM ===========================================================================
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set WT=%ROOT%\_poppy_skyab_wt
set TGT=%ROOT%\target-poppysky
set OUT=%ROOT%\_poppy_skyab2
set DONE=%OUT%\build.done

if not exist "%OUT%" mkdir "%OUT%"
del /q "%DONE%" 2>nul
del /q "%OUT%\after_build.log" 2>nul
del /q "%OUT%\before_build.log" 2>nul

powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%\scripts\_poppy_wait_build_slot.ps1" > "%OUT%\slot.log" 2>&1
if errorlevel 1 goto :abort

cd /d "%WT%"

REM ---- arm 1: AFTER = the tip verbatim -------------------------------------
git checkout HEAD -- client/src/look.rs
cargo build --release -p voxelforge --bin voxelforge -j 2 --target-dir "%TGT%" > "%OUT%\after_build.log" 2>&1
if errorlevel 1 goto :fail_after
copy /y "%TGT%\release\voxelforge.exe" "%OUT%\voxelforge_SKY_AFTER_676e2c5.exe" >nul

REM ---- arm 2: BEFORE = the same tree, sky patch reverted --------------------
git checkout 0e54d85 -- client/src/look.rs
cargo build --release -p voxelforge --bin voxelforge -j 2 --target-dir "%TGT%" > "%OUT%\before_build.log" 2>&1
if errorlevel 1 goto :fail_before
copy /y "%TGT%\release\voxelforge.exe" "%OUT%\voxelforge_SKY_BEFORE_676e2c5-minus-e4ceb80.exe" >nul

REM ---- leave the worktree back on the tip ----------------------------------
git checkout HEAD -- client/src/look.rs
echo 0 > "%DONE%"
goto :end

:fail_after
git checkout HEAD -- client/src/look.rs
echo AFTER-ARM-FAILED >> "%OUT%\after_build.log"
echo 11 > "%DONE%"
goto :end

:fail_before
git checkout HEAD -- client/src/look.rs
echo BEFORE-ARM-FAILED >> "%OUT%\before_build.log"
echo 12 > "%DONE%"
goto :end

:abort
echo GUARD-ABORT no free cargo slot, nothing built > "%OUT%\after_build.log"
echo 90 > "%DONE%"

:end
