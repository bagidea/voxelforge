@echo off
REM ===========================================================================
REM Pose probe for the beauty-axes ladder (flamingo, 2026-08-16).
REM
REM WHY THIS EXISTS.  Every camera pose committed in this repo aims at y~1.5-2.0
REM (grep VOXELFORGE_CINE over scripts/): they are all ground shots.  Measured on
REM the ab-elev plates I shot yesterday, all six carry a sky mask of 0.76-0.79%
REM of frame with the TOP ROW 0% sky -- the dome is not in frame at all.
REM
REM docs/flamingo-beauty-axes-2026-08-14.md footnotes 1-2 already ruled that a
REM sky axis measured on a mask that thin cannot stand (A' = 208 px = 0.01%, B =
REM 0.99%; both were refused).  So the ladder cannot be shot on any existing pose
REM -- it needs poses that actually contain a horizon.  This probe finds them.
REM
REM It sweeps eye height x aim tilt around the edhari village (map extents
REM x 0..63, y 0..30, z 0..63; village centred ~32.5,2,29.5) and shoots one plate
REM each.  The picker (_flamingo_beauty_pick.py) then keeps only poses whose sky
REM mask clears 3% of frame AND whose near/far bands are both non-degenerate.
REM Nothing here is a result -- it is pose selection, run once, before the ladder.
REM
REM Baseline light on every probe: NO knob overrides at all, so the poses are
REM chosen under the shipped look, not under a grade that flatters them.
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set WT=%ROOT%\_flamingo_beauty_wt
set EXE=%WT%\target-pixel\perf\voxelforge.exe
set OUT=%ROOT%\docs\assets\look\beauty\probe
if not exist "%OUT%" mkdir "%OUT%"
del /q "%OUT%\_probe.done" 2>nul

set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0
set VOXELFORGE_LOOK_GEN=v3

REM ---- eye high on the east rim, three tilts --------------------------------
set VOXELFORGE_CINE=52,9,48, 52,9,48, 24,9,18, 1
call :shoot p1-east-level
set VOXELFORGE_CINE=52,9,48, 52,9,48, 24,13,18, 1
call :shoot p2-east-up
set VOXELFORGE_CINE=52,9,48, 52,9,48, 24,5,18, 1
call :shoot p3-east-down

REM ---- long axis down z ------------------------------------------------------
set VOXELFORGE_CINE=32,7,58, 32,7,58, 32,9,6, 1
call :shoot p4-zaxis-level
set VOXELFORGE_CINE=32,7,58, 32,7,58, 32,13,6, 1
call :shoot p5-zaxis-up

REM ---- opposite diagonal, low eye -------------------------------------------
set VOXELFORGE_CINE=8,6,10, 8,6,10, 48,9,50, 1
call :shoot p6-west-level
set VOXELFORGE_CINE=8,6,10, 8,6,10, 48,13,50, 1
call :shoot p7-west-up

REM ---- the ab-elev vista eye, but aimed at the horizon instead of the dirt ---
set VOXELFORGE_CINE=44,10,44, 44,10,44, 20,12,20, 1
call :shoot p8-abelev-eye-horizon

echo DONE > "%OUT%\_probe.done"
endlocal
goto :eof

:shoot
set VOXELFORGE_SHOT=%OUT%\%~1.png
echo [probe] %~1  cine=%VOXELFORGE_CINE%
echo [probe] %~1  cine=%VOXELFORGE_CINE% >> "%OUT%\_probe.log"
"%EXE%" --play >> "%OUT%\_probe.log" 2>&1
goto :eof
