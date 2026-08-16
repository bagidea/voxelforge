@echo off
REM Look-v5 build launcher (poppy). Thin wrapper so the queue+build script gets
REM launched by cmd.exe -- Start-Process splitting an ArgumentList on spaces
REM mangles `bash -c "<one long string>"` and the redirect lands nowhere.
set ROOT=E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge
"C:\Program Files\Git\bin\bash.exe" "%ROOT%\scripts\_poppy_lookv5_build.sh" > "%ROOT%\_poppy_lookv5_chain.log" 2>&1
