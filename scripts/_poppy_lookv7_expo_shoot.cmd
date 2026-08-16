@echo off
REM ===========================================================================
REM Look v7 -- exposure ladder capture (poppy).
REM
REM WHY THIS EXISTS AND NOT JUST _poppy_lookv3_shoot.cmd. The question this
REM round is "does ev100 8.6 survive on ALL THREE plates", and the answer has to
REM come from ONE binary or it is measuring a relink. `VOXELFORGE_LOOK_EXPOSURE`
REM is applied in look.rs AFTER the v3 fork, so driving it here reproduces
REM EXACTLY what a v3-only `EV100_V3` constant would do to the pixels -- without
REM a rebuild per rung.
REM
REM THE AFTER LEG MOVES, THE BEFORE LEG NEVER DOES. A v3-only exposure fork
REM cannot touch the v2 baseline, so a ladder that moved both legs would move
REM the bar and the reading together and measure nothing (same trap
REM TEMPERATURE_V3 was forked to avoid -- see look.rs). Every rung's before
REM plate is shot at the scene's own baseline exposure.
REM
REM Scene poses / suns / lights are copied VERBATIM from
REM _poppy_lookv3_shoot.cmd so a ladder frame is comparable to a shipped plate.
REM
REM Usage: _poppy_lookv7_expo_shoot.cmd <exe> <outdir> <ev_noon> <ev_evening> <ev_night> [mode]
REM        ev_*  = the ev100 the AFTER (v3) leg runs at. "base" = leave the
REM                scene/hour default alone.
REM        mode  = both (default) | after. `after` skips the BEFORE legs, which
REM                are identical at every rung by construction -- 12 boots of a
REM                15-rung ladder spent re-shooting the same three files. Copy
REM                them in from the base run instead.
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set EXE=%~1
set OUT=%~2
set EV_NOON=%~3
set EV_EVE=%~4
set EV_NIGHT=%~5
set MODE=%~6
if "%MODE%"=="" set MODE=both
if "%EXE%"=="" set EXE=%ROOT%\target-poppy\perf\voxelforge.exe
if not exist "%OUT%" mkdir "%OUT%"
del /q "%OUT%\_shoot.done" 2>nul

set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0

REM ---- scene 1: outdoor noon ----------------------------------------------
REM The scene pins exposure at 10.6 (it is part of what makes this hour "noon",
REM not part of the rig), so the BEFORE leg runs 10.6 and the AFTER leg runs the
REM rung. base == 10.6.
set SCENE=outdoor-noon
set VOXELFORGE_CINE=44,14,44, 44,14,44, 32.5,2.0,29.5, 1
set VOXELFORGE_LOOK_SUN=66,205,20000
set VOXELFORGE_LOOK_LIGHT=1.00,0.98,0.93,0.84,0.88,1.00
set VOXELFORGE_LOOK_EXPOSURE=10.6
call :shoot v2 before
if /i "%EV_NOON%"=="base" (set VOXELFORGE_LOOK_EXPOSURE=10.6) else (set VOXELFORGE_LOOK_EXPOSURE=%EV_NOON%)
call :shoot v3 after
set VOXELFORGE_LOOK_SUN=
set VOXELFORGE_LOOK_LIGHT=
set VOXELFORGE_LOOK_EXPOSURE=

REM ---- scene 2: evening raking (Hour::GOLDEN, ev100 10.3) ------------------
set SCENE=evening-raking
set VOXELFORGE_CINE=18,9,44, 18,9,44, 31,1.5,29, 1
call :shoot v2 before
if /i "%EV_EVE%"=="base" (set VOXELFORGE_LOOK_EXPOSURE=) else (set VOXELFORGE_LOOK_EXPOSURE=%EV_EVE%)
call :shoot v3 after
set VOXELFORGE_LOOK_EXPOSURE=

REM ---- scene 3: night, lit by fire (Hour::NIGHT, ev100 7.5) ----------------
set SCENE=night-firelit
set VOXELFORGE_CINE=36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1
set VOXELFORGE_LOOK_NIGHT=1
call :shoot v2 before
if /i "%EV_NIGHT%"=="base" (set VOXELFORGE_LOOK_EXPOSURE=) else (set VOXELFORGE_LOOK_EXPOSURE=%EV_NIGHT%)
call :shoot v3 after
set VOXELFORGE_LOOK_EXPOSURE=
set VOXELFORGE_LOOK_NIGHT=

echo DONE > "%OUT%\_shoot.done"
endlocal
goto :eof

:shoot
set VOXELFORGE_LOOK_GEN=%~1
set VOXELFORGE_SHOT=%OUT%\%SCENE%_%~2.png
echo [shoot] %SCENE% gen=%~1 ev=%VOXELFORGE_LOOK_EXPOSURE% -^> %VOXELFORGE_SHOT%
echo [shoot] %SCENE% gen=%~1 ev=%VOXELFORGE_LOOK_EXPOSURE% >> "%OUT%\_shoot.log"
"%EXE%" --play >> "%OUT%\_shoot.log" 2>&1
goto :eof
