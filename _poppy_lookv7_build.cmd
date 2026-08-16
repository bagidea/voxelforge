@echo off
REM ===========================================================================
REM Look-v7 build launcher (poppy lane) -- night-ambient v3 + the ev100 work.
REM
REM ONE ENTRY POINT ONLY: scripts\lane-build.ps1 -Lane poppy. Nothing in here
REM calls cargo directly, so the busy gate / lock check / grep-'^error' verdict
REM all still apply. -Profile perf because the shoot harness
REM (_poppy_lookv3_shoot.cmd) reads target-poppy\perf\voxelforge.exe.
REM
REM Detached on purpose: a build that lives inside a session dies with it.
REM The .done file is the ONLY completion signal the poller may trust -- the
REM log gets `Compiling` lines written after a failure, so its tail is not a
REM verdict (see lane-build.ps1 header, bug (b)).
REM ===========================================================================
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
cd /d "%ROOT%"
del /q "%ROOT%\_poppy_lookv7_build.done" 2>nul
REM -NoWait: the drain-wait STARVES on this box. Six lanes build in parallel and
REM a lane that does not queue re-takes the machine in the gap between two 15s
REM polls -- measured: pixel's build finished 07:53:50 and rose's
REM voxelforge_enemyai_proof started 07:53:54, so this lane sat at "WAIT other
REM build in progress... 645s" without ever getting a turn. -NoWait is
REM lane-build's own documented escape hatch and it KEEPS the busy gate, which
REM is the check that actually protects target-poppy from a collision; only the
REM politeness wait is skipped. Cost: two concurrent cargo builds instead of
REM one. Accepted knowingly -- the 0xC0000142 headroom failure on this box has
REM only ever been seen with a THIRD build stacked on, and the chain's
REM `grep -c '^error'` + `Finished` check catches it if it does happen.
powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%\scripts\lane-build.ps1" -Lane poppy -Profile perf -NoWait > "%ROOT%\_poppy_lookv7_lane.log" 2>&1
echo LANE_EXIT=%ERRORLEVEL% > "%ROOT%\_poppy_lookv7_build.done"
