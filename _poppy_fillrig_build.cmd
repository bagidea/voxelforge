@echo off
REM Fill-rig v2 build, retry #2 (poppy).
REM
REM WHY -j 2 AND NOT -j 4: retry #1 (`_poppy_lookv2_build.log`) died on two E0594s
REM in vfx.rs:1412 that are NOT on disk any more — cargo read that file while the
REM vfx lane was mid-save. Nothing to fix; just re-read the tree. But the box is at
REM 28.7/32.2 GB commit with three other lanes linking, and that is the exact
REM condition that turns a build into a false 0xc0000142 DLL-init red. Two jobs.
REM
REM Same CARGO_TARGET_DIR as retry #1 on purpose — every dep is already built there,
REM so this run is voxelforge + sim + link, not another full bevy compile.
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\client"
set CARGO_TARGET_DIR=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-poppy
del /q "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_fillrig_build.done" 2>nul
cargo build --bin voxelforge --profile perf -j 2 > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_fillrig_build.log" 2>&1
echo EXIT=%ERRORLEVEL% > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_fillrig_build.done"
