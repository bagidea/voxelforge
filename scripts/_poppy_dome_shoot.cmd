@echo off
REM ===========================================================================
REM A1 dome-dead close-out capture (poppy).
REM
REM ONE BINARY, THREE FRAMES. Every difference below is an env lever on the SAME
REM exe -- the scar that made the 2026-08-14 sky A/B worthless was two exes shot
REM 40 minutes apart with a block-repaint commit in between, so nothing here is
REM allowed to be a second build.
REM
REM   d0_atmos-on   SKYPROBE=1, atmosphere left ON   (the shipped default)
REM   d1_atmos-off  SKYPROBE=1, VOXELFORGE_LOOK_ATMOS=off
REM   d2_gradient   no probe,   VOXELFORGE_LOOK_ATMOS=off  (the real dome)
REM
REM THE CAMERA IS AIMED UP, ON PURPOSE. `_rose_a1_dome_pixels.py` grades the top
REM 40% of the frame and returns UNMEASURABLE (exit 2) when that band is textured
REM terrain instead of sky -- a play spawn framing the Edhari canyon wall says
REM nothing about the dome either way. Eye is the outdoor-noon eye verbatim
REM (44,14,44); only the look-at is lifted to y=40, ~54 deg up, which fills the
REM graded band with sky and leaves the skyline in the bottom of frame.
REM
REM WHY VOXELFORGE_CINE AND NOT VOXELFORGE_LOOK_CAM: the orbit boom is pulled in
REM by terrain collision on this campsite hard enough that the boom pitch does not
REM survive (`_poppy_lookv3_shoot.cmd` header). CINE writes the transform outright.
REM
REM Usage:  _poppy_dome_shoot.cmd [exe] [outdir]
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set EXE=%~1
set OUT=%~2
if "%EXE%"=="" set EXE=%ROOT%\target-poppy\perf\voxelforge.exe
if "%OUT%"=="" set OUT=%ROOT%\_poppy_dome\shots
if not exist "%OUT%" mkdir "%OUT%"
del /q "%OUT%\_shoot.done" 2>nul
del /q "%OUT%\_shoot.log" 2>nul

set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0
set VOXELFORGE_CINE=44,14,44, 44,14,44, 32.5,40,29.5, 1

set VOXELFORGE_LOOK_SKYPROBE=1
set VOXELFORGE_LOOK_ATMOS=
set VOXELFORGE_LOOK_SKYGRAD=
call :shoot d0_atmos-on
set VOXELFORGE_LOOK_ATMOS=off
call :shoot d1_atmos-off
set VOXELFORGE_LOOK_SKYPROBE=
call :shoot d2_gradient
REM d3 = the BEFORE half of the one-lever pair: same camera, same atmosphere
REM state, dome switched OFF at `sky_grad_enabled()` so the sky falls back to the
REM flat `ClearColor`. d3 vs d2 is the only pair where exactly one thing moved.
set VOXELFORGE_LOOK_SKYGRAD=off
call :shoot d3_skygrad-off

echo DONE > "%OUT%\_shoot.done"
endlocal
goto :eof

:shoot
set VOXELFORGE_SHOT=%OUT%\%~1.png
echo [shoot] %~1 atmos="%VOXELFORGE_LOOK_ATMOS%" skyprobe="%VOXELFORGE_LOOK_SKYPROBE%" >> "%OUT%\_shoot.log"
"%EXE%" --play >> "%OUT%\_shoot.log" 2>&1
echo [done ] %~1 exit=%ERRORLEVEL% >> "%OUT%\_shoot.log"
goto :eof
