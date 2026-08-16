@echo off
REM Detached lane build for the pixel lane (Flamingo) -- G2 mullion shadow bars + G4 penumbra.
REM Detached on purpose: a build started inside the agent session dies WITH the session
REM (that is what killed the 07:5x round-two build, not a compile error). `start "" /b cmd /c`
REM on this file survives it. Same pattern as _pixel_beauty_build.cmd.
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\lane-build.ps1" -Lane pixel -Profile release -Bin voxelforge_shot > "_pixel_g2_build.log" 2>&1
echo LANEBUILD_EXIT=%ERRORLEVEL% >> "_pixel_g2_build.log"
