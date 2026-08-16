@echo off
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
del /q "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_rose_aiproof_bin.done" 2>nul
set "VOXELFORGE_AIFRAMES=docs/assets/ai/_frames"
set "VOXELFORGE_AILOG=docs/assets/ai/trace.csv"
"target-rose\debug\voxelforge_enemyai_proof.exe" > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_rose_aiproof_bin.log" 2>&1
echo %ERRORLEVEL% > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_rose_aiproof_bin.done"
