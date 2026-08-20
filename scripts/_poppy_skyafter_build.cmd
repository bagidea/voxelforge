@echo off
REM AFTER half of the sky A/B: build e4ceb80 EXACTLY, from its own detached
REM worktree, so no other lane's uncommitted edits land in the binary.
REM BEFORE half is target-poppy\release\voxelforge_SKY_BEFORE_e4ceb80parent.exe (0e54d85).
REM
REM Fire-and-forget: waits for a free cargo slot, builds, writes the .done file.
REM If the slot guard gives up (exit 1) we do NOT build -- starting a 3rd cargo
REM here is exactly the STATUS_DLL_INIT_FAILED case the guard exists to prevent.
REM .done codes: cargo's own exit code, or 90 = guard aborted, nothing was built.
REM
REM   _poppy_skyafter_build.cmd                    real run
REM   _poppy_skyafter_build.cmd DRYRUN-GUARDFAIL   force the guard to fail; proves
REM                                                the abort path, writes *_dryrun.*
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set SUFFIX=
set GUARDARGS=
if /i "%~1"=="DRYRUN-GUARDFAIL" set SUFFIX=_dryrun
if /i "%~1"=="DRYRUN-GUARDFAIL" set GUARDARGS=-MaxOtherBuilds 0 -MaxWaitMinutes 0
set LOG=%ROOT%\_poppy_skyafter_build%SUFFIX%.log
set DONE=%ROOT%\_poppy_skyafter_build%SUFFIX%.done
set WAITLOG=%ROOT%\_poppy_skyafter_wait%SUFFIX%.log
del /q "%DONE%" 2>nul
del /q "%LOG%" 2>nul
powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%\scripts\_poppy_wait_build_slot.ps1" %GUARDARGS% > "%WAITLOG%" 2>&1
if errorlevel 1 goto :abort
cd /d "%ROOT%\_poppy_sky_wt"
cargo build --release -p voxelforge --bin voxelforge -j 2 --target-dir "%ROOT%\target-poppy-sky" > "%LOG%" 2>&1
echo %ERRORLEVEL% > "%DONE%"
goto :end
:abort
echo GUARD-ABORT no free cargo slot, build NOT started > "%LOG%"
echo 90 > "%DONE%"
:end
