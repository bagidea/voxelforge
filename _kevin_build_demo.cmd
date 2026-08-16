@echo off
REM Kevin save/load demo build — perf profile, own target dir (target-kevin).
REM -j 2 caps link.exe concurrency so this build never trips the 0xc0000142
REM DLL-init false-red the box produces when lanes link at once.
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\client"
set CARGO_TARGET_DIR=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-kevin
del /q "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_kevin_build_demo.done" 2>nul
cargo build --bin voxelforge --profile perf -j 2 > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_kevin_build_demo.log" 2>&1
echo EXIT=%ERRORLEVEL% > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_kevin_build_demo.done"
