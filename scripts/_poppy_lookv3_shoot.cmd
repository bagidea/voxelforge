@echo off
REM ===========================================================================
REM Look v3 before/after capture (poppy).
REM
REM ONE BINARY, SIX FRAMES. Every pair below is the SAME exe, the SAME camera
REM pose and the SAME hour -- the only thing that moves is VOXELFORGE_LOOK_GEN
REM (v2 = the rig that shipped 2026-08-14, v3 = the surface rig). That is the
REM whole point of look.rs's LookGen switch: a before/after that needs two
REM builds is showing you two builds, not one change.
REM
REM WHY VOXELFORGE_CINE AND NOT VOXELFORGE_LOOK_CAM. The orbit boom is pulled in
REM by terrain collision, and on the Edhari campsite it is pinned hard enough
REM that yaw 0 / 90 / 180 / 270 all render the same wall (measured -- see
REM _poppy_lookv3/scout/cam_*.png). A pair shot through it would be honest and
REM useless. CINE writes the gameplay camera's transform outright, so the two
REM frames of a pair are provably the same pose.
REM
REM Usage:  _poppy_lookv3_shoot.cmd [exe] [outdir]
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set EXE=%~1
set OUT=%~2
if "%EXE%"=="" set EXE=%ROOT%\target-poppy\perf\voxelforge.exe
if "%OUT%"=="" set OUT=%ROOT%\docs\assets\look
if not exist "%OUT%" mkdir "%OUT%"
del /q "%OUT%\_shoot.done" 2>nul

REM ---- shared: playable scene, no HUD, top tier, one still at t=3.2s --------
set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0

REM ---- scene 1: outdoor noon -----------------------------------------------
REM Sun at 66 deg with a neutral-white key and a cool fill -- the hour GOLDEN is
REM not. Deliberately the least flattering hour for this change: with the warm
REM key gone, none of the v3 difference can be a colour trick.
set SCENE=outdoor-noon
set VOXELFORGE_CINE=44,14,44, 44,14,44, 32.5,2.0,29.5, 1
set VOXELFORGE_LOOK_SUN=66,205,20000
set VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00
set VOXELFORGE_LOOK_EXPOSURE=10.6
call :shoot v2 before
call :shoot v3 after
set VOXELFORGE_LOOK_SUN=
set VOXELFORGE_LOOK_LIGHT=
set VOXELFORGE_LOOK_EXPOSURE=

REM ---- scene 2: raking evening sun (the shipped GOLDEN hour) ----------------
REM No hour override at all: this is the frame every gate in look.rs is graded
REM against, so it is the pair the shipped numbers can be checked against. Long
REM shadows across open grass -- where a penumbra that grades with blocker
REM distance either shows or does not.
set SCENE=evening-raking
set VOXELFORGE_CINE=18,9,44, 18,9,44, 31,1.5,29, 1
call :shoot v2 before
call :shoot v3 after

REM ---- scene 3: night, lit by fire -----------------------------------------
REM Hour::NIGHT + the campsite fire, close on the avatar. The one scene where
REM the bloom change and the night IBL/kicker budget are the only lights in
REM frame worth arguing about.
set SCENE=night-firelit
set VOXELFORGE_CINE=36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1
set VOXELFORGE_LOOK_NIGHT=1
call :shoot v2 before
call :shoot v3 after
set VOXELFORGE_LOOK_NIGHT=

echo DONE > "%OUT%\_shoot.done"
endlocal
goto :eof

:shoot
set VOXELFORGE_LOOK_GEN=%~1
set VOXELFORGE_SHOT=%OUT%\%SCENE%_%~2.png
echo [shoot] %SCENE% gen=%~1 -^> %VOXELFORGE_SHOT%
"%EXE%" --play >> "%OUT%\_shoot.log" 2>&1
goto :eof
