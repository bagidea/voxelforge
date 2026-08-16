@echo off
REM Detached launcher for the v7 chain (wait build -> verdict -> relink proof ->
REM ev100 ladder). Start-Process mangles `bash -c "<long string>"`, so the chain
REM lives in its own .sh and this file only redirects it.
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
cd /d "%ROOT%"
del /q "%ROOT%\_poppy_lookv7_chain.done" 2>nul
"C:\Program Files\Git\bin\bash.exe" "%ROOT%\scripts\_poppy_lookv7_chain.sh" > "%ROOT%\_poppy_lookv7_chain.log" 2>&1
echo CHAIN_EXIT=%ERRORLEVEL% > "%ROOT%\_poppy_lookv7_chain.done"
