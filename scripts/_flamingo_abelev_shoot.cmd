@echo off
REM ===========================================================================
REM Blocker #1 A/B: does the missing cast shadow follow the SUN ELEVATION or
REM the CAMERA?  (flamingo, 2026-08-15)
REM
REM look-gap-v4-2026-08-15.md says: scene 1 (wide vista cam, sun 66 deg) has no
REM cast shadow on the ground; scene 2 (closer cam, sun 22 deg) has correct soft
REM shadows on the SAME grass, SAME binary, SAME map.  Two things differ at once
REM there -- elevation and camera -- so the sheet could not name the cause.
REM
REM This crosses them.  FOUR plates, 2 cameras x 2 elevations, and EVERYTHING
REM else pinned identical: one exe, gen=v3, azimuth 205, illuminance 20000,
REM exposure 10.6, the neutral noon light rig, ultra tier, same CINE_START.
REM Elevation and camera pose are the ONLY two variables in the grid.
REM
REM   shadow follows the ELEVATION column -> depth bias / cascade at high sun
REM   shadow follows the CAMERA row       -> the vista camera's shadow frustum
REM
REM WHY THE NOON RIG ON ALL FOUR AND NOT EACH SCENE'S OWN.  Scene 2 normally
REM runs Hour::GOLDEN's own rig (22 000 lux, ev100 10.3, warm key).  Letting it
REM keep that would put a light-rig difference inside the grid alongside the two
REM variables under test.  The s2cam/elev22 cell is the control that says the
REM swap was harmless: it is the known-good shadow condition, and if the shadow
REM is still there under the pinned rig, the rig is not what moves the result.
REM
REM Writes to docs/assets/look/ab-elev/ -- NOT docs/assets/look/, which is a
REM live path being re-shot on a loop (see the pin note in the v4 gap sheet).
REM
REM Usage:  _flamingo_abelev_shoot.cmd [exe] [outdir]
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set EXE=%~1
set OUT=%~2
if "%EXE%"=="" set EXE=%ROOT%\target-poppy\perf\voxelforge.exe
if "%OUT%"=="" set OUT=%ROOT%\docs\assets\look\ab-elev
if not exist "%OUT%" mkdir "%OUT%"
del /q "%OUT%\_shoot.done" 2>nul

REM ---- pinned on every plate ------------------------------------------------
set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0
set VOXELFORGE_LOOK_GEN=v3
set VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00
set VOXELFORGE_LOOK_EXPOSURE=10.6

REM ---- row A: scene 1 camera (the wide vista) -------------------------------
set VOXELFORGE_CINE=44,14,44, 44,14,44, 32.5,2.0,29.5, 1
set VOXELFORGE_LOOK_SUN=66,205,20000
call :shoot s1cam-elev66
set VOXELFORGE_LOOK_SUN=22,205,20000
call :shoot s1cam-elev22

REM ---- row B: scene 2 camera (the close raking shot) ------------------------
set VOXELFORGE_CINE=18,9,44, 18,9,44, 31,1.5,29, 1
set VOXELFORGE_LOOK_SUN=66,205,20000
call :shoot s2cam-elev66
set VOXELFORGE_LOOK_SUN=22,205,20000
call :shoot s2cam-elev22

echo DONE > "%OUT%\_shoot.done"
endlocal
goto :eof

:shoot
set VOXELFORGE_SHOT=%OUT%\%~1.png
echo [shoot] %~1  sun=%VOXELFORGE_LOOK_SUN%  cine=%VOXELFORGE_CINE%
echo [shoot] %~1  sun=%VOXELFORGE_LOOK_SUN%  cine=%VOXELFORGE_CINE% >> "%OUT%\_shoot.log"
"%EXE%" --play >> "%OUT%\_shoot.log" 2>&1
goto :eof
