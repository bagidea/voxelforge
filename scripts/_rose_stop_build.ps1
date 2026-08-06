# Stop Rose's detached probe build chain (cmd->bash->cargo/rustc).
# Iron rule this round: NO cargo — Poppy owns the build lane; look-perf paused.
# Filtered by process Name so it never matches the powershell running this file
# (its command line is just "powershell -File ...ps1", no pattern inside).
$ErrorActionPreference = 'SilentlyContinue'
$targets = Get-CimInstance Win32_Process | Where-Object {
  ($_.Name -eq 'cmd.exe'   -and $_.CommandLine -match '_rose_probe_build') -or
  ($_.Name -eq 'cargo.exe' -and $_.CommandLine -match 'voxelforge_perf') -or
  ($_.Name -eq 'bash.exe'  -and $_.CommandLine -match '_rose_probe_build\.sh') -or
  ($_.Name -eq 'rustc.exe' -and $_.CommandLine -match 'voxelforge_perf')
}
if ($targets) {
  foreach ($t in $targets) {
    Write-Output ("killing pid=" + $t.ProcessId + " " + $t.Name)
    & taskkill.exe /F /T /PID $t.ProcessId 2>$null | Out-Null
  }
} else {
  Write-Output "no detached build chain running (already finished or gone)"
}
Start-Sleep -Seconds 2
Write-Output "=== verify (any row below = still running) ==="
Get-CimInstance Win32_Process | Where-Object {
  ($_.Name -ne 'powershell.exe') -and
  ($_.CommandLine -match 'voxelforge_perf|_rose_probe_build|target-rose')
} | Select-Object ProcessId, Name | Format-Table -Auto | Out-String
Write-Output "=== stop done ==="
