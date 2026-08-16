@echo off
REM Kevin main_menu/save_game compile check (detached, non-blocking).
REM
REM WHY -j 2: the box is running several lanes' builds at once right now, and the
REM same 0xc0000142 DLL-init false-red the fillrig build hit (see
REM _poppy_fillrig_build.cmd) turns up when too many link.exe run concurrently.
REM `check` still links the proc-macro host DLLs, so cap parallelism.
REM
REM WHY --bin voxelforge: the other six bins belong to other lanes
REM (shot/perf/audio_proof/charshot/enemyai_proof/vfx_proof) and each
REM `#[path]`-includes its own module set — checking them would surface
REM unrelated lanes' breakage as my error count.
REM
REM target-kevin keeps this check off every other lane's build lock.
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\client"
set CARGO_TARGET_DIR=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-kevin
del /q "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_kevin_check2.done" 2>nul
cargo check --bin voxelforge -j 2 > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_kevin_check2.log" 2>&1
echo EXIT=%ERRORLEVEL% > "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_kevin_check2.done"
