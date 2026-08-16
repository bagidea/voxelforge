@echo off
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\client"
set CARGO_TARGET_DIR=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-quest
set CARGO_PROFILE_DEV_DEBUG=0
del /q "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_look_check.done" 2>nul
cargo check --bin voxelforge -j 4 > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_look_check.log" 2>&1
echo EXIT=%ERRORLEVEL% > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_look_check.done"
