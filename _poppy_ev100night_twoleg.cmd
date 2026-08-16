@echo off
REM ===========================================================================
REM Poppy -- two-leg ev100 proof on the 2026-08-16T13:53:49Z binary.
REM
REM WHAT THIS ANSWERS. 7399a2a REJECTED Hour::NIGHT.ev100 8.6 and left the
REM source at 7.5 (look.rs:1706). This run proves the REBUILT binary actually
REM carries 7.5 -- at runtime, out of the frame's own provenance line, not from
REM a `strings` scan and not from the commit clock.
REM
REM LEG A is the load-bearing leg: NO lever at all, so `LOOK_FILL ... ev100=`
REM reads `Hour::NIGHT.ev100` straight out of the binary. It must print 7.50.
REM LEG B sets the lever to 8.6 purely as the knob-alive control: if B also
REM printed 7.50 then A's 7.50 would be a dead knob rather than the compiled
REM value, and the whole proof would be vacuous.
REM
REM WHY --play IS NOT RUN BARE. `--play` alone is an interactive window that
REM never exits. `VOXELFORGE_SHOT` makes the app capture one frame and exit, so
REM each leg terminates on its own -- which is what makes this safe to detach
REM behind a `.done` sentinel instead of watched on screen.
REM
REM WHY VOXELFORGE_LOOK_NIGHT=1. The printed ev100 is whichever `Hour` is live.
REM Without the night flag the run picks the day hour and prints 10.30, which
REM would say nothing about the constant under review. gen=v3 pins the rig so
REM the exposure is the only thing that can move between the legs.
REM
REM Env copied from _poppy_ev100_night_shoot.cmd so these legs are comparable
REM to the ladder plates 7399a2a rests on.
REM ===========================================================================
setlocal
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
set EXE=%ROOT%\target-poppy\perf\voxelforge.exe
set OUT=%ROOT%\_poppy_ev100night_twoleg
if not exist "%OUT%" mkdir "%OUT%"
del /q "%OUT%\_legA.done" 2>nul
del /q "%OUT%\_legB.done" 2>nul
del /q "%OUT%\_twoleg.done" 2>nul

set VOXELFORGE_PLAY=1
set VOXELFORGE_NOHUD=1
set VOXELFORGE_LOOK_QUALITY=ultra
set VOXELFORGE_CINE_START=1.0
set VOXELFORGE_LOOK_GEN=v3
set VOXELFORGE_LOOK_NIGHT=1
set VOXELFORGE_CINE=36,3.6,34, 36,3.6,34, 32.5,1.8,29.5, 1

REM ---- LEG A: no lever -- the compiled Hour::NIGHT.ev100 ------------------
set VOXELFORGE_LOOK_EXPOSURE=
echo [legA] no lever ^(VOXELFORGE_LOOK_EXPOSURE unset^) - expect ev100=7.50 > "%ROOT%\_poppy_ev100night_legA.log"
set VOXELFORGE_SHOT=%OUT%\legA.png
"%EXE%" --play >> "%ROOT%\_poppy_ev100night_legA.log" 2>&1
echo LEGA_EXIT=%ERRORLEVEL% > "%OUT%\_legA.done"

REM ---- LEG B: lever 8.6 -- knob-alive control ------------------------------
set VOXELFORGE_LOOK_EXPOSURE=8.6
echo [legB] lever VOXELFORGE_LOOK_EXPOSURE=8.6 - expect ev100=8.60 > "%ROOT%\_poppy_ev100night_legB.log"
set VOXELFORGE_SHOT=%OUT%\legB.png
"%EXE%" --play >> "%ROOT%\_poppy_ev100night_legB.log" 2>&1
echo LEGB_EXIT=%ERRORLEVEL% > "%OUT%\_legB.done"

echo DONE > "%OUT%\_twoleg.done"
endlocal
