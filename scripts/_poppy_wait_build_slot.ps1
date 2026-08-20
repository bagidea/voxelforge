# Wait until the machine has a free cargo slot before starting another build.
# Two concurrent cargo builds is the proven ceiling here; a third one has bricked
# runs with STATUS_DLL_INIT_FAILED. One "cargo build" shows up as 2 cargo.exe
# processes (the .cargo shim + the toolchain binary), so 2 builds = 4 processes.
# We start only when at most one OTHER build is running.
param(
  [int]$MaxOtherBuilds = 1,
  [int]$PollSeconds    = 30,
  [int]$MaxWaitMinutes = 120
)
$ErrorActionPreference = "Continue"
$deadline = (Get-Date).AddMinutes($MaxWaitMinutes)
while ((Get-Date) -lt $deadline) {
  $procs = @(Get-CimInstance Win32_Process -Filter "Name='cargo.exe'" |
             Where-Object { $_.CommandLine -like "*build*" })
  $builds = [Math]::Ceiling($procs.Count / 2.0)
  if ($builds -le $MaxOtherBuilds) {
    Write-Output ("slot free: {0} cargo build(s) running" -f $builds)
    exit 0
  }
  Start-Sleep -Seconds $PollSeconds
}
Write-Output "timeout waiting for a free build slot"
exit 1
