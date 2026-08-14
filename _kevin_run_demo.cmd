@echo off
REM Kevin save/load full-circle + menu-shot evidence — the game prints its OWN proof
REM lines and shoots its OWN PNGs into docs/assets/menu/. Run from repo root so
REM savegame.json / quest_save.json / docs/assets/menu/ all resolve to one place.
cd /d "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"

set EXE=target-kevin\perf\voxelforge.exe
if not exist "%EXE%" (
    echo RUN_DEMO_ABORT no binary %EXE%
    exit /b 1
)

REM The > redirects below need the folder to already exist (the exe only creates
REM it lazily when it writes the PNG, too late for the redirect).
if not exist docs\assets\menu mkdir docs\assets\menu

REM 0. Clean slate: no save on disk => the "before" menu shot has Continue dimmed.
del /q savegame.json quest_save.json 2>nul

REM 1. menu-a-before.png — real menu, Continue dimmed (no save).
set VOXELFORGE_MENUSHOT=before
"%EXE%" > docs\assets\menu\menu-a-before.runlog 2>&1
echo MENUSHOT_BEFORE_EXIT=%ERRORLEVEL%

REM 2. Save run — New Game -> walk -> plant quest flag -> save -> save-before-load.png.
set VOXELFORGE_MENUSHOT=
set VOXELFORGE_SAVE_DEMO=save
"%EXE%" > docs\assets\menu\save-before-load.runlog 2>&1
echo SAVE_DEMO_EXIT=%ERRORLEVEL%

REM 3. menu-b-after.png — real menu, save on disk => Continue lit + slot line.
set VOXELFORGE_SAVE_DEMO=
set VOXELFORGE_MENUSHOT=after
"%EXE%" > docs\assets\menu\menu-b-after.runlog 2>&1
echo MENUSHOT_AFTER_EXIT=%ERRORLEVEL%

REM 4. Continue run — restores the save -> save-after-load.png (same pos/hp/quest).
set VOXELFORGE_MENUSHOT=
set VOXELFORGE_SAVE_DEMO=continue
"%EXE%" > docs\assets\menu\save-after-load.runlog 2>&1
echo CONTINUE_DEMO_EXIT=%ERRORLEVEL%

echo RUN_DEMO_DONE > _kevin_run_demo.done
