@echo off
REM Detached lane build for the pixel lane (Flamingo) -- golden beauty shot re-render.
REM Detached on purpose: run_in_background orphans the chain when the session is torn
REM down, and this box has killed five mid-build agents already (docs/LANES.md).
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\scripts\lane-build.ps1" -Lane pixel -Profile release -Bin voxelforge_shot > "_pixel_beauty_build.log" 2>&1
echo LANEBUILD_EXIT=%ERRORLEVEL% >> "_pixel_beauty_build.log"
