@echo off
rem Detached launcher for the husk-AI before/after driver — see the .sh for what it owns.
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
"C:\Program Files\Git\bin\bash.exe" scripts/_flamingo_ai_ba_driver.sh > _fl_ai_driver.log 2>&1
