# _rose_look_beauty_capture.ps1 -- Rose's prep for Poppy: clean look-lane BEAUTY
# VISTA still, straight to a gradeable PNG via VOXELFORGE_SHOT.
#
# ASCII ONLY. Windows PowerShell 5.1 reads .ps1 as ANSI; a UTF-8 em-dash inside a
# string literal is a parse error. Keep this file ASCII.
#
# WHY THIS EXISTS (task: "capture beauty shot via the VOXELFORGE_NOHUD/CINE lane,
# hand to Poppy the moment the exe is green"):
#   * The canonical vista framing is VOXELFORGE_LOOK_CAM=35,-18,26 -- the SAME
#     value every prior audit/sweep (look-audit-2026-08-05, vista_grade_sweep,
#     verify_baked_grade, haze-sweep-g7b) and grade_character.py --cam use. So a
#     frame shot here drops straight into _rose_look_regrade.py and compares
#     apples-to-apples.
#   * VOXELFORGE_SHOT saves a PNG directly from the MAIN exe at 3.2s (main.rs:2636)
#     and self-exits at 4.4s (main.rs:2643 AppExit::Success). No mkv pull, no
#     gdigrab, no process kill.
#   * TWO de-HUD layers, on purpose:
#       1. VOXELFORGE_NOHUD=1  (scene.rs:269) hides widgets LIVE -- crosshair,
#          FPS, HP/ST bars, gizmo aim-box, egui, quest prompts -- so no widget
#          pixel is ever baked into the frame (the old risk: a late-spawned
#          prompt that a post-process detector races and misses).
#       2. _flamingo_dehud2.py STILL runs after, because it CROPS the top 80px
#          (HUD_BAND). Every baseline number in _rose_look_regrade.py was graded
#          on a cropped 1280x640 frame, so the new frame must be cropped the same
#          way or the band axes (warmth/sat/p05-L) do not line up. With NOHUD on,
#          dehud2's inpaint steps are a safe no-op; only the crop fires.
#
# LANE + SAFETY RULES (Rose's, from office memory + the Director's brief):
#   * NO cargo. The exe is a FROZEN COPY of Sun's target-flamingo build -- never
#     the linker's live output (running target-flamingo/release/voxelforge.exe
#     holds a file lock another lane's linker needs; that already cost this
#     project one false "build green").
#   * Targets target-flamingo (the build the Director told Rose to wait on),
#     NOT target-rose -- _rose_finish_20260806.sh waits on target-rose, which has
#     no exe and is the orphaned lane. Do not run that finisher unedited.
#   * STALENESS GATE first: refuses to shoot if the exe is older than any
#     client/src/*.rs (a stale binary grades a look that is not in the exe).
#
# Output: _rose_look_beauty/<Tag>.png (raw) + <Tag>-nohud2.png (graded) +
#         <Tag>.png.runlog (provenance) + <Tag>.log (exe stdout)
param(
    [string]$ExePath = "target-flamingo\release\voxelforge.exe",
    [string]$Tag     = "grade-vista-new",   # stem "grade-vista" matches _rose_look_regrade.py BASELINE
    [switch]$DryRun
)
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $Root
try {

$OutDir = "_rose_look_beauty"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Png    = Join-Path $Root "$OutDir\$Tag.png"
$Runlog = "$Png.runlog"
$Log    = Join-Path $Root "$OutDir\$Tag.log"

# ---- 0. STALENESS GATE (refuse a stale binary before it lies on the grade) ----
if (-not (Test-Path $ExePath)) { throw "EXE NOT FOUND: $ExePath -- is Sun's target-flamingo build green yet?" }
$exeItem  = Get-Item $ExePath
$newest   = Get-ChildItem "client\src\*.rs" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$stale    = $exeItem.LastWriteTime -lt $newest.LastWriteTime
$head     = try { (git rev-parse --short HEAD) } catch { "unknown" }
"STALE=$stale EXE=$($exeItem.LastWriteTime.ToString('MM-dd_HH:mm')) SRC=$($newest.Name)@$($newest.LastWriteTime.ToString('MM-dd_HH:mm')) HEAD=$head"
if ($stale -and -not $DryRun) {
    throw "STALE BINARY: exe ($($exeItem.LastWriteTime.ToString('HH:mm'))) is older than $($newest.Name) ($($newest.LastWriteTime.ToString('HH:mm'))). Sun's build has not finished linking the voxelforge crate yet. NOT shooting -- wait."
}

# ---- 1. binary provenance ----
$sha = (Get-FileHash -Path $ExePath -Algorithm SHA256).Hash
$envLine = "VOXELFORGE_PLAY=1 VOXELFORGE_NOHUD=1 VOXELFORGE_LOOK_CAM=35,-18,26 VOXELFORGE_LOOK_QUALITY=ultra VOXELFORGE_SHOT=$Tag.png"
$meta = @"
exe:        $ExePath
exe_mtime:  $($exeItem.LastWriteTime.ToString('o'))
exe_sha256: $sha
commit:     $head
shot_env:   $envLine
note:       NOHUD live + dehud2 crop after (geometry-matched to regrade baseline)
---
"@

if ($DryRun) {
    "$meta`n[DryRun -- exe not launched]" | Out-File -FilePath $Runlog -Encoding utf8
    "DRYRUN OK: would shoot $Tag.png from $ExePath, then dehud2 -> $Tag-nohud2.png. No exe launched."
    return
}

# ---- 2. frozen copy (never run the linker's live output) ----
$Probe = Join-Path $Root "$OutDir\vfprobe_$Tag.exe"
Copy-Item $ExePath $Probe -Force

# ---- 3. shoot: clean HUD + canonical vista framing, straight to PNG ----
$env:VOXELFORGE_PLAY         = "1"
$env:VOXELFORGE_NOHUD        = "1"
$env:VOXELFORGE_LOOK_CAM     = "35,-18,26"
$env:VOXELFORGE_LOOK_QUALITY = "ultra"
$env:VOXELFORGE_SHOT         = $Png

$p = Start-Process -FilePath $Probe -PassThru -WindowStyle Minimized `
      -RedirectStandardOutput $Log -RedirectStandardError "$Log.err"
$p.WaitForExit(30000) | Out-Null        # exe self-exits ~4.4s; 30s hard ceiling
if (-not $p.HasExited) { $p.Kill(); throw "exe did not self-exit in 30s -- killed. See $Log" }
$rc = $p.ExitCode

# clear env so it cannot leak into a later non-capture run
Remove-Item Env:\VOXELFORGE_PLAY, Env:\VOXELFORGE_NOHUD, Env:\VOXELFORGE_LOOK_CAM, `
             Env:\VOXELFORGE_LOOK_QUALITY, Env:\VOXELFORGE_SHOT -ErrorAction SilentlyContinue

# ---- 4. write runlog (provenance + raw exe stdout) ----
$stdout = if (Test-Path $Log) { Get-Content $Log -Raw } else { "" }
"$meta`n--- exe stdout ---`n$stdout" | Out-File -FilePath $Runlog -Encoding utf8

# ---- 5. artifact + sanity checks ----
$miss  = -not (Test-Path $Png)
$bytes = if ($miss) { 0 } else { (Get-Item $Png).Length }
$saved = Select-String -Path $Log,"$Log.err" -Pattern 'SHOT saved to' -Quiet -ErrorAction SilentlyContinue
$panic = Select-String -Path $Log,"$Log.err" -Pattern 'panicked|RUST_BACKTRACE|B0001' -Quiet -ErrorAction SilentlyContinue
"exit=$rc png_bytes=$bytes SHOT_saved=$saved panic=$panic"
if ($bytes -eq 0 -or -not $saved) { throw "CAPTURE FAILED: no PNG (exit=$rc, SHOT_saved=$saved). See $Log" }
if ($panic) { Write-Warning "PANIC/warn signature in log -- frame may be bad. See $Log" }

# ---- 6. de-HUD crop (geometry-matched to the regrade baseline) ----
# NB: NO `2>&1` here -- on PS 5.1 that merges python's stderr (a Pillow
# DeprecationWarning) into the success stream and $ErrorActionPreference=Stop
# turns every warning line into a NativeCommandError, aborting the script AFTER
# the shot already succeeded. Redirect stderr to a file; check $LASTEXITCODE.
python "scripts\_flamingo_dehud2.py" $Png 2>"$Png.dehud2.err"
if ($LASTEXITCODE -ne 0) { throw "dehud2 exited $LASTEXITCODE -- see $Png.dehud2.err" }
$Graded = "$([System.IO.Path]::GetDirectoryName($Png))\$([System.IO.Path]::GetFileNameWithoutExtension($Png))-nohud2.png"
if (-not (Test-Path $Graded)) { throw "dehud2 produced no -nohud2.png from $Png" }

""
"OK  RAW=$Png"
"OK  GRADED=$Graded  ($(Get-Item $Graged).Length bytes)"
"RUNLOG=$Runlog"
"NEXT -- Rose look re-grade (diffs vs scorecard baseline):"
"  python scripts\_rose_look_regrade.py $Graded > _rose_look_beauty\regrade.md"
"  # add gate3 interior re-captures (separate framing/scene) to the same run for G3 gap #1."

} finally { Pop-Location }
