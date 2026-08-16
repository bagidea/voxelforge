@echo off
REM ===========================================================================
REM Poppy -- Hour::NIGHT.ev100 7.5 -> 8.6, before/after on ONE binary.
REM
REM WHY NOT _poppy_lookv7_expo_shoot.cmd. That harness's BEFORE leg is
REM `VOXELFORGE_LOOK_GEN=v2` -- it grades the v2/v3 RIG change, and its after leg
REM moves the exposure on top of that. Two things move between its columns, so
REM it cannot answer "what did ev100 alone do". This one holds the rig at v3 on
REM both legs and moves exactly one float.
REM
REM ONE BINARY, BOTH LEGS. `VOXELFORGE_LOOK_EXPOSURE` is applied in look.rs
REM AFTER the generation fork and AFTER the `Hour` constant is picked, so
REM `=7.5` reproduces the shipped-before default exactly. The AFTER leg sets NO
REM lever at all: it reads `Hour::NIGHT.ev100` straight out of the binary, which
REM is what makes it a proof of the committed source value and not of an env
REM string. If the two frames come out byte-identical the source edit never made
REM it into this exe.
REM
REM Scene pose / night flag copied VERBATIM from _poppy_lookv7_expo_shoot.cmd's
REM scene 3 so these frames are comparable to the v7 ladder plates.
REM
REM Usage: _poppy_ev100_night_shoot.cmd <exe> <outdir>
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set EXE=%~1
set OUT=%~2
if "%EXE%"=="" set EXE=%ROOT%\target-poppy\perf\voxelforge.exe
if "%OUT%"=="" set OUT=%ROOT%\_poppy_ev100night
if not exist "%OUT%" mkdir "%OUT%"
del /q "%OUT%\_shoot.done" 2>nul

set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0
set VOXELFORGE_LOOK_GEN=v3
set VOXELFORGE_LOOK_NIGHT=1
set VOXELFORGE_CINE=36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1
set SCENE=night-firelit

REM ---- before: the shipped 7.5, forced through the env lever ---------------
set VOXELFORGE_LOOK_EXPOSURE=7.5
call :shoot before

REM ---- after: NO lever -- whatever Hour::NIGHT.ev100 is in this binary -----
set VOXELFORGE_LOOK_EXPOSURE=
call :shoot after

echo DONE > "%OUT%\_shoot.done"
endlocal
goto :eof

:shoot
set VOXELFORGE_SHOT=%OUT%\%SCENE%_%~1.png
echo [shoot] %SCENE% %~1 gen=v3 ev=%VOXELFORGE_LOOK_EXPOSURE% -^> %VOXELFORGE_SHOT%
echo [shoot] %SCENE% %~1 gen=v3 ev=%VOXELFORGE_LOOK_EXPOSURE% >> "%OUT%\_shoot.log"
"%EXE%" --play >> "%OUT%\_shoot.log" 2>&1
goto :eof
