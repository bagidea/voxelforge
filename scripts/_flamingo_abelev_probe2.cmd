@echo off
REM ===========================================================================
REM Probe B (flamingo, 2026-08-15). Probe A framed itself into the ruin -- it
REM pulled the eye 55% down the view ray and the frame came back walls, with
REM almost no open grass left to receive a shadow. Inconclusive by FRAMING, not
REM by physics, and it stays on disk saying so.
REM
REM Probe B changes the one thing the cascade hypothesis is about -- how far the
REM ground is from the eye -- while keeping MORE open grass in frame, not less.
REM Same X/Z and the same look-at point as scene 1; the eye drops y 14 -> 6, so
REM height above the grass goes 12 -> 4 units and the foreground ground lands
REM inside first_cascade_far_bound = 10.0 (look.rs:691). Sun stays at 66 deg.
REM
REM   shadow on open grass -> the near-cascade bound is the knob
REM   still nothing        -> distance is not it
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

set VOXELFORGE_CINE=44,6,44, 44,6,44, 32.5,2.0,29.5, 1

set VOXELFORGE_LOOK_SUN=66,205,20000
set VOXELFORGE_SHOT=%OUT%\probeB-lowcam-elev66.png
echo [probeB] s1 x/z, eye y=6, sun 66 -^> %VOXELFORGE_SHOT%
echo [probeB] s1 x/z, eye y=6, sun 66 cine=%VOXELFORGE_CINE% >> "%OUT%\_shoot.log"
"%EXE%" --play >> "%OUT%\_shoot.log" 2>&1
endlocal
