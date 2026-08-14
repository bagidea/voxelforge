@echo off
REM ===========================================================================
REM The instrument, not the verdict (flamingo, 2026-08-15).
REM
REM My first pass measured "dark ground far from any non-ground pixel" and
REM called that a cast shadow. The overlay
REM (docs/assets/look/ab-elev/overlay/) says it is not: a grass block's own SIDE
REM face is grass-hued too, so it sits INSIDE the ground mask, tens of px from
REM any mask boundary, and got counted as open ground. Same failure C2 was
REM retired for -- a statistic that cannot tell face shading from a shadow.
REM
REM THIS SEPARATES THEM, using one fact: a HORIZONTAL face's direct sun term is
REM illuminance * sin(elev) -- it does not depend on the sun's COMPASS bearing at
REM all. A VERTICAL face's does: cos(elev) * cos(delta-azimuth). So shooting the
REM same camera and elevation twice, 180 deg apart in azimuth (205 and 25):
REM
REM   * a lit top face is equally bright in BOTH shots
REM   * every side face flips -- lit in one, ambient-only in the other
REM   * a top face the sun cannot reach in one of them is a CAST SHADOW, and
REM     nothing else can produce that signature
REM
REM Cameras and world geometry are identical inside a row, so the top-face mask
REM derived at 66 deg is valid pixel-for-pixel at 22 deg too. That is what makes
REM the 2x2 gradeable on horizontal ground only.
REM
REM 4 shots, same exe and same pinned rig as the grid; only the azimuth moves.
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set EXE=%ROOT%\target-poppy\perf\voxelforge.exe
set OUT=%ROOT%\docs\assets\look\ab-elev

set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0
set VOXELFORGE_LOOK_GEN=v3
set VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00
set VOXELFORGE_LOOK_EXPOSURE=10.6

set VOXELFORGE_CINE=44,14,44, 44,14,44, 32.5,2.0,29.5, 1
set VOXELFORGE_LOOK_SUN=66,25,20000
call :shoot s1cam-elev66-az25
set VOXELFORGE_LOOK_SUN=22,25,20000
call :shoot s1cam-elev22-az25

set VOXELFORGE_CINE=18,9,44, 18,9,44, 31,1.5,29, 1
set VOXELFORGE_LOOK_SUN=66,25,20000
call :shoot s2cam-elev66-az25
set VOXELFORGE_LOOK_SUN=22,25,20000
call :shoot s2cam-elev22-az25
goto :done

:shoot
set VOXELFORGE_SHOT=%OUT%\%~1.png
echo [azflip] %~1  sun=%VOXELFORGE_LOOK_SUN%
echo [azflip] %~1  sun=%VOXELFORGE_LOOK_SUN% cine=%VOXELFORGE_CINE% >> "%OUT%\_shoot.log"
"%EXE%" --play >> "%OUT%\_shoot.log" 2>&1
goto :eof

:done
endlocal
