$ErrorActionPreference = "Continue"
$root  = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
$cargo = "C:\Users\BagIdea\.cargo\bin\cargo.exe"
$status = Join-Path $root "_sun_build2.status"
$out    = Join-Path $root "_sun_build2.log"
$err    = Join-Path $root "_sun_build2.err"
Set-Location $root

function Mark($msg) { Add-Content -Path $status -Value $msg -Encoding ascii }

Mark "BUILD_WAIT_START"

# --- build-lock gate: never stack a 3rd cargo build (STATUS_DLL_INIT_FAILED).
# Wait until other-lane builds (kevin + proof) drop to <= 1, i.e. one slot frees.
$deadline = (Get-Date).AddMinutes(40)
$slotFree = $false
while ((Get-Date) -lt $deadline) {
  $procs = Get-CimInstance Win32_Process -Filter "name='cargo.exe' OR name='rustc.exe'"
  $kevin = @($procs | Where-Object { $_.CommandLine -like '*target-kevin*' }).Count
  $proof = @($procs | Where-Object { $_.CommandLine -like '*target-proof*' }).Count
  if (($kevin + $proof) -le 1) { $slotFree = $true; break }
  Mark ("wait kevin=$kevin proof=$proof " + (Get-Date -Format "HH:mm:ss"))
  Start-Sleep -Seconds 45
}
if (-not $slotFree) { Mark "BUILD_SLOT_TIMEOUT"; exit 3 }

Mark ("BUILD_START " + (Get-Date -Format "HH:mm:ss"))

$p = Start-Process -FilePath $cargo `
  -ArgumentList @("build","--profile","perf","--target-dir","target-sun","--bin","voxelforge","-j","2") `
  -WorkingDirectory $root `
  -RedirectStandardOutput $out `
  -RedirectStandardError $err `
  -WindowStyle Hidden -PassThru -Wait
$ec = $p.ExitCode
Mark "BUILD_EXIT=$ec"
exit $ec
