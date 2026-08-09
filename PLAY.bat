@echo off
rem ===========================================================================
rem  Voxelforge - double-click launcher
rem
rem  Boots straight into the playable Act 1 scene (--play), no editor.
rem  Works from any location: %~dp0 is this .bat's own folder, so the CWD is
rem  always the project root - the game loads assets/story/act1.json and
rem  maps/edhari.json by relative path and would fail from anywhere else.
rem
rem  `start "" ...` hands the exe off to its own process, so this console
rem  window closes the moment the game is up instead of hanging around.
rem
rem  NOTE: keep this file CRLF + plain ASCII. cmd.exe mis-splits LF-only
rem  batch lines (rem -> re + m) and garbles non-ASCII in the OEM codepage.
rem ===========================================================================

cd /d "%~dp0"

if not exist "target\release\voxelforge.exe" (
    echo.
    echo   voxelforge.exe not found.
    echo   Expected: %~dp0target\release\voxelforge.exe
    echo.
    echo   Build it first:  cargo build --release --bin voxelforge
    echo.
    pause
    exit /b 1
)

start "Voxelforge" "%~dp0target\release\voxelforge.exe" --play
exit /b 0
