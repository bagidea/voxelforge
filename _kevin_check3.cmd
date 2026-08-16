@echo off
REM Kevin main_menu/save_game compile check #3 (after cross-lane compile-fixes).
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\client"
set CARGO_TARGET_DIR=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-kevin
del /q "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_kevin_check3.done" 2>nul
cargo check --bin voxelforge -j 2 > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_kevin_check3.log" 2>&1
echo EXIT=%ERRORLEVEL% > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_kevin_check3.done"
