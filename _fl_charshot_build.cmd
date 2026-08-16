@echo off
REM Flamingo lane: build voxelforge_charshot into target-flamingo.
REM Separate CARGO_TARGET_DIR so we never contend on the build-lock held by
REM the voxelforge_shot / perf / audio_proof builds running in target\ and
REM target-yama. -j 3 keeps memory pressure down while 3 other cargo builds
REM are live (stacked builds have produced STATUS_DLL_INIT_FAILED before).
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
set CARGO_TARGET_DIR=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-flamingo
del /q "_fl_build.done" 2>nul
cargo build --bin voxelforge_charshot -j 3 > "_fl_build.log" 2>&1
echo %ERRORLEVEL% > "_fl_build.done"
