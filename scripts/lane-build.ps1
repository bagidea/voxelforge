# lane-build.ps1 -- ONE build tool for every lane, so 6 parallel lanes stop
# stepping on each other. Takes a lane name, builds into target-<lane>/, and
# prints a verdict that CANNOT be faked by a pipe or a stale exe.
#
# The three ways a build has lied "green" on this box, and how this kills each:
#   (a) A running exe holds target\...\voxelforge.exe open, so the relink dies
#       with "Access is denied (os error 5)" / exit 101. The OLD exe is still
#       sitting there looking fine.  -> pre-link lock check + real-exit gate +
#       mtime gate (three layers, on purpose: defense in depth).
#   (b) `cargo build | tail` returns tail's exit code (0), so a failed build
#       prints green.  -> cargo output is REDIRECTED to a log, never piped; the
#       error verdict is `grep -c '^error'` on that full log, nothing else.
#   (c) 0xC0000142 (STATUS_DLL_INIT_FAILED) when the box runs out of process/
#       commit headroom from too many parallel links.  -> -j 2 and
#       CARGO_PROFILE_DEV_DEBUG=0 keep the lane small; we wait for other cargo
#       builds to drain before starting.
#
# Why the redirect goes through cmd.exe (this is the whole point of (b)):
#   In PowerShell 5.1, `& cargo ... 2>&1 | ...` (or `*> file`) wraps every native
#   stderr line in an ErrorRecord ("cargo.exe : error: ..." + CategoryInfo noise)
#   so `^error` matches NOTHING and a real failure looks clean. cmd.exe's
#   `> log 2>&1` is a raw byte-level redirect, so the log is cargo's true text.
#   Start-Process -Wait -PassThru then gives us cargo's REAL exit code directly
#   (no `%ERRORLEVEL%` parse-time trap, no `$?` after a pipe).
#
# Safe by construction:
#   * Hard deny-list: refuses to build into target-pixel / target-rose /
#     target-poppy / target-monanisa / target-yamamoto (lanes building now).
#   * No `cargo clean` anywhere in this file, ever.
#   * Builds only into target-<lane>/ via CARGO_TARGET_DIR.
#
# Usage:
#   .\scripts\lane-build.ps1 -Lane kevin                 # dev build, -j 2
#   .\scripts\lane-build.ps1 -Lane kevin -Profile release
#   .\scripts\lane-build.ps1 -Lane kevin -Profile perf   # reuses a warm perf cache
#   .\scripts\lane-build.ps1 -Lane kevin -Bin voxelforge_vfx_proof
#   .\scripts\lane-build.ps1 -SelfTest                   # prove (a) and (b) are caught
#
# ASCII only. Windows PowerShell 5.1 safe (no pwsh-only syntax).

param(
    [Parameter(Position = 0)]
    [string]$Lane,
    [string]$Bin = "voxelforge",
    [string]$Profile = "dev",          # dev | release | perf | any named profile
    [int]$Jobs = 2,
    [switch]$NoWait,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

# ---------------------------------------------------------------------------
# Self-test: prove the tool catches (a) and (b), using the SAME verdict code
# the real build uses. Returns a process exit code (0 = both caught).
# ---------------------------------------------------------------------------
if ($SelfTest) {
    $fail = 0
    function Say($m) { Write-Output ("SELFTEST: " + $m) }

    $tmp = Join-Path $env:TEMP ("lane-build-selftest-" + $PID)
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null

    # --- (b) pipe swallows the exit code -> false green --------------------
    # A fake "cargo" that writes two errors to stderr and exits 101.
    $producer = Join-Path $tmp "fake_cargo_fail.ps1"
    @'
$ErrorActionPreference = "Continue"
[Console]::Error.WriteLine("error[E0999]: simulated link failure")
[Console]::Error.WriteLine("  = note: this is the error a pipe would hide")
exit 101
'@ | Set-Content -Encoding ascii $producer

    Say "=== (b) naive pattern `cargo build | tail` returns 0 (false green) ==="
    $bash = Get-Command bash -ErrorAction SilentlyContinue
    if ($bash) {
        # The exact bug: the pipe's exit status is tail's, not cargo's. The fake
        # cargo here is a pure-bash one-liner (two errors to stderr + exit 101),
        # NOT a .ps1, so it runs under WHATEVER bash PowerShell resolves. On this
        # box `bash` is WSL's /Windows/System32/bash.exe, where a Windows
        # powershell.exe path (C:\... or /c/...) is unreachable -- the old
        # `powershell -File` repro died "command not found"/"No such file" there
        # and STILL read CONFIRMED because tail exits 0. The error[E0999] assert
        # below closes that hole for good. `$? is backtick-escaped so PowerShell
        # passes the literal `$?` to bash.
        $naive = & bash -c "(echo 'error[E0999]: simulated link failure' >&2; echo '  = note: this is the error a pipe would hide' >&2; exit 101) 2>&1 | tail -n 2; echo NAIVE_EXIT=`$?"
        Say ($naive -join " | ")
        $naiveText = $naive -join " "
        $sawErr = $naiveText.Contains("error[E0999]")
        $sawExit0 = $naiveText.Contains("NAIVE_EXIT=0")
        if ($sawErr -and $sawExit0) {
            Say "CONFIRMED: naive pipe ran the fake cargo (error[E0999] seen) yet reports exit 0."
        } else {
            if (-not $sawErr) { Say "MISS: fake cargo never printed error[E0999] -- the naive run did not actually run the producer."; $fail++ }
            if (-not $sawExit0) { Say "MISS: naive pipe did not report exit 0 -- check the repro."; $fail++ }
        }
    } else {
        Say "bash not on PATH; skipping the literal-tail repro, doing the redirect comparison instead."
    }

    Say "=== (b) this tool: redirect to log, judge by grep '^error' only ==="
    $bLog = Join-Path $tmp "b.log"
    $inner = "powershell -NoProfile -ExecutionPolicy Bypass -File `"$producer`" > `"$bLog`" 2>&1"
    $p = Start-Process -FilePath cmd.exe -ArgumentList '/c', $inner -Wait -PassThru -WindowStyle Hidden
    $bErr = (Select-String -Path $bLog -Pattern '^error').Count
    $bExit = $p.ExitCode
    Say ("log='" + $bLog + "' errors=" + $bErr + " exit=" + $bExit)
    if ($bErr -gt 0 -and $bExit -ne 0) {
        Say "CONFIRMED: grep sees $bErr error(s) + real exit $bExit -> verdict FAIL (not green)."
    } else {
        Say "MISS: verdict did not catch the failure."; $fail++
    }

    # --- (a) locked exe -> relink dies with access denied ------------------
    Say "=== (a) pre-link lock check sees the exe held (what a running game does) ==="
    $lockedExe = Join-Path $tmp "voxelforge.exe"
    Set-Content -Encoding ascii -Path $lockedExe -Value "placeholder exe"
    # Hold the file with FileShare.None = the same write-denying lock a running
    # image section puts on its own .exe. A second open-for-write must fail.
    $hold = [System.IO.File]::Open($lockedExe, 'Open', 'ReadWrite', 'None')

    # Reuse the exact probe the real pre-link gate runs.
    $locked = $false; $lockMsg = ""
    $running = Get-Process -Name $Bin -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $lockedExe }
    if ($running) { $locked = $true; $lockMsg = "running process pid=" + ($running.Id -join ',') }
    if (-not $locked -and (Test-Path $lockedExe)) {
        try {
            $fs = [System.IO.File]::Open($lockedExe, 'Open', 'ReadWrite', 'None'); $fs.Close()
        } catch { $locked = $true; $lockMsg = "exclusive open denied: " + $_.Exception.Message }
    }
    $hold.Close()
    Say ("locked=" + $locked + " detail='" + $lockMsg + "'")
    if ($locked) { Say "CONFIRMED: lock detected BEFORE any relink attempt -> abort, no access-denied burn." }
    else { Say "MISS: lock not detected."; $fail++ }

    Say "=== (a) a relink-failure log is still judged FAIL by grep ==="
    $aLog = Join-Path $tmp "a.log"
    @'
error: linking with `link.exe` failed: exit code: 1104
  = note: LINK : fatal error LNK1104: cannot open file 'voxelforge.exe'
  = note: Access is denied. (os error 5)
'@ | Set-Content -Encoding ascii $aLog
    $aErr = (Select-String -Path $aLog -Pattern '^error').Count
    Say ("log='" + $aLog + "' errors=" + $aErr)
    if ($aErr -gt 0) { Say "CONFIRMED: grep sees the access-denied relink as $aErr error(s) -> verdict FAIL." }
    else { Say "MISS: relink failure not seen."; $fail++ }

    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue

    if ($fail -eq 0) { Say "SELFTEST PASS: both (a) and (b) false-greens are caught."; exit 0 }
    else { Say "SELFTEST FAIL: $fail assertion(s) missed."; exit 1 }
}

# ---------------------------------------------------------------------------
# Real build
# ---------------------------------------------------------------------------
if (-not $Lane) { Write-Output "usage: lane-build.ps1 -Lane <name> [-Bin voxelforge] [-Profile dev|release|perf] [-Jobs 2]"; exit 2 }

# --- hard deny-list: these target dirs belong to lanes building RIGHT NOW ---
$Forbidden = @("pixel", "rose", "poppy", "monanisa", "yamamoto")
if ($Forbidden -contains $Lane.ToLower()) {
    Write-Output "REFUSING: lane '$Lane' maps to target-$Lane, which is on the deny-list (a teammate is building it now)."
    exit 2
}
if ($Lane -match '[\\/:*?"<>|]' -or $Lane -eq '.' -or $Lane -eq '..') {
    Write-Output "REFUSING: lane name '$Lane' is not a safe directory name."
    exit 2
}

if ($Profile -eq "dev")     { $profileFlag = "";               $exeSubdir = "debug" }
elseif ($Profile -eq "release") { $profileFlag = " --release"; $exeSubdir = "release" }
else                        { $profileFlag = " --profile $Profile"; $exeSubdir = $Profile }

$TargetDir = "target-$Lane"
$TargetAbs = Join-Path $Root $TargetDir
$Exe = Join-Path $TargetAbs "$exeSubdir\$Bin.exe"
$LogPath = Join-Path $Root ("_${Lane}_build.log")
$StartUtc = (Get-Date).ToUniversalTime()

Write-Output ("LANE_BUILD START lane={0} target={1} bin={2} profile={3} jobs={4}" -f $Lane, $TargetDir, $Bin, $Profile, $Jobs)
Write-Output ("BUILD_START {0}" -f $StartUtc.ToString("o"))

# --- before snapshot, so "the exe is new" is a comparison, not a vibe -------
$before = @{ exists = $false; mtime = "(absent)"; size = 0 }
if (Test-Path $Exe) {
    $i = Get-Item $Exe
    $before = @{ exists = $true; mtime = $i.LastWriteTimeUtc.ToString("o"); size = $i.Length }
}
Write-Output ("EXE_BEFORE exists={0} mtime={1} size={2}" -f $before.exists, $before.mtime, $before.size)

# --- wait for other cargo builds to drain (headroom, not politeness) --------
$waited = 0
if (-not $NoWait) {
    while ($waited -lt 1800) {
        $busy = Get-CimInstance Win32_Process -Filter "name='cargo.exe' or name='rustc.exe' or name='link.exe'" -ErrorAction SilentlyContinue
        if (-not $busy) { break }
        Write-Output ("WAIT other build in progress... {0}s" -f $waited)
        Start-Sleep -Seconds 15
        $waited += 15
    }
    if ($waited -ge 1800) { Write-Output "GAVE UP waiting for other cargo builds after 30 min."; exit 2 }
    if ($waited -gt 0) { Write-Output ("LANE_FREE after {0}s" -f $waited) }
}

# --- pre-link lock check (a): is something already holding the exe? ---------
$locked = $false; $lockMsg = "clear"
$running = Get-Process -Name $Bin -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $Exe }
if ($running) { $locked = $true; $lockMsg = "running process pid=" + ($running.Id -join ',') }
if (-not $locked -and (Test-Path $Exe)) {
    try {
        $fs = [System.IO.File]::Open($Exe, 'Open', 'ReadWrite', 'None'); $fs.Close()
    } catch { $locked = $true; $lockMsg = "exclusive open denied: " + $_.Exception.Message }
}
Write-Output ("LOCK_CHECK locked={0} detail={1}" -f $locked, $lockMsg)
if ($locked) {
    Write-Output ("VERDICT ABORT lane={0} locked={1}" -f $Lane, $lockMsg)
    exit 3
}

# --- env for the child (inherited by cmd.exe -> cargo) ----------------------
$env:CARGO_TARGET_DIR = $TargetAbs
# dev builds shed debuginfo so they stay small on the box (the 0xC0000142 fix);
# release/perf are already stripped or opt-level-3 and don't need this.
if ($Profile -eq "dev") { $env:CARGO_PROFILE_DEV_DEBUG = "0" }

Remove-Item $LogPath -ErrorAction SilentlyContinue
$inner = "cargo build --bin $Bin -j $Jobs$profileFlag > `"$LogPath`" 2>&1"
Write-Output ("BUILD_CMD cargo build --bin {0} -j {1}{2}" -f $Bin, $Jobs, $profileFlag)

$p = Start-Process -FilePath cmd.exe -ArgumentList '/c', $inner -Wait -PassThru -WindowStyle Hidden
$realExit = $p.ExitCode
$EndUtc = (Get-Date).ToUniversalTime()

# --- verdict: error signal comes from grep '^error' ONLY (never tail / $?) ---
$errors = (Select-String -Path $LogPath -Pattern '^error').Count
$warnings = (Select-String -Path $LogPath -Pattern '^warning').Count

$exeExists = Test-Path $Exe
$exeSize = 0; $exeMtime = "(absent)"; $exeNewerThanStart = $false
if ($exeExists) {
    $ei = Get-Item $Exe
    $exeSize = $ei.Length
    $exeMtime = $ei.LastWriteTimeUtc.ToString("o")
    $exeNewerThanStart = ($ei.LastWriteTimeUtc -gt $StartUtc)
}

Write-Output ("BUILD_DONE exit={0} errors={1} warnings={2} duration={3:0}s" -f $realExit, $errors, $warnings, ($EndUtc - $StartUtc).TotalSeconds)
Write-Output ("EXE_AFTER exists={0} size={1} mtime={2} newer_than_start={3}" -f $exeExists, $exeSize, $exeMtime, $exeNewerThanStart)

# --- gates ---
$gateError = ($errors -eq 0)
$gateExit  = ($realExit -eq 0)
$gateMtime = $false
if ($exeExists -and $exeSize -gt 0) {
    if ($before.exists) {
        $beforeDt = [DateTime]::Parse($before.mtime)
        $gateMtime = ($ei.LastWriteTimeUtc -ge $beforeDt)
    } else {
        $gateMtime = $true   # first-ever build: any real exe is fresh enough
    }
}
Write-Output ("GATE errors=0:{0} exit=0:{1} mtime_fresh:{2}" -f $gateError, $gateExit, $gateMtime)

if ($gateError -and $gateExit -and $gateMtime) {
    if (-not $exeNewerThanStart) { Write-Output "NOTE: cargo reported success but the exe was not re-linked this run (no-op / already up to date)." }
    Write-Output ("VERDICT PASS lane={0} exe={1}" -f $Lane, $Exe)
    exit 0
} else {
    $why = @()
    if (-not $gateError) { $why += "errors=$errors" }
    if (-not $gateExit)  { $why += "exit=$realExit" }
    if (-not $gateMtime) { $why += "mtime_stale(exists=$exeExists,size=$exeSize,mtime=$exeMtime)" }
    Write-Output ("VERDICT FAIL lane={0} reason={1}" -f $Lane, ($why -join ';'))
    Write-Output ("LOG {0}" -f $LogPath)
    exit 1
}
