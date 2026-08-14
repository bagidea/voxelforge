@echo off
REM ===========================================================================
REM Follow-up probe to the 2x2 grid (flamingo, 2026-08-15).
REM
REM The grid says the missing shadow needs BOTH the scene-1 camera AND the 66
REM deg sun -- neither alone. The leading mechanism is cascade DISTANCE:
REM look.rs:2544 fits 4 cascades with first_cascade_far_bound = 10.0 blocks, and
REM scene 1's eye sits 22.1 units from its look-at point and 12 units above the
REM ground, so every ground pixel it sees is past the tight near cascade. Scene
REM 2's eye is 7.5 units above the ground, so its foreground grass -- exactly
REM where its shadow reads -- lands inside it.
REM
REM This probe holds the scene-1 camera's DIRECTION and target and the 66 deg
REM sun fixed, and only pulls the eye in to 45% of its distance (22.06 -> 9.93
REM units), which drops the ground into the near cascade. Same exe, same rig as
REM the grid.
REM
REM   shadow appears -> the near-cascade bound is the knob
REM   shadow absent   -> distance is not it; look at the frustum's shape instead
REM
REM Scale caveat, stated up front: moving in makes a block ~2.2x wider on screen,
REM so a fixed-pixel OPEN band is more forgiving here. Compare this plate's
REM radius r against the grid's radius r/2.2, not r.
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set EXE=%ROOT%\target-poppy\perf\voxelforge.exe
set OUT=%ROOT%\docs\assets\look\ab-elev
if not exist "%OUT%" mkdir "%OUT%"

set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0
set VOXELFORGE_LOOK_GEN=v3
set VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00
set VOXELFORGE_LOOK_EXPOSURE=10.6
set VOXELFORGE_LOOK_SUN=66,205,20000

set VOXELFORGE_CINE=37.675,7.4,36.025, 37.675,7.4,36.025, 32.5,2.0,29.5, 1
set VOXELFORGE_SHOT=%OUT%\probe-s1cam-near66.png
echo [probe] s1 direction, eye pulled to 9.93u, sun 66 -^> %VOXELFORGE_SHOT%
echo [probe] s1 direction, eye pulled to 9.93u, sun 66 cine=%VOXELFORGE_CINE% >> "%OUT%\_shoot.log"
"%EXE%" --play >> "%OUT%\_shoot.log" 2>&1
endlocal
