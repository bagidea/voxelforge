@echo off
REM Detached launcher for the ev100-night chain. Start-Process mangles
REM `bash -c "<long string>"`, so the chain lives in its own .sh and this file
REM only redirects it. A chain that lives inside a session dies with the session.
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
cd /d "%ROOT%"
del /q "%ROOT%\_poppy_ev100night_chain.done" 2>nul
"C:\Program Files\Git\bin\bash.exe" "%ROOT%\scripts\_poppy_ev100_night_chain.sh" > "%ROOT%\_poppy_ev100night_chain.log" 2>&1
echo CHAIN_EXIT=%ERRORLEVEL% > "%ROOT%\_poppy_ev100night_chain.done"
