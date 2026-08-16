@echo off
REM Look-v5 chain launcher (poppy). Same wrapper reason as _poppy_lookv5_build.cmd.
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
"C:\Program Files\Git\bin\bash.exe" "%ROOT%\scripts\_poppy_lookv5_chain.sh" > "%ROOT%\_poppy_lookv5_chainrun.log" 2>&1
