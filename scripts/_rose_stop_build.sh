#!/usr/bin/env bash
# Stop Rose's detached probe build to comply with the Director's iron rule:
# NO cargo this round — Poppy owns the build lane exclusively; look-perf is paused.
# Tree-kills the cmd->bash->cargo chain launched by the rose_probe_build scheduled
# task, then deletes the task so it can't relaunch. Filtered by process Name so it
# never matches this script's own command line.
set -u
cd "E:/Projects/bagidea-ai-agents-office/workspace/projects/Voxelforge" || exit 1

echo "=== 1. delete scheduled task ==="
schtasks //delete //tn "rose_probe_build" //f 2>/dev/null && echo "deleted" || echo "(already gone)"

echo "=== 2. tree-kill detached build chain (cmd root -> bash -> cargo/rustc) ==="
powershell -NoProfile -Command "
$targets = Get-CimInstance Win32_Process | Where-Object {
  ($_.Name -eq 'cmd.exe'   -and $_.CommandLine -match '_rose_probe_build') -or
  ($_.Name -eq 'cargo.exe' -and $_.CommandLine -match 'voxelforge_perf') -or
  ($_.Name -eq 'bash.exe'  -and $_.CommandLine -match '_rose_probe_build\.sh') -or
  ($_.Name -eq 'rustc.exe' -and $_.CommandLine -match 'voxelforge_perf')
}
if ($targets) {
  $targets | ForEach-Object {
    Write-Output ('killing pid=' + $_.ProcessId + ' ' + $_.Name)
    & taskkill.exe /F /T /PID $_.ProcessId 2>$null | Out-Null
  }
} else { Write-Output 'no detached build chain running' }
"

sleep 2
echo "=== 3. verify nothing of Rose's remains (powershell row below = this check, ignore) ==="
powershell -NoProfile -Command "Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -match 'voxelforge_perf|_rose_probe_build' } | Select-Object ProcessId,Name | Format-Table -Auto | Out-String"
echo "=== stop done ==="
