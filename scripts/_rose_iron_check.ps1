# Iron-rule proof: confirm Rose runs NO cargo/rustc this round (Poppy owns the
# build lane). The patterns live inside this file, not in the launcher's command
# line, so the powershell running it never self-matches.
$ErrorActionPreference = 'SilentlyContinue'
$cargo = Get-CimInstance Win32_Process -Filter "Name='cargo.exe'" |
    Where-Object { $_.CommandLine -match 'voxelforge_perf|target-rose' }
$rustc = Get-CimInstance Win32_Process -Filter "Name='rustc.exe'" |
    Where-Object { $_.CommandLine -match 'voxelforge_perf|target-rose' }
if ($cargo) {
    Write-Output "VIOLATION: cargo of Rose running:"
    $cargo | ForEach-Object { Write-Output ("  pid=" + $_.ProcessId + " " + $_.CommandLine) }
} elseif ($rustc) {
    Write-Output "VIOLATION: rustc of Rose running:"
    $rustc | ForEach-Object { Write-Output ("  pid=" + $_.ProcessId) }
} else {
    Write-Output "CLEAN: no cargo/rustc of Rose (voxelforge_perf/target-rose) running. Iron rule upheld."
}
