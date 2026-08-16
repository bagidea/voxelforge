@echo off
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
set CARGO_TARGET_DIR=target-rose
set CARGO_PROFILE_DEV_DEBUG=0
del /q "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_rose_aiproof.done" 2>nul
cargo build -j 2 --bin voxelforge_enemyai_proof > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_rose_aiproof_build.log" 2>&1
echo EXIT=%ERRORLEVEL% >> "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_rose_aiproof_build.log"
echo %ERRORLEVEL% > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_rose_aiproof.done"
