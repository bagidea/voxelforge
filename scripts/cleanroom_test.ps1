<#
.SYNOPSIS
  Clean-room test: prove the packed folder boots to the game on a machine that
  has no repo, no cargo, no source tree.

.DESCRIPTION
  Copies the pack produced by scripts\pack_win64.ps1 into a scratch folder
  OUTSIDE the repository (default: %TEMP%\voxelforge-cleanroom) and launches it
  there. Each case runs the shipped exe with VOXELFORGE_SHOT set, which is the
  client's own deterministic capture path (main.rs screenshot_once: save PNG at
  t=3.2 s, AppExit at t=4.4 s) - so a case that reaches the game screen exits by
  itself and leaves a PNG as proof.

  The environment is scrubbed first: BEVY_ASSET_ROOT and CARGO_MANIFEST_DIR both
  override bevy's asset root (bevy_asset io\file\mod.rs get_base_path), so a dev
  box with either one set would "pass" for the wrong reason. VOXELFORGE_PLAY is
  scrubbed too, and never set by this harness: game mode has to come from the
  same place the player's does - the `--play` argument on the Steam launch
  option, or the one baked into run-voxelforge.cmd. A harness that exports
  VOXELFORGE_PLAY itself would green-light a depot that boots the editor.

  ASCII only - Windows PowerShell 5.1 parses .ps1 as ANSI without a BOM.

  Cases:
    hero      pack intact, VOXELFORGE_HERO        EXPECT PASS  (control: proves the
                                                  exe launches, resolves its DLLs
                                                  and reaches the GPU with no repo,
                                                  using main.rs's small hero App.
                                                  This is the ONE case that uses an
                                                  env var - it is a diagnostic, not
                                                  a player path, and hero.rs touches
                                                  no assets, so it proves nothing
                                                  about the packed data files.)
    play      exe --play, CWD = pack folder       EXPECT PASS  (the Steam config:
                                                  same exe + same argument the
                                                  launch option in section 4 sets)
    nostory   exe --play, assets\story removed    EXPECT FAIL  (proves act1.json
                                                  is a hard dependency)
    wrongcwd  exe --play, CWD = C:\               EXPECT FAIL  (proves the client
                                                  reads maps\ + assets\story\
                                                  relative to the CWD, not the exe)
    launcher  run-voxelforge.cmd from CWD = C:\   EXPECT PASS  (no arguments, no
                                                  env: the .cmd has to supply
                                                  --play and pin the CWD by itself.
                                                  The .cmd must NOT use `start` -
                                                  a detached child writes to its
                                                  own console and this case would
                                                  read an empty log forever.)

  The launcher case is the one that matters for a Steam shortcut / desktop icon.

  A pass needs four things, not one: exit 0, a proof PNG, no panic, and no bevy
  asset-load error in the log. That last one is what makes `play` a test of the
  PACK and not just of the exe - the wavs and .vox live under assets\ and are
  resolved against the exe directory, so a missing one shows up as
  "Path not found" / AssetReaderError rather than a crash.

.PARAMETER Pack
  Folder produced by pack_win64.ps1. Default: _ship\voxelforge-win64

.PARAMETER Room
  Scratch folder to run in. Must be outside the repo.

.PARAMETER Case
  Which case(s) to run. Default: all.

.EXAMPLE
  powershell -File scripts\cleanroom_test.ps1
  powershell -File scripts\cleanroom_test.ps1 -Case play
#>
[CmdletBinding()]
param(
    [string]   $Pack = "_ship\voxelforge-win64",
    [string]   $Room = (Join-Path $env:TEMP "voxelforge-cleanroom"),
    [string[]] $Case = @("hero", "play", "nostory", "wrongcwd", "launcher"),
    [int]      $TimeoutSec = 120
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo

if (-not (Test-Path $Pack)) { throw "pack not found: $Pack (run scripts\pack_win64.ps1 first)" }
$packFull = (Resolve-Path $Pack).Path
if ($Room.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase)) {
    throw "clean room must live OUTSIDE the repo, got: $Room"
}

# A stale voxelforge is another lane's capture - never race it.
$busy = @(Get-Process -Name voxelforge -ErrorAction SilentlyContinue)
if ($busy.Count -gt 0) { throw "voxelforge.exe already running (pid $($busy[0].Id)) - another lane owns the box" }

# Evidence lives OUTSIDE the room: every case wipes and re-copies the room, so
# logs and proof PNGs written inside it would be destroyed by the next case.
$Evidence = "$Room-evidence"
New-Item -ItemType Directory -Force -Path $Evidence | Out-Null

Write-Host "CLEANROOM  pack     = $packFull"
Write-Host "CLEANROOM  room     = $Room"
Write-Host "CLEANROOM  evidence = $Evidence"
Write-Host ""

function Reset-Room {
    if (Test-Path $Room) { Remove-Item $Room -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $Room | Out-Null
    Copy-Item (Join-Path $packFull "*") -Destination $Room -Recurse -Force
}

# Every case, including `launcher`, runs synchronously under cmd.exe with stdout
# and stderr redirected to files. That only works because run-voxelforge.cmd
# calls the exe directly instead of via `start` - a detached `start` would put
# the game in its own console and this harness would have nothing to read, which
# is exactly how the launcher case used to come back "inconclusive" forever.
function Invoke-Case($name, $workDir, $exePath, $exeArgs = "") {
    $shot = Join-Path $Evidence "proof_$name.png"
    if (Test-Path $shot) { Remove-Item $shot -Force }
    $outLog = Join-Path $Evidence "run_$name.out.txt"
    $errLog = Join-Path $Evidence "run_$name.err.txt"

    # Scrub the env vars that would silently redirect bevy's asset root or switch
    # the game on behind the launcher's back, then ask for a self-terminating
    # screenshot. VOXELFORGE_PLAY is deliberately NOT set: game mode must arrive
    # the way it does for a player, as the --play argument in $exeArgs (Steam
    # launch option) or from inside run-voxelforge.cmd.
    Remove-Item Env:\BEVY_ASSET_ROOT -ErrorAction SilentlyContinue
    Remove-Item Env:\CARGO_MANIFEST_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\VOXELFORGE_PLAY, Env:\VOXELFORGE_HERO -ErrorAction SilentlyContinue
    if ($name -eq "hero") { $env:VOXELFORGE_HERO = "1" }
    $env:VOXELFORGE_SHOT = $shot
    $env:VOXELFORGE_LOOK_QUALITY = "high"

    # Run through cmd.exe so stdout/stderr land in files, and start it via .NET
    # rather than Start-Process: PowerShell's Start-Process only captures an exit
    # code when -Wait is used, so `$p.ExitCode` reads back as $null on the
    # -PassThru + timeout path (verified on 5.1). cmd's exit code IS the exe's.
    $cmdLine = '/c ""{0}" {1} > "{2}" 2> "{3}""' -f $exePath, $exeArgs, $outLog, $errLog
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "cmd.exe"
    $psi.Arguments = $cmdLine
    $psi.WorkingDirectory = $workDir
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $shell = [System.Diagnostics.Process]::Start($psi)
    if (-not $shell.WaitForExit($TimeoutSec * 1000)) {
        Write-Host "  (timeout after ${TimeoutSec}s - killing)" -ForegroundColor Yellow
        Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force
        $shell.WaitForExit()
    }
    $p = $shell
    $sw.Stop()

    Remove-Item Env:\VOXELFORGE_PLAY, Env:\VOXELFORGE_HERO, Env:\VOXELFORGE_SHOT, Env:\VOXELFORGE_LOOK_QUALITY -ErrorAction SilentlyContinue

    $out = ""
    if (Test-Path $outLog) { $out = Get-Content $outLog -Raw }
    $err = ""
    if (Test-Path $errLog) { $err = Get-Content $errLog -Raw }
    $blob = "$out`n$err"

    if ([string]::IsNullOrWhiteSpace($blob)) {
        Write-Host "  (no output captured - the process detached its console?)" -ForegroundColor Yellow
    }

    $shotOk = (Test-Path $shot) -and ((Get-Item $shot).Length -ge 2048)
    $panicked = $blob -match "panicked"
    $storyOk = $blob -match "STORY_LOAD ok"
    # bevy_asset does not panic on a missing file - it logs and hands back a
    # failed handle, so a depot that forgot the wavs would otherwise render a
    # perfectly good silent PNG and "pass". This is the check that turns the
    # pack audit from static reasoning into a measurement.
    # Kept to strings bevy_asset actually emits - a looser needle like a bare
    # "NotFound" would trip on unrelated wgpu/winit chatter and fail a good pack.
    $assetErr = $blob -match "Path not found|AssetReaderError|Failed to load asset|MissingAssetLoader"

    # cmd's exit code is the exe's; "n/a" only if the handle went away on us.
    $code = "n/a"
    $codeOk = $true
    if ($p -and $null -ne $p.ExitCode) {
        $code = $p.ExitCode
        $codeOk = ($p.ExitCode -eq 0)
    }

    [pscustomobject]@{
        Case      = $name
        ExitCode  = $code
        Seconds   = [math]::Round($sw.Elapsed.TotalSeconds, 1)
        ShotOk    = $shotOk
        ShotBytes = $(if (Test-Path $shot) { (Get-Item $shot).Length } else { 0 })
        StoryOk   = $storyOk
        Panicked  = $panicked
        AssetErr  = $assetErr
        Pass      = ($codeOk -and $shotOk -and -not $panicked -and -not $assetErr)
        Log       = $outLog
        Shot      = $shot
    }
}

$results = @()

if ($Case -contains "hero") {
    # Control: VOXELFORGE_HERO takes main.rs's `if cfg.hero` branch, which builds a
    # far smaller App. It proves the packed folder can launch the exe, resolve its
    # DLL imports and reach the GPU with no repo in sight - so a failure in the
    # `play` cases below is about the game code, not about the pack. It says
    # nothing about the packed DATA: hero.rs contains no `asset_server` call at
    # all, so a hero pass never opens a single file the packer copied.
    Write-Host "=== CASE hero        (control: exe + GPU, no repo, NO assets read) ==="
    Reset-Room
    $results += Invoke-Case "hero" $Room (Join-Path $Room "voxelforge.exe")
}

if ($Case -contains "play") {
    Write-Host "=== CASE play        (exe --play, CWD = pack: the Steam config) ==="
    Reset-Room
    $results += Invoke-Case "play" $Room (Join-Path $Room "voxelforge.exe") "--play"
}

if ($Case -contains "nostory") {
    Write-Host "=== CASE nostory     (exe --play, assets\story removed) ==="
    Reset-Room
    Remove-Item (Join-Path $Room "assets\story") -Recurse -Force
    $results += Invoke-Case "nostory" $Room (Join-Path $Room "voxelforge.exe") "--play"
}

if ($Case -contains "wrongcwd") {
    Write-Host "=== CASE wrongcwd    (exe --play, CWD = C:\) ==="
    Reset-Room
    $results += Invoke-Case "wrongcwd" "C:\" (Join-Path $Room "voxelforge.exe") "--play"
}

if ($Case -contains "launcher") {
    # No arguments and no env on purpose - if run-voxelforge.cmd does not pass
    # --play itself, this case boots the editor and StoryOk stays false.
    Write-Host "=== CASE launcher    (run-voxelforge.cmd, CWD = C:\, no args, no env) ==="
    Reset-Room
    $results += Invoke-Case "launcher" "C:\" (Join-Path $Room "run-voxelforge.cmd")
}

Write-Host ""
$results | Format-Table Case, Pass, ExitCode, Seconds, ShotOk, ShotBytes, StoryOk, Panicked, AssetErr -AutoSize
# StoryOk is the game-mode witness: quest.rs only prints STORY_LOAD when the
# client is in --play. A case that renders a PNG with StoryOk=False booted the
# editor sandbox, which for `play` and `launcher` is a failure of the depot
# config even though the exe itself is fine.
foreach ($r in $results) {
    if ($r.Case -in @("play", "launcher") -and -not $r.StoryOk) {
        Write-Host "CLEANROOM  WARN  case '$($r.Case)' never printed STORY_LOAD - it booted the EDITOR, not the game" -ForegroundColor Yellow
    }
}
Write-Host "CLEANROOM  logs + proof PNGs in $Evidence"
