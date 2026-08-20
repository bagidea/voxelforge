# ===========================================================================
# Flamingo - queue the hue-gap build BEHIND whatever cargo is already running.
#
# WHY A WAITER AND NOT A PLAIN BUILD. A third concurrent cargo on this box has
# produced STATUS_DLL_INIT_FAILED before, and the build already in flight was
# started at 20:22 - BEFORE the look.rs edits this lane just made - so its exe
# cannot be trusted to carry them. This waits the running one out, then runs a
# fresh build so the crate is recompiled against the edited source, and prints
# the exe identity (size + mtime + md5) so the relink is provable rather than
# assumed.
#
# ASCII ONLY: PS 5.1 mojibakes a UTF-8 em-dash into a string delimiter.
# ===========================================================================
param(
  [string]$TargetDir = "target-pixel",
  [int]$MaxWaitMin   = 60
)
$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root

$stamp = Join-Path $root "_flamingo_huegap\build.log"
New-Item -ItemType Directory -Force -Path (Split-Path $stamp) | Out-Null
function Say([string]$m) {
  $line = "[{0}] {1}" -f (Get-Date -Format "HH:mm:ss"), $m
  Add-Content -Path $stamp -Value $line -Encoding UTF8
}

$exe = Join-Path $root "$TargetDir\release\voxelforge.exe"
Say "waiter start; target=$TargetDir exe=$exe"

# ---- phase 1: wait out the in-flight cargo --------------------------------
$deadline = (Get-Date).AddMinutes($MaxWaitMin)
while ((Get-Date) -lt $deadline) {
  $live = @(Get-Process cargo -ErrorAction SilentlyContinue)
  if ($live.Count -eq 0) { break }
  Say ("phase1 waiting: {0} cargo alive" -f $live.Count)
  Start-Sleep -Seconds 45
}
$live = @(Get-Process cargo -ErrorAction SilentlyContinue)
if ($live.Count -gt 0) { Say "phase1 TIMEOUT, cargo still alive, refusing to stack"; exit 2 }
Say "phase1 clear"

if (Test-Path $exe) {
  $b = Get-Item $exe
  Say ("pre-build exe {0} bytes mtime {1} md5 {2}" -f $b.Length, $b.LastWriteTime, (Get-FileHash $exe -Algorithm MD5).Hash)
} else {
  Say "pre-build exe ABSENT"
}

# ---- phase 2: our own build, against the edited source --------------------
Say "phase2 cargo build starting"
$out = Join-Path $root "_flamingo_huegap\cargo.log"
& cargo build --release -p voxelforge --bin voxelforge -j 2 --target-dir $TargetDir 2>&1 |
  Out-File -FilePath $out -Encoding UTF8
$code = $LASTEXITCODE
Say "phase2 cargo exit $code"

if (Test-Path $exe) {
  $b = Get-Item $exe
  Say ("post-build exe {0} bytes mtime {1} md5 {2}" -f $b.Length, $b.LastWriteTime, (Get-FileHash $exe -Algorithm MD5).Hash)
} else {
  Say "post-build exe ABSENT - build failed"
}
Say "waiter done"
