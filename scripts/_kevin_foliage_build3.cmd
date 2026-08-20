@echo off
set CARGO_TARGET_DIR=target-kevin
cargo build --release --bin voxelforge -j 2 > scripts\_kevin_foliage_build3.log 2>&1
echo BUILD_EXIT=%ERRORLEVEL%>>"scripts\_kevin_foliage_build3.log"
