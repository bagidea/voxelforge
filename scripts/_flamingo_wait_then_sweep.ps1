# Flamingo — wait for OUR release build to finish, then run the G6 r1 sweep.
#
# Why a detached waiter instead of polling from the session: `cargo build --release`
# here is a fat-LTO single-codegen-unit link (Cargo.toml budgets ~20 min for it) and
# two earlier attempts died when the session was torn down mid-link. This script is
# started with Start-Process so it outlives the session.
#
# Takes the build's PID as $args[0]. That is the whole point: an earlier version
# gated on `(Get-Process cargo,rustc).Count -eq 0`, which is a MACHINE-WIDE count.
# Other lanes build this same repo concurrently (there is a second `cargo build
# --release --bin voxelforge --target-dir ...` running right now), so that gate
# could never go to zero and the waiter would have spun until MAX_MIN and written
# "GAVE UP" without ever shooting a frame. Watch our own process, nobody else's.
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$BUILD_PID = [int]$args[0]
$EXE       = "$root\target-flamingo\release\voxelforge.exe"
$BUILDLOG  = "$root\_flamingo_g6\build3.err.log"
$PROG      = "$root\_flamingo_g6\waiter.log"
$PLAN      = "_flamingo_g6\plan-r1.txt"
$MAX_MIN   = 120

function Note($m) { "[$(Get-Date -Format 'HH:mm:ss')] $m" | Out-File -Encoding utf8 -Append $PROG }

Note "waiter v2 up; watching BUILD PID $BUILD_PID (not a global cargo count)"

$deadline = (Get-Date).AddMinutes($MAX_MIN)
while ((Get-Date) -lt $deadline) {
  $alive = $null -ne (Get-Process -Id $BUILD_PID -ErrorAction SilentlyContinue)
  if (-not $alive) { Note "build pid $BUILD_PID has exited"; break }
  $sz = if (Test-Path $EXE) { (Get-Item $EXE).Length } else { 0 }
  Note "building... pid $BUILD_PID alive, exe=${sz}B"
  Start-Sleep -Seconds 10
}

if ($null -ne (Get-Process -Id $BUILD_PID -ErrorAction SilentlyContinue)) {
  Note "GAVE UP: build still running after $MAX_MIN min"
  exit 1
}

# cargo is gone. Did it produce a binary, or did it fail? The log is the tie-breaker
# — a compile error leaves `error[E...]`/`error:` lines and no exe.
if (-not (Test-Path $EXE)) {
  $errs = @(Select-String -Path $BUILDLOG -Pattern '^error' -ErrorAction SilentlyContinue)
  Note "BUILD FAILED: no exe. '^error' lines in build log = $($errs.Count)"
  foreach ($e in $errs | Select-Object -First 10) { Note "  $($e.Line)" }
  exit 1
}

# The linker writes into the file for a while; cargo exiting means it is done, but
# hold for one quiet beat so a shot never launches against a half-flushed image.
$sz1 = (Get-Item $EXE).Length
Start-Sleep -Seconds 5
$sz2 = (Get-Item $EXE).Length
Note "LINK DONE size=$sz2 (was $sz1 5s ago) mtime=$((Get-Item $EXE).LastWriteTime)"

Note "starting sweep over $PLAN"
$env:FL_EXE = $EXE
& "$root\scripts\_flamingo_g6_sweep.ps1" $PLAN *>> "$root\_flamingo_g6\sweep.run.log"
Note "SWEEP EXIT code=$LASTEXITCODE"
