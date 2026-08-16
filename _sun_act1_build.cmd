@echo off
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\client"
set CARGO_TARGET_DIR=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-quest
set CARGO_PROFILE_DEV_DEBUG=0
del /q "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_sun_act1_build.done" 2>nul
cargo build --bin voxelforge -j 2 > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_sun_act1_build.log" 2>&1
echo EXIT=%ERRORLEVEL% > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_sun_act1_build.done"
