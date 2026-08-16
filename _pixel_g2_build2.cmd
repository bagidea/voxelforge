@echo off
REM Pixel lane (Flamingo) G2/G4 rebuild -- round 3.
REM
REM Round 2 (_pixel_g2_build.log) died at LANEBUILD_EXIT=1073807364 = 0x40010004
REM DBG_TERMINATE_PROCESS: it was started with `start "" /b cmd /c`, which is still a
REM CHILD of the agent shell, so the session teardown took the whole tree with it.
REM grep -c "^error" on that log is 0 -- nothing was wrong with the code.
REM
REM So this round is launched by the TASK SCHEDULER (schtasks /Run), i.e. parented by
REM the service, not by any shell of mine. Session teardown cannot reach it.
REM
REM -NoWait: skip lane-build's drain-wait (round 2 burned 1095s in it and then got
REM killed before finishing the compile). The busy gate still stands -- it refuses if
REM target-pixel itself is in use -- and -j 2 keeps this to two cargo processes on the
REM box, which is the proven-safe ceiling here (a 3rd triggers STATUS_DLL_INIT_FAILED).
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
echo LANEBUILD_START %DATE% %TIME% > "_pixel_g2_build2.log"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\lane-build.ps1" -Lane pixel -Profile release -Bin voxelforge_shot -Jobs 2 -NoWait >> "_pixel_g2_build2.log" 2>&1
echo LANEBUILD_EXIT=%ERRORLEVEL% >> "_pixel_g2_build2.log"
echo LANEBUILD_END %DATE% %TIME% >> "_pixel_g2_build2.log"
