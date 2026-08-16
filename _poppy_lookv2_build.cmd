@echo off
REM Look-v2 build (poppy). `perf` profile = opt-level 3 without fat LTO, so the
REM frame is fast enough for a --play capture but the link is minutes, not tens of
REM minutes. Own target dir so it never fights another lane's build lock.
REM -j 2: the box is running 8-10 other cargo lanes; more jobs = 0xc0000142.
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\client"
set CARGO_TARGET_DIR=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-poppy
del /q "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_lookv2_build.done" 2>nul
cargo build --bin voxelforge --profile perf -j 2 > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_lookv2_build.log" 2>&1
echo EXIT=%ERRORLEVEL% > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_lookv2_build.done"
