# Flamingo - build HEAD (+ the streaming B0002 fix), then shoot the look audit in ONE
# detached chain, so a session teardown cannot orphan the run.
# (2026-08-03 lesson: the previous build was killed mid-link and "still linking" got
#  reported instead of a real result.)
#
# Every native process is run through Start-Process -Redirect*, NOT `*>>`: PowerShell 5.1
# wraps a native command's stderr in ErrorRecord objects and writes UTF-16, which turns
# the build log into unmatchable garbage. Redirect files are raw bytes.
# Verdict is decided by matching '^error' in the build log - never by tail / exit code.
$ErrorActionPreference = 'Continue'
Set-Location (Split-Path -Parent $PSScriptRoot)
$OUT  = '_flamingo_look_audit'
$BOUT = "$OUT\rebuild.out.log"
$BERR = "$OUT\rebuild.err.log"
$VERD = "$OUT\verdict.txt"
$EXE  = (Resolve-Path 'target\release').Path + '\voxelforge.exe'
New-Item -ItemType Directory -Force $OUT | Out-Null

$start = Get-Date
$p = Start-Process -FilePath 'cargo' -ArgumentList 'build','--release','-p','voxelforge' `
     -NoNewWindow -Wait -PassThru -RedirectStandardOutput $BOUT -RedirectStandardError $BERR
$rc = $p.ExitCode
$mins = [math]::Round(((Get-Date) - $start).TotalMinutes, 1)

# cargo writes diagnostics to stderr; 'error[E0308]: ...' / 'error: ...' anchored at col 0.
$errs = @(Select-String -Path $BERR -Pattern '^error' -ErrorAction SilentlyContinue)
if ($errs.Count -gt 0) {
  "BUILD FAILED after ${mins}min - compiler errors:" | Out-File -Encoding utf8 $VERD
  $errs | ForEach-Object { '  ' + $_.Line } | Out-File -Encoding utf8 -Append $VERD
  "=== CHAIN DONE (build failed) ===" | Out-File -Encoding utf8 -Append $VERD
  exit 1
}
if ($rc -ne 0) {
  "BUILD FAILED rc=$rc after ${mins}min (no '^error' line - link/toolchain failure)" | Out-File -Encoding utf8 $VERD
  "=== CHAIN DONE (build failed) ===" | Out-File -Encoding utf8 -Append $VERD
  exit 1
}
$fi = Get-Item $EXE
"BUILD OK in ${mins}min - exe $($fi.Length)b mtime $($fi.LastWriteTime.ToString('o'))" | Out-File -Encoding utf8 $VERD

function Shoot {
  param([string]$Name, [hashtable]$Vars = @{})
  $png = (Resolve-Path $OUT).Path + "\$Name.png"
  Remove-Item $png -ErrorAction SilentlyContinue
  $saved = @{}
  foreach ($k in $Vars.Keys) { $saved[$k] = [Environment]::GetEnvironmentVariable($k); [Environment]::SetEnvironmentVariable($k, $Vars[$k]) }
  [Environment]::SetEnvironmentVariable('VOXELFORGE_PLAY', '1')
  [Environment]::SetEnvironmentVariable('VOXELFORGE_SHOT', $png)
  $gp = Start-Process -FilePath $EXE -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput "$OUT\$Name.out.log" -RedirectStandardError "$OUT\$Name.err.log"
  $code = $gp.ExitCode
  [Environment]::SetEnvironmentVariable('VOXELFORGE_PLAY', $null)
  [Environment]::SetEnvironmentVariable('VOXELFORGE_SHOT', $null)
  foreach ($k in $Vars.Keys) { [Environment]::SetEnvironmentVariable($k, $saved[$k]) }
  if (Test-Path $png) {
    "OK   $Name.png ($((Get-Item $png).Length)b) exit=$code" | Out-File -Encoding utf8 -Append $VERD
  } else {
    "MISS $Name.png exit=$code" | Out-File -Encoding utf8 -Append $VERD
    @(Select-String -Path "$OUT\$Name.err.log","$OUT\$Name.out.log" -Pattern 'panicked|error\[B' -ErrorAction SilentlyContinue) |
      Select-Object -First 2 | ForEach-Object { '     ' + $_.Line } | Out-File -Encoding utf8 -Append $VERD
  }
}

Shoot 'look-on-boot'
Shoot 'look-off-boot'    @{ VOXELFORGE_LOOK_DISABLE = '1' }
Shoot 'look-on-vista'    @{ VOXELFORGE_LOOK_CAM = '35,-18,26' }
Shoot 'look-off-vista'   @{ VOXELFORGE_LOOK_DISABLE = '1'; VOXELFORGE_LOOK_CAM = '35,-18,26' }
Shoot 'look-ultra-vista' @{ VOXELFORGE_LOOK_QUALITY = 'ultra'; VOXELFORGE_LOOK_CAM = '35,-18,26' }

"=== CHAIN DONE $(Get-Date -Format o) ===" | Out-File -Encoding utf8 -Append $VERD
