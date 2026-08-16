@echo off
REM ===========================================================================
REM Poppy -- Hour::NIGHT.ev100 ladder, EVERY rung on ONE binary.
REM
REM WHY THIS EXISTS. `_poppy_ev100_night_shoot.cmd` shoots exactly two legs:
REM lever=7.5 and no-lever (= whatever is compiled in). That answers "did the
REM source edit land", which it did -- and the gate then said the edit was a
REM REGRESSION on p05/warmth/separation. Answering "then what value is right"
REM needs more than two points, and a rebuild per point would cost six minutes
REM of a box six lanes share. `VOXELFORGE_LOOK_EXPOSURE` is applied in look.rs
REM after the `Hour` constant is picked, so one binary can shoot the whole
REM ladder and only ONE float moves between rungs.
REM
REM EVERY RUNG INCLUDING THE BASELINE SETS THE LEVER EXPLICITLY. The no-lever
REM leg is deliberately NOT reused as the 8.6 rung: mixing "lever" and "no
REM lever" columns puts the lever itself in the comparison. The no-lever leg's
REM job was to prove lever and source are the same knob (it printed ev100=8.60
REM against lever=7.5's ev100=7.50), and that proof is already banked in
REM _poppy_ev100night_pinned/_shoot.log. From here on the lever is the only
REM path, so all rungs are apples to apples.
REM
REM Scene pose / night flag / quality copied VERBATIM from
REM _poppy_ev100_night_shoot.cmd so these frames stay comparable to the pinned
REM before/after pair and to the v7 ladder plates.
REM
REM Usage: _poppy_ev100_night_sweep.cmd <exe> <outdir>
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set EXE=%~1
set OUT=%~2
if "%EXE%"=="" set EXE=%ROOT%\target-poppy\perf\voxelforge.exe
if "%OUT%"=="" set OUT=%ROOT%\_poppy_ev100night_sweep
if not exist "%OUT%" mkdir "%OUT%"
del /q "%OUT%\_sweep.done" 2>nul

set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0
set VOXELFORGE_LOOK_GEN=v3
set VOXELFORGE_LOOK_NIGHT=1
set VOXELFORGE_CINE=36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1
set SCENE=night-firelit

REM ---- the ladder ----------------------------------------------------------
REM 8.6 = the uncommitted edit that failed the gate (kept as the top of the
REM ladder so the failing direction stays visible in the table rather than
REM being quietly dropped). 7.5 = shipped baseline. Below it, the untested
REM direction: raising ev100 darkens, so brightening means going DOWN.
call :shoot 8.6 ev86
call :shoot 7.5 ev75
call :shoot 7.0 ev70
call :shoot 6.5 ev65
call :shoot 6.0 ev60

echo DONE > "%OUT%\_sweep.done"
endlocal
goto :eof

:shoot
set VOXELFORGE_LOOK_EXPOSURE=%~1
set VOXELFORGE_SHOT=%OUT%\%SCENE%_%~2.png
echo [sweep] %SCENE% %~2 gen=v3 ev=%~1 -^> %VOXELFORGE_SHOT%
echo [sweep] %SCENE% %~2 gen=v3 ev=%~1 >> "%OUT%\_sweep.log"
"%EXE%" --play >> "%OUT%\_sweep.log" 2>&1
goto :eof
