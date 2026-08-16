@echo off
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
set CARGO_PROFILE_DEV_DEBUG=0
cargo build -p voxelforge --bin voxelforge -j 4 > _rose_after_build.log 2>&1
echo EXIT=%ERRORLEVEL% >> _rose_after_build.log
echo done > _rose_after_build.done
