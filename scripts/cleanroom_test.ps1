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
  box with either one set would "pass" for the wrong reason.

  ASCII only - Windows PowerShell 5.1 parses .ps1 as ANSI without a BOM.

  Cases:
    hero      pack intact, VOXELFORGE_HERO        EXPECT PASS  (control: proves the
                                                  exe launches, resolves its DLLs
                                                  and reaches the GPU with no repo,
                                                  using main.rs's small hero App)
    play      pack intact, CWD = pack folder      EXPECT PASS  (the ship config)
    nostory   assets\story removed                EXPECT FAIL  (proves act1.json
                                                  is a hard dependency)
    wrongcwd  pack intact, CWD = C:\              EXPECT FAIL  (proves the client
                                                  reads maps\ + assets\story\
                                                  relative to the CWD, not the exe)
    launcher  run-voxelforge.cmd from CWD = C:\   EXPECT PASS  (the .cmd pins CWD)

  The launcher case is the one that matters for a Steam shortcut / desktop icon.

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

function Invoke-Case($name, $workDir, $exePath, [switch]$Detached) {
    $shot = Join-Path $Evidence "proof_$name.png"
    if (Test-Path $shot) { Remove-Item $shot -Force }
    $outLog = Join-Path $Evidence "run_$name.out.txt"
    $errLog = Join-Path $Evidence "run_$name.err.txt"

    # Scrub the two env vars that would silently redirect bevy's asset root, then
    # ask for a --play session with a self-terminating screenshot.
    Remove-Item Env:\BEVY_ASSET_ROOT -ErrorAction SilentlyContinue
    Remove-Item Env:\CARGO_MANIFEST_DIR -ErrorAction SilentlyContinue
    Remove-Item Env:\VOXELFORGE_PLAY, Env:\VOXELFORGE_HERO -ErrorAction SilentlyContinue
    if ($name -eq "hero") { $env:VOXELFORGE_HERO = "1" } else { $env:VOXELFORGE_PLAY = "1" }
    $env:VOXELFORGE_SHOT = $shot
    $env:VOXELFORGE_LOOK_QUALITY = "high"

    # Run through cmd.exe so stdout/stderr land in files, and start it via .NET
    # rather than Start-Process: PowerShell's Start-Process only captures an exit
    # code when -Wait is used, so `$p.ExitCode` reads back as $null on the
    # -PassThru + timeout path (verified on 5.1). cmd's exit code IS the exe's.
    $cmdLine = '/c ""{0}" > "{1}" 2> "{2}""' -f $exePath, $outLog, $errLog
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "cmd.exe"
    $psi.Arguments = $cmdLine
    $psi.WorkingDirectory = $workDir
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $shell = [System.Diagnostics.Process]::Start($psi)
    $p = $null
    if ($Detached) {
        # run-voxelforge.cmd uses `start`, so no console lingers for the player -
        # which means the .cmd returns immediately. Wait on the game it spawned.
        $shell.WaitForExit(30000) | Out-Null
        for ($i = 0; $i -lt 40 -and -not $p; $i++) {
            $p = @(Get-Process -Name voxelforge -ErrorAction SilentlyContinue)[0]
            if (-not $p) { Start-Sleep -Milliseconds 500 }
        }
        if ($p) {
            if (-not $p.WaitForExit($TimeoutSec * 1000)) {
                Write-Host "  (timeout after ${TimeoutSec}s - killing)" -ForegroundColor Yellow
                $p.Kill(); $p.WaitForExit()
            }
        } else {
            # The game either never started or died faster than the 500 ms poll.
            # Either way the PNG check below is the verdict.
            Write-Host "  (launcher: no live voxelforge seen - judging by the PNG)" -ForegroundColor Yellow
        }
    } else {
        if (-not $shell.WaitForExit($TimeoutSec * 1000)) {
            Write-Host "  (timeout after ${TimeoutSec}s - killing)" -ForegroundColor Yellow
            Get-Process -Name voxelforge -ErrorAction SilentlyContinue | Stop-Process -Force
            $shell.WaitForExit()
        }
        $p = $shell
    }
    $sw.Stop()

    Remove-Item Env:\VOXELFORGE_PLAY, Env:\VOXELFORGE_HERO, Env:\VOXELFORGE_SHOT, Env:\VOXELFORGE_LOOK_QUALITY -ErrorAction SilentlyContinue

    $out = ""
    if (Test-Path $outLog) { $out = Get-Content $outLog -Raw }
    $err = ""
    if (Test-Path $errLog) { $err = Get-Content $errLog -Raw }
    $blob = "$out`n$err"

    $shotOk = (Test-Path $shot) -and ((Get-Item $shot).Length -ge 2048)
    $panicked = $blob -match "panicked"
    $storyOk = $blob -match "STORY_LOAD ok"

    # The detached launcher case can miss the process handle entirely; there the
    # PNG is the verdict and the exit code is reported as n/a.
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
        Pass      = ($codeOk -and $shotOk -and -not $panicked)
        Log       = $outLog
        Shot      = $shot
    }
}

$results = @()

if ($Case -contains "hero") {
    # Control: VOXELFORGE_HERO takes main.rs's `if cfg.hero` branch, which builds a
    # far smaller App. It proves the packed folder can launch the exe, resolve its
    # DLL imports and reach the GPU with no repo in sight - so a failure in the
    # `play` cases below is about the game code, not about the pack.
    Write-Host "=== CASE hero        (control: exe + GPU, no repo) ==="
    Reset-Room
    $results += Invoke-Case "hero" $Room (Join-Path $Room "voxelforge.exe")
}

if ($Case -contains "play") {
    Write-Host "=== CASE play        (pack intact, CWD = pack) ==="
    Reset-Room
    $results += Invoke-Case "play" $Room (Join-Path $Room "voxelforge.exe")
}

if ($Case -contains "nostory") {
    Write-Host "=== CASE nostory     (assets\story removed) ==="
    Reset-Room
    Remove-Item (Join-Path $Room "assets\story") -Recurse -Force
    $results += Invoke-Case "nostory" $Room (Join-Path $Room "voxelforge.exe")
}

if ($Case -contains "wrongcwd") {
    Write-Host "=== CASE wrongcwd    (pack intact, CWD = C:\) ==="
    Reset-Room
    $results += Invoke-Case "wrongcwd" "C:\" (Join-Path $Room "voxelforge.exe")
}

if ($Case -contains "launcher") {
    Write-Host "=== CASE launcher    (run-voxelforge.cmd, CWD = C:\) ==="
    Reset-Room
    $results += Invoke-Case "launcher" "C:\" (Join-Path $Room "run-voxelforge.cmd") -Detached
}

Write-Host ""
$results | Format-Table Case, Pass, ExitCode, Seconds, ShotOk, ShotBytes, StoryOk, Panicked -AutoSize
Write-Host "CLEANROOM  logs + proof PNGs in $Evidence"
