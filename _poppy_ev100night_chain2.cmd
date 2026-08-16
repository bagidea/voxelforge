@echo off
REM Detached launcher for chain2 (re-shoot; rebuild only when a watched file moves).
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
cd /d "%ROOT%"
del /q "%ROOT%\_poppy_ev100night_chain2.done" 2>nul
"C:\Program Files\Git\bin\bash.exe" "%ROOT%\scripts\_poppy_ev100_night_chain2.sh" > "%ROOT%\_poppy_ev100night_chain2.log" 2>&1
echo CHAIN_EXIT=%ERRORLEVEL% > "%ROOT%\_poppy_ev100night_chain2.done"
