@echo off
rem Rose - one detached release build of the client into target-rose.
rem Started via Start-Process so it survives the agent session exiting
rem mid-link (the scar from 2026-08-18: the run_in_background build was
rem orphaned with no log, no exit code, no exe). Exit code lands in the
rem log as BUILD_EXIT=N for the grep -c '^error' verdict.
cd /d E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
echo BUILD_START %DATE% %TIME% > _rose_water_build.log
cargo build --release -p voxelforge --target-dir target-rose >> _rose_water_build.log 2>&1
echo BUILD_EXIT=%ERRORLEVEL%>> _rose_water_build.log
