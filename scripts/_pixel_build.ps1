# _pixel_build.ps1 -- release build for the lighting lane, detached and verifiable.
#
# BUILD RULES THIS FILE EXISTS TO ENFORCE (the lane has been burned by all three):
#   * NEVER judge a build from the tail of a log. A pipe swallows the exit code and
#     a warning-only tail reads "green" on a build that failed. The real exit code
#     is written to <log>.exitcode and is the only thing the caller reads.
#   * `^error` must be 0 in the log, checked separately from the exit code.
#   * The exe must be NEWER and non-trivial in size -- a failed link leaves the
#     previous exe sitting there looking perfectly fine.
#   * 0xC0000142 (STATUS_DLL_INIT_FAILED) is the box running out of headroom, not a
#     code fault. -Jobs 2 is the retry.
#
# Detached via Start-Process on purpose: a build started as a tracked background job
# dies with the session and orphans the link. ASCII only.
param(
    [string]$TargetDir = "target-flamingo",
    [string]$Log       = "_pixel_build.log",
    [int]$Jobs         = 0            # 0 = cargo default; use 2 after a 0xC0000142
)
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
# NOTE: named $logPath, not $log -- PowerShell variables are case-INSENSITIVE, so
# `$log` and the `$Log` parameter are the SAME variable and this line would eat it.
$logPath = Join-Path $Root $Log
$exe  = Join-Path $Root "$TargetDir\release\voxelforge.exe"

# Record what "before" was, so "the exe is new" is a comparison and not a vibe.
$before = if (Test-Path $exe) {
    $i = Get-Item $exe
    @{ mtime = $i.LastWriteTimeUtc.ToString("o"); size = $i.Length }
} else { @{ mtime = "(absent)"; size = 0 } }
$before | ConvertTo-Json -Compress | Set-Content -Encoding utf8 (Join-Path $Root "$Log.before.json")

Remove-Item "$logPath","$logPath.exitcode","$logPath.done" -ErrorAction SilentlyContinue

$jobArg = if ($Jobs -gt 0) { " -j $Jobs" } else { "" }
# cmd.exe wrapper so the exit code of cargo itself lands in the file, not the exit
# code of a PowerShell pipeline that reformatted it.
$inner = "cargo build --release -p voxelforge --bin voxelforge$jobArg > `"$logPath`" 2>&1 & echo %ERRORLEVEL% > `"$logPath.exitcode`" & echo done > `"$logPath.done`""
$env:CARGO_TARGET_DIR = Join-Path $Root $TargetDir
$p = Start-Process -FilePath "cmd.exe" -ArgumentList "/c", $inner -WorkingDirectory $Root -PassThru -WindowStyle Hidden
Write-Output "BUILD_LAUNCHED pid=$($p.Id) target=$TargetDir jobs=$(if($Jobs -gt 0){$Jobs}else{'default'})"
Write-Output ("BEFORE exe mtime={0} size={1}" -f $before.mtime, $before.size)
